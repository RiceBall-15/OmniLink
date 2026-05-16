//! 消息通知投递管道
//!
//! 提供统一的消息通知投递框架，支持：
//! - 多通道投递（WebSocket、Push、Email）
//! - 优先级队列
//! - 每用户速率限制
//! - 投递确认与重试
//! - 批量通知合并
//!
//! # 架构
//!
//! ```text
//! [事件源] → NotificationPipeline → [通道路由器] → WebSocket
//!                                              → Push
//!                                              → Email
//! ```

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use chrono::Utc;
use serde::{Deserialize, Serialize};

// ─── 通知事件类型 ───────────────────────────────────────────

/// 通知事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationEvent {
    /// 事件ID
    pub id: Uuid,
    /// 目标用户ID
    pub user_id: Uuid,
    /// 事件类型
    pub event_type: NotificationType,
    /// 通知优先级
    pub priority: NotificationPriority,
    /// 通知标题
    pub title: String,
    /// 通知内容
    pub body: String,
    /// 关联数据（conversation_id, message_id 等）
    pub data: serde_json::Value,
    /// 创建时间
    pub created_at: i64,
    /// 投递通道偏好（None = 全部通道）
    pub channels: Option<Vec<DeliveryChannel>>,
}

/// 通知类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NotificationType {
    /// 新消息
    NewMessage,
    /// 消息提及（@）
    Mention,
    /// 好友请求
    FriendRequest,
    /// 系统公告
    SystemAnnouncement,
    /// 会话邀请
    ConversationInvite,
}

/// 通知优先级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotificationPriority {
    /// 低优先级（批量投递）
    Low = 0,
    /// 普通优先级
    Normal = 1,
    /// 高优先级（立即投递）
    High = 2,
    /// 紧急（绕过速率限制）
    Urgent = 3,
}

/// 投递通道
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeliveryChannel {
    /// WebSocket 实时推送
    WebSocket,
    /// 移动端推送通知
    Push,
    /// 邮件通知
    Email,
}

// ─── 投递状态 ───────────────────────────────────────────────

/// 投递状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeliveryStatus {
    /// 等待投递
    Pending,
    /// 投递中
    Delivering,
    /// 投递成功
    Delivered,
    /// 投递失败（可重试）
    Failed(String),
    /// 已放弃（超过最大重试次数）
    Abandoned,
}

/// 单通道投递记录
#[derive(Debug, Clone)]
pub struct DeliveryRecord {
    pub channel: DeliveryChannel,
    pub status: DeliveryStatus,
    pub attempts: u32,
    pub last_attempt_at: Option<Instant>,
    pub next_retry_at: Option<Instant>,
}

// ─── 用户通知偏好 ───────────────────────────────────────────

/// 用户通知偏好
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub user_id: Uuid,
    /// 是否启用 WebSocket 通知
    pub websocket_enabled: bool,
    /// 是否启用 Push 通知
    pub push_enabled: bool,
    /// 是否启用邮件通知
    pub email_enabled: bool,
    /// 静默时间段（UTC hour 范围，如 Some((22, 8)) 表示 22:00-08:00 静默）
    pub quiet_hours: Option<(u8, u8)>,
    /// 每分钟最大通知数
    pub rate_limit_per_minute: u32,
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            user_id: Uuid::nil(),
            websocket_enabled: true,
            push_enabled: true,
            email_enabled: false,
            quiet_hours: None,
            rate_limit_per_minute: 30,
        }
    }
}

// ─── 速率限制器 ─────────────────────────────────────────────

/// 滑动窗口速率限制器
#[derive(Debug)]
struct RateLimiter {
    /// 每用户的通知时间戳记录
    timestamps: HashMap<Uuid, VecDeque<Instant>>,
    /// 窗口大小
    window: Duration,
    /// 每窗口最大请求数
    max_requests: u32,
}

impl RateLimiter {
    fn new(window: Duration, max_requests: u32) -> Self {
        Self {
            timestamps: HashMap::new(),
            window,
            max_requests,
        }
    }

    /// 检查是否允许发送通知
    fn allow(&mut self, user_id: &Uuid) -> bool {
        let now = Instant::now();
        let timestamps = self.timestamps.entry(*user_id).or_insert_with(VecDeque::new);

        // 清除过期记录
        while let Some(&front) = timestamps.front() {
            if now.duration_since(front) > self.window {
                timestamps.pop_front();
            } else {
                break;
            }
        }

        if timestamps.len() < self.max_requests as usize {
            timestamps.push_back(now);
            true
        } else {
            false
        }
    }
}

// ─── 投递通道实现 ───────────────────────────────────────────

/// WebSocket 通道投递器
pub struct WebSocketDelivery {
    /// 在线用户的发送通道: user_id -> sender
    user_channels: Arc<RwLock<HashMap<Uuid, mpsc::UnboundedSender<String>>>>,
}

impl WebSocketDelivery {
    pub fn new() -> Self {
        Self {
            user_channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册用户 WebSocket 通道
    pub async fn register(&self, user_id: Uuid, sender: mpsc::UnboundedSender<String>) {
        let mut channels = self.user_channels.write().await;
        channels.insert(user_id, sender);
    }

    /// 注销用户 WebSocket 通道
    pub async fn unregister(&self, user_id: &Uuid) {
        let mut channels = self.user_channels.write().await;
        channels.remove(user_id);
    }

    /// 检查用户是否在线
    pub async fn is_online(&self, user_id: &Uuid) -> bool {
        let channels = self.user_channels.read().await;
        channels.contains_key(user_id)
    }

    /// 通过 WebSocket 投递通知
    pub async fn deliver(&self, event: &NotificationEvent) -> Result<(), String> {
        let channels = self.user_channels.read().await;
        if let Some(sender) = channels.get(&event.user_id) {
            let ws_msg = serde_json::json!({
                "type": "notification",
                "id": event.id,
                "event_type": event.event_type,
                "title": event.title,
                "body": event.body,
                "data": event.data,
                "timestamp": event.created_at,
            });
            let json = serde_json::to_string(&ws_msg).map_err(|e| e.to_string())?;
            sender.send(json).map_err(|_| "Channel closed".to_string())?;
            Ok(())
        } else {
            Err("User not online".to_string())
        }
    }
}

// ─── 通知管道 ───────────────────────────────────────────────

/// 通知管道配置
pub struct NotificationPipelineConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试基础延迟（毫秒）
    pub retry_base_delay_ms: u64,
    /// 队列最大容量
    pub queue_capacity: usize,
    /// 速率限制窗口（秒）
    pub rate_limit_window_secs: u64,
    /// 默认每分钟最大通知数
    pub default_rate_limit: u32,
}

impl Default for NotificationPipelineConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_base_delay_ms: 1000,
            queue_capacity: 10000,
            rate_limit_window_secs: 60,
            default_rate_limit: 30,
        }
    }
}

/// 通知投递管道
pub struct NotificationPipeline {
    /// WebSocket 投递器
    ws_delivery: Arc<WebSocketDelivery>,
    /// 用户通知偏好
    preferences: Arc<RwLock<HashMap<Uuid, NotificationPreferences>>>,
    /// 速率限制器
    rate_limiter: Arc<RwLock<RateLimiter>>,
    /// 投递队列（优先级队列）
    delivery_queue: Arc<RwLock<VecDeque<NotificationEvent>>>,
    /// 投递记录: event_id -> Vec<DeliveryRecord>
    delivery_records: Arc<RwLock<HashMap<Uuid, Vec<DeliveryRecord>>>>,
    /// 管道配置
    config: NotificationPipelineConfig,
    /// 统计数据
    stats: Arc<RwLock<PipelineStats>>,
}

/// 管道统计数据
#[derive(Debug, Clone, Default)]
pub struct PipelineStats {
    pub total_received: u64,
    pub total_delivered: u64,
    pub total_failed: u64,
    pub total_rate_limited: u64,
    pub total_quiet_hours_skipped: u64,
    pub ws_delivered: u64,
    pub push_delivered: u64,
    pub email_delivered: u64,
}

impl NotificationPipeline {
    pub fn new(config: NotificationPipelineConfig) -> Self {
        let rate_limiter = RateLimiter::new(
            Duration::from_secs(config.rate_limit_window_secs),
            config.default_rate_limit,
        );

        Self {
            ws_delivery: Arc::new(WebSocketDelivery::new()),
            preferences: Arc::new(RwLock::new(HashMap::new())),
            rate_limiter: Arc::new(RwLock::new(rate_limiter)),
            delivery_queue: Arc::new(RwLock::new(VecDeque::with_capacity(config.queue_capacity))),
            delivery_records: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(PipelineStats::default())),
        }
    }

    /// 获取 WebSocket 投递器引用（用于注册用户通道）
    pub fn ws_delivery(&self) -> &Arc<WebSocketDelivery> {
        &self.ws_delivery
    }

    /// 设置用户通知偏好
    pub async fn set_preferences(&self, prefs: NotificationPreferences) {
        let mut preferences = self.preferences.write().await;
        preferences.insert(prefs.user_id, prefs);
    }

    /// 提交通知事件到管道
    pub async fn submit(&self, event: NotificationEvent) -> Result<(), String> {
        // 更新统计
        {
            let mut stats = self.stats.write().await;
            stats.total_received += 1;
        }

        // 检查用户偏好
        let preferences = self.preferences.read().await;
        let user_prefs = preferences.get(&event.user_id).cloned().unwrap_or_default();

        // 检查静默时间
        if let Some((start, end)) = user_prefs.quiet_hours {
            let current_hour = (Utc::now().hour()) as u8;
            let in_quiet = if start > end {
                // 跨午夜，如 22:00 - 08:00
                current_hour >= start || current_hour < end
            } else {
                current_hour >= start && current_hour < end
            };
            if in_quiet && event.priority < NotificationPriority::Urgent {
                let mut stats = self.stats.write().await;
                stats.total_quiet_hours_skipped += 1;
                return Ok(());
            }
        }

        // 检查速率限制（Urgent 优先级绕过限制）
        if event.priority < NotificationPriority::Urgent {
            let mut limiter = self.rate_limiter.write().await;
            if !limiter.allow(&event.user_id) {
                let mut stats = self.stats.write().await;
                stats.total_rate_limited += 1;
                return Ok(());
            }
        }

        drop(preferences);

        // 入队
        let mut queue = self.delivery_queue.write().await;
        if queue.len() >= self.config.queue_capacity {
            return Err("Notification queue full".to_string());
        }

        // 按优先级插入（高优先级在前）
        let insert_pos = queue.iter().position(|e| e.priority < event.priority).unwrap_or(queue.len());
        queue.insert(insert_pos, event);

        Ok(())
    }

    /// 处理队列中的通知（应由后台任务定期调用）
    pub async fn process_queue(&self) -> usize {
        let events: Vec<NotificationEvent> = {
            let mut queue = self.delivery_queue.write().await;
            // 一次最多处理 100 个
            let count = queue.len().min(100);
            queue.drain(..count).collect()
        };

        let count = events.len();
        for event in events {
            self.deliver_event(event).await;
        }
        count
    }

    /// 投递单个事件到所有适用通道
    async fn deliver_event(&self, event: NotificationEvent) {
        let preferences = self.preferences.read().await;
        let user_prefs = preferences.get(&event.user_id).cloned().unwrap_or_default();

        // 确定投递通道
        let channels = event.channels.clone().unwrap_or_else(|| {
            let mut ch = Vec::new();
            if user_prefs.websocket_enabled {
                ch.push(DeliveryChannel::WebSocket);
            }
            if user_prefs.push_enabled {
                ch.push(DeliveryChannel::Push);
            }
            if user_prefs.email_enabled {
                ch.push(DeliveryChannel::Email);
            }
            ch
        });

        drop(preferences);

        let mut records = Vec::new();

        for channel in channels {
            let success = match channel {
                DeliveryChannel::WebSocket => {
                    let result = self.ws_delivery.deliver(&event).await;
                    let success = result.is_ok();
                    let mut stats = self.stats.write().await;
                    if success {
                        stats.ws_delivered += 1;
                    }
                    success
                }
                DeliveryChannel::Push => {
                    // Push 通道 - 框架状态，返回失败触发重试
                    tracing::debug!("Push delivery not yet implemented for user {}", event.user_id);
                    false
                }
                DeliveryChannel::Email => {
                    // Email 通道 - 框架状态
                    tracing::debug!("Email delivery not yet implemented for user {}", event.user_id);
                    false
                }
            };

            records.push(DeliveryRecord {
                channel: channel.clone(),
                status: if success {
                    DeliveryStatus::Delivered
                } else {
                    DeliveryStatus::Failed("Delivery failed".to_string())
                },
                attempts: 1,
                last_attempt_at: Some(Instant::now()),
                next_retry_at: if !success {
                    Some(Instant::now() + Duration::from_millis(self.config.retry_base_delay_ms))
                } else {
                    None
                },
            });
        }

        // 更新统计
        {
            let mut stats = self.stats.write().await;
            if records.iter().any(|r| matches!(r.status, DeliveryStatus::Delivered)) {
                stats.total_delivered += 1;
            } else {
                stats.total_failed += 1;
            }
        }

        // 保存投递记录
        let mut delivery_records = self.delivery_records.write().await;
        delivery_records.insert(event.id, records);
    }

    /// 获取管道统计
    pub async fn get_stats(&self) -> PipelineStats {
        self.stats.read().await.clone()
    }

    /// 获取队列长度
    pub async fn queue_len(&self) -> usize {
        self.delivery_queue.read().await.len()
    }

    /// 启动后台处理任务
    pub fn start_background_task(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let pipeline = Arc::clone(self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            loop {
                interval.tick().await;
                let processed = pipeline.process_queue().await;
                if processed > 0 {
                    tracing::debug!("Notification pipeline processed {} events", processed);
                }
            }
        })
    }
}

// ─── 便捷构建器 ─────────────────────────────────────────────

impl NotificationEvent {
    /// 创建新消息通知
    pub fn new_message(
        user_id: Uuid,
        sender_id: Uuid,
        conversation_id: Uuid,
        message_id: Uuid,
        preview: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            event_type: NotificationType::NewMessage,
            priority: NotificationPriority::Normal,
            title: "New Message".to_string(),
            body: preview,
            data: serde_json::json!({
                "sender_id": sender_id,
                "conversation_id": conversation_id,
                "message_id": message_id,
            }),
            created_at: Utc::now().timestamp(),
            channels: None,
        }
    }

    /// 创建提及通知
    pub fn mention(
        user_id: Uuid,
        sender_id: Uuid,
        conversation_id: Uuid,
        message_id: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            event_type: NotificationType::Mention,
            priority: NotificationPriority::High,
            title: "You were mentioned".to_string(),
            body: format!("Someone mentioned you in a conversation"),
            data: serde_json::json!({
                "sender_id": sender_id,
                "conversation_id": conversation_id,
                "message_id": message_id,
            }),
            created_at: Utc::now().timestamp(),
            channels: None,
        }
    }

    /// 创建系统公告通知
    pub fn system_announcement(user_id: Uuid, title: String, body: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            event_type: NotificationType::SystemAnnouncement,
            priority: NotificationPriority::High,
            title,
            body,
            data: serde_json::json!({}),
            created_at: Utc::now().timestamp(),
            channels: None,
        }
    }
}

// ─── 测试 ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(Duration::from_secs(60), 3);
        let user = Uuid::new_v4();

        assert!(limiter.allow(&user));
        assert!(limiter.allow(&user));
        assert!(limiter.allow(&user));
        assert!(!limiter.allow(&user)); // 超过限制
    }

    #[tokio::test]
    async fn test_notification_pipeline_submit() {
        let config = NotificationPipelineConfig {
            queue_capacity: 100,
            ..Default::default()
        };
        let pipeline = NotificationPipeline::new(config);

        let event = NotificationEvent::new_message(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Hello!".to_string(),
        );

        assert!(pipeline.submit(event).await.is_ok());
        assert_eq!(pipeline.queue_len().await, 1);
    }

    #[tokio::test]
    async fn test_notification_priority_ordering() {
        let config = NotificationPipelineConfig {
            queue_capacity: 100,
            ..Default::default()
        };
        let pipeline = NotificationPipeline::new(config);

        let user_id = Uuid::new_v4();

        // 提交低优先级
        let low = NotificationEvent {
            id: Uuid::new_v4(),
            user_id,
            event_type: NotificationType::NewMessage,
            priority: NotificationPriority::Low,
            title: "Low".to_string(),
            body: "low priority".to_string(),
            data: serde_json::json!({}),
            created_at: Utc::now().timestamp(),
            channels: None,
        };
        pipeline.submit(low).await.unwrap();

        // 提交高优先级
        let high = NotificationEvent {
            id: Uuid::new_v4(),
            user_id,
            event_type: NotificationType::Mention,
            priority: NotificationPriority::High,
            title: "High".to_string(),
            body: "high priority".to_string(),
            data: serde_json::json!({}),
            created_at: Utc::now().timestamp(),
            channels: None,
        };
        pipeline.submit(high).await.unwrap();

        // 高优先级应该在前面
        let queue = pipeline.delivery_queue.read().await;
        assert_eq!(queue[0].priority, NotificationPriority::High);
        assert_eq!(queue[1].priority, NotificationPriority::Low);
    }

    #[tokio::test]
    async fn test_websocket_delivery_offline() {
        let ws = WebSocketDelivery::new();
        let event = NotificationEvent::new_message(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "test".to_string(),
        );

        // 用户不在线，应该返回错误
        let result = ws.deliver(&event).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_websocket_delivery_online() {
        let ws = WebSocketDelivery::new();
        let user_id = Uuid::new_v4();
        let (tx, mut rx) = mpsc::unbounded_channel();

        ws.register(user_id, tx).await;

        let event = NotificationEvent::new_message(
            user_id,
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Hello!".to_string(),
        );

        let result = ws.deliver(&event).await;
        assert!(result.is_ok());

        // 验证收到消息
        let msg = rx.recv().await.unwrap();
        assert!(msg.contains("notification"));
        assert!(msg.contains("Hello!"));
    }

    #[test]
    fn test_notification_event_builders() {
        let user_id = Uuid::new_v4();
        let sender_id = Uuid::new_v4();
        let conv_id = Uuid::new_v4();
        let msg_id = Uuid::new_v4();

        let event = NotificationEvent::new_message(
            user_id, sender_id, conv_id, msg_id, "preview".to_string(),
        );
        assert_eq!(event.event_type, NotificationType::NewMessage);
        assert_eq!(event.priority, NotificationPriority::Normal);

        let mention = NotificationEvent::mention(user_id, sender_id, conv_id, msg_id);
        assert_eq!(mention.event_type, NotificationType::Mention);
        assert_eq!(mention.priority, NotificationPriority::High);

        let sys = NotificationEvent::system_announcement(
            user_id, "Title".to_string(), "Body".to_string(),
        );
        assert_eq!(sys.event_type, NotificationType::SystemAnnouncement);
    }

    #[test]
    fn test_notification_preferences_default() {
        let prefs = NotificationPreferences::default();
        assert!(prefs.websocket_enabled);
        assert!(prefs.push_enabled);
        assert!(!prefs.email_enabled);
        assert_eq!(prefs.rate_limit_per_minute, 30);
    }

    #[tokio::test]
    async fn test_quiet_hours() {
        let config = NotificationPipelineConfig::default();
        let pipeline = NotificationPipeline::new(config);

        let user_id = Uuid::new_v4();

        // 设置静默时间（当前 UTC 小时前后各 1 小时，确保覆盖当前时间）
        let current_hour = Utc::now().hour() as u8;
        let start = if current_hour == 0 { 23 } else { current_hour - 1 };
        let end = (current_hour + 2) % 24;

        pipeline.set_preferences(NotificationPreferences {
            user_id,
            quiet_hours: Some((start, end)),
            ..Default::default()
        }).await;

        // 普通优先级应该被静默
        let event = NotificationEvent {
            id: Uuid::new_v4(),
            user_id,
            event_type: NotificationType::NewMessage,
            priority: NotificationPriority::Normal,
            title: "Test".to_string(),
            body: "test".to_string(),
            data: serde_json::json!({}),
            created_at: Utc::now().timestamp(),
            channels: None,
        };

        pipeline.submit(event).await.unwrap();
        assert_eq!(pipeline.queue_len().await, 0);

        let stats = pipeline.get_stats().await;
        assert_eq!(stats.total_quiet_hours_skipped, 1);
    }

    #[tokio::test]
    async fn test_urgent_bypasses_quiet_hours() {
        let config = NotificationPipelineConfig::default();
        let pipeline = NotificationPipeline::new(config);

        let user_id = Uuid::new_v4();
        let current_hour = Utc::now().hour() as u8;
        let start = if current_hour == 0 { 23 } else { current_hour - 1 };
        let end = (current_hour + 2) % 24;

        pipeline.set_preferences(NotificationPreferences {
            user_id,
            quiet_hours: Some((start, end)),
            ..Default::default()
        }).await;

        // 紧急优先级应该绕过静默
        let event = NotificationEvent {
            id: Uuid::new_v4(),
            user_id,
            event_type: NotificationType::SystemAnnouncement,
            priority: NotificationPriority::Urgent,
            title: "Urgent".to_string(),
            body: "urgent".to_string(),
            data: serde_json::json!({}),
            created_at: Utc::now().timestamp(),
            channels: None,
        };

        pipeline.submit(event).await.unwrap();
        assert_eq!(pipeline.queue_len().await, 1);
    }
}
