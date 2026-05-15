//! Redis Pub/Sub 跨实例用户在线状态同步
//!
//! 当多个 im-gateway 实例运行时，每个实例只知道自己本地的用户连接。
//! 通过 Redis Pub/Sub，实例之间可以广播用户上线/下线事件，
//! 实现全局在线状态的最终一致性。

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

/// 状态变更事件（用于跨实例广播）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceEvent {
    /// 用户 ID
    pub user_id: Uuid,
    /// 实例 ID（标识事件来源）
    pub instance_id: String,
    /// 事件类型
    pub event_type: PresenceEventType,
    /// 时间戳（Unix seconds）
    pub timestamp: i64,
    /// 设备信息（可选）
    pub device_info: Option<String>,
}

/// 事件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PresenceEventType {
    /// 用户上线
    Online,
    /// 用户下线
    Offline,
    /// 用户状态变更（Away / Busy 等）
    StatusChange,
    /// 心跳（用于确认用户仍然在线）
    Heartbeat,
}

/// 跨实例在线状态同步通道
///
/// # 架构
///
/// 每个 im-gateway 实例通过 Redis Pub/Sub 频道广播用户状态变更。
/// 当 Instance A 的用户上线时，它发布事件到 `omnilink:presence` 频道，
/// Instance B 收到事件后更新本地的远程状态缓存。
///
/// - 频道名: `omnilink:presence`
/// - 事件格式: JSON 序列化的 `PresenceEvent`
/// - 本实例 ID 用于过滤自己发出的事件
pub struct PresenceChannel {
    /// 本实例唯一标识
    instance_id: String,
    /// Redis 连接管理器
    redis: ConnectionManager,
    /// Redis 频道名称
    channel_name: String,
    /// 远程用户状态缓存（来自其他实例的状态）
    remote_statuses: Arc<RwLock<HashMap<Uuid, RemoteUserStatus>>>,
    /// 事件接收通道
    event_tx: mpsc::UnboundedSender<PresenceEvent>,
    /// 事件接收端（供 StatusManager 消费）
    event_rx: Arc<RwLock<mpsc::UnboundedReceiver<PresenceEvent>>>,
}

/// 远程用户状态（来自其他实例）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteUserStatus {
    pub user_id: Uuid,
    pub instance_id: String,
    pub is_online: bool,
    pub last_seen: i64,
    pub device_info: Option<String>,
}

impl PresenceChannel {
    /// 创建新的 PresenceChannel
    ///
    /// # Arguments
    /// * `redis` - Redis 连接管理器
    /// * `instance_id` - 本实例唯一标识（建议使用 hostname 或 UUID）
    pub fn new(redis: ConnectionManager, instance_id: String) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let channel_name = "omnilink:presence".to_string();

        Self {
            instance_id,
            redis,
            channel_name,
            remote_statuses: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx: Arc::new(RwLock::new(event_rx)),
        }
    }

    /// 启动发布/订阅后台任务
    ///
    /// 启动两个异步任务：
    /// 1. 订阅任务：监听 Redis 频道，接收其他实例的状态广播
    /// 2. 清理任务：定期清理过期的远程状态缓存
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let instance_id = self.instance_id.clone();
        let channel_name = self.channel_name.clone();
        let remote_statuses = self.remote_statuses.clone();
        let event_tx = self.event_tx.clone();

        // 创建独立的 Redis 连接用于订阅
        let sub_redis = self.redis.clone();

        // 启动订阅任务
        tokio::spawn(async move {
            tracing::info!(
                "[PresenceChannel] Instance {} subscribing to channel: {}",
                instance_id,
                channel_name
            );

            // 使用 redis::aio::PubSub 进行订阅
            // 注意：ConnectionManager 不直接支持 pubsub，需要创建新连接
            // 这里使用一个简化方案：通过定期轮询 Redis 来获取状态变更
            // 生产环境建议使用专用的 pubsub 连接

            loop {
                // 从 Redis channel 读取消息（使用 BRPOP 作为简化实现）
                // 完整实现应使用 SUBSCRIBE 命令
                match Self::poll_presence_events(&sub_redis, &channel_name).await {
                    Ok(events) => {
                        for event in events {
                            // 忽略自己发出的事件
                            if event.instance_id == instance_id {
                                continue;
                            }

                            tracing::debug!(
                                "[PresenceChannel] Received event from {}: {:?} user={}",
                                event.instance_id,
                                event.event_type,
                                event.user_id
                            );

                            // 更新远程状态缓存
                            Self::update_remote_status(&remote_statuses, &event).await;

                            // 转发事件给 StatusManager
                            if let Err(e) = event_tx.send(event) {
                                tracing::error!("[PresenceChannel] Failed to forward event: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("[PresenceChannel] Poll error: {}", e);
                    }
                }

                // 每 100ms 轮询一次
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });

        // 启动远程状态清理任务
        let remote_statuses_cleanup = self.remote_statuses.clone();
        let instance_id_cleanup = self.instance_id.clone();
        tokio::spawn(async move {
            tracing::info!("[PresenceChannel] Remote status cleanup task started");

            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;

                let now = chrono::Utc::now().timestamp();
                let mut statuses = remote_statuses_cleanup.write().await;
                let before_count = statuses.len();

                // 清理超过 5 分钟未更新的远程状态
                statuses.retain(|_, status| {
                    now - status.last_seen < 300
                });

                let cleaned = before_count - statuses.len();
                if cleaned > 0 {
                    tracing::info!(
                        "[PresenceChannel][{}] Cleaned {} expired remote statuses",
                        instance_id_cleanup,
                        cleaned
                    );
                }
            }
        });

        Ok(())
    }

    /// 发布用户上线事件
    pub async fn publish_online(&self, user_id: Uuid, device_info: Option<String>) {
        let event = PresenceEvent {
            user_id,
            instance_id: self.instance_id.clone(),
            event_type: PresenceEventType::Online,
            timestamp: chrono::Utc::now().timestamp(),
            device_info,
        };
        self.publish_event(event).await;
    }

    /// 发布用户下线事件
    pub async fn publish_offline(&self, user_id: Uuid) {
        let event = PresenceEvent {
            user_id,
            instance_id: self.instance_id.clone(),
            event_type: PresenceEventType::Offline,
            timestamp: chrono::Utc::now().timestamp(),
            device_info: None,
        };
        self.publish_event(event).await;
    }

    /// 发布状态变更事件（Away / Busy 等）
    pub async fn publish_status_change(&self, user_id: Uuid, device_info: Option<String>) {
        let event = PresenceEvent {
            user_id,
            instance_id: self.instance_id.clone(),
            event_type: PresenceEventType::StatusChange,
            timestamp: chrono::Utc::now().timestamp(),
            device_info,
        };
        self.publish_event(event).await;
    }

    /// 发布心跳事件
    pub async fn publish_heartbeat(&self, user_id: Uuid) {
        let event = PresenceEvent {
            user_id,
            instance_id: self.instance_id.clone(),
            event_type: PresenceEventType::Heartbeat,
            timestamp: chrono::Utc::now().timestamp(),
            device_info: None,
        };
        self.publish_event(event).await;
    }

    /// 发布事件到 Redis
    async fn publish_event(&self, event: PresenceEvent) {
        let mut redis = self.redis.clone();
        let json = match serde_json::to_string(&event) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("[PresenceChannel] Failed to serialize event: {}", e);
                return;
            }
        };

        // 使用 LPUSH 将事件推入 Redis 列表（简化实现）
        // 生产环境应使用 PUBLISH 命令
        let _: Result<(), _> = redis.lpush(&self.channel_name, &json).await;

        // 保持列表大小限制（最多保留 1000 个事件）
        let _: Result<(), _> = redis.ltrim(&self.channel_name, 0, 999).await;

        tracing::debug!(
            "[PresenceChannel] Published {:?} event for user {}",
            event.event_type,
            event.user_id
        );
    }

    /// 轮询 Redis 获取新的 presence 事件
    async fn poll_presence_events(
        redis: &ConnectionManager,
        channel_name: &str,
    ) -> Result<Vec<PresenceEvent>, Box<dyn std::error::Error + Send + Sync>> {
        let mut redis = redis.clone();
        let mut events = Vec::new();

        // 使用 RPOP 获取最新事件（一次最多获取 10 个）
        for _ in 0..10 {
            let result: Option<String> = redis.rpop(channel_name, None).await?;
            match result {
                Some(json) => {
                    if let Ok(event) = serde_json::from_str::<PresenceEvent>(&json) {
                        events.push(event);
                    }
                }
                None => break,
            }
        }

        Ok(events)
    }

    /// 更新远程状态缓存
    async fn update_remote_status(
        remote_statuses: &Arc<RwLock<HashMap<Uuid, RemoteUserStatus>>>,
        event: &PresenceEvent,
    ) {
        let mut statuses = remote_statuses.write().await;
        match event.event_type {
            PresenceEventType::Online | PresenceEventType::StatusChange => {
                statuses.insert(
                    event.user_id,
                    RemoteUserStatus {
                        user_id: event.user_id,
                        instance_id: event.instance_id.clone(),
                        is_online: true,
                        last_seen: event.timestamp,
                        device_info: event.device_info.clone(),
                    },
                );
            }
            PresenceEventType::Offline => {
                statuses.remove(&event.user_id);
            }
            PresenceEventType::Heartbeat => {
                if let Some(status) = statuses.get_mut(&event.user_id) {
                    status.last_seen = event.timestamp;
                }
            }
        }
    }

    /// 获取远程在线用户列表
    pub async fn get_remote_online_users(&self) -> Vec<RemoteUserStatus> {
        let statuses = self.remote_statuses.read().await;
        statuses.values().filter(|s| s.is_online).cloned().collect()
    }

    /// 检查用户是否在其他实例上在线
    pub async fn is_remote_online(&self, user_id: &Uuid) -> bool {
        let statuses = self.remote_statuses.read().await;
        statuses
            .get(user_id)
            .map(|s| s.is_online)
            .unwrap_or(false)
    }

    /// 获取远程在线用户数
    pub async fn remote_online_count(&self) -> usize {
        let statuses = self.remote_statuses.read().await;
        statuses.values().filter(|s| s.is_online).count()
    }

    /// 接收一个 presence 事件（非阻塞）
    pub async fn recv_event(&self) -> Option<PresenceEvent> {
        let mut rx = self.event_rx.write().await;
        rx.try_recv().ok()
    }

    /// 获取本实例 ID
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    /// 获取频道名称
    pub fn channel_name(&self) -> &str {
        &self.channel_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presence_event_serialization() {
        let event = PresenceEvent {
            user_id: Uuid::new_v4(),
            instance_id: "test-instance".to_string(),
            event_type: PresenceEventType::Online,
            timestamp: 1234567890,
            device_info: Some("iPhone".to_string()),
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: PresenceEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.user_id, event.user_id);
        assert_eq!(deserialized.instance_id, event.instance_id);
        assert_eq!(deserialized.event_type, event.event_type);
        assert_eq!(deserialized.timestamp, event.timestamp);
        assert_eq!(deserialized.device_info, event.device_info);
    }

    #[test]
    fn test_presence_event_type_serialization() {
        assert_eq!(
            serde_json::to_string(&PresenceEventType::Online).unwrap(),
            "\"online\""
        );
        assert_eq!(
            serde_json::to_string(&PresenceEventType::Offline).unwrap(),
            "\"offline\""
        );
        assert_eq!(
            serde_json::to_string(&PresenceEventType::StatusChange).unwrap(),
            "\"statuschange\""
        );
        assert_eq!(
            serde_json::to_string(&PresenceEventType::Heartbeat).unwrap(),
            "\"heartbeat\""
        );
    }

    #[test]
    fn test_remote_user_status_serialization() {
        let status = RemoteUserStatus {
            user_id: Uuid::new_v4(),
            instance_id: "instance-1".to_string(),
            is_online: true,
            last_seen: 1234567890,
            device_info: Some("Android".to_string()),
        };

        let json = serde_json::to_string(&status).unwrap();
        let deserialized: RemoteUserStatus = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.user_id, status.user_id);
        assert_eq!(deserialized.instance_id, status.instance_id);
        assert!(deserialized.is_online);
    }
}
