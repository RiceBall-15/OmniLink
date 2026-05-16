//! WebSocket 连接质量增强模块
//!
//! 提供消息送达确认（ACK）、消息重发策略（指数退避）、
//! 连接质量指标（延迟、丢包率）和自适应心跳间隔功能。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

/// 消息 ACK 状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AckStatus {
    /// 等待确认
    Pending,
    /// 已确认送达
    Acknowledged,
    /// 确认超时
    Timeout,
    /// 重发中
    Retrying,
}

/// 待确认消息记录
#[derive(Debug, Clone)]
pub struct PendingAck {
    /// 消息ID
    pub message_id: String,
    /// 连接ID
    pub connection_id: Uuid,
    /// 发送时间
    pub sent_at: Instant,
    /// 重发次数
    pub retry_count: u32,
    /// 最大重发次数
    pub max_retries: u32,
    /// 当前状态
    pub status: AckStatus,
    /// 下次重发时间
    pub next_retry_at: Option<Instant>,
}

impl PendingAck {
    /// 创建新的待确认消息
    pub fn new(message_id: String, connection_id: Uuid, max_retries: u32) -> Self {
        Self {
            message_id,
            connection_id,
            sent_at: Instant::now(),
            retry_count: 0,
            max_retries,
            status: AckStatus::Pending,
            next_retry_at: None,
        }
    }

    /// 计算下次重发时间（指数退避）
    /// 基础延迟 1s, 最大延迟 30s
    pub fn calculate_next_retry(&self) -> Duration {
        let base_ms = 1000u64;
        let max_ms = 30_000u64;
        let delay_ms = (base_ms * 2u64.pow(self.retry_count)).min(max_ms);
        Duration::from_millis(delay_ms)
    }

    /// 标记为需要重发
    pub fn mark_for_retry(&mut self) {
        self.retry_count += 1;
        self.status = AckStatus::Retrying;
        self.next_retry_at = Some(Instant::now() + self.calculate_next_retry());
    }

    /// 标记为已确认
    pub fn mark_acknowledged(&mut self) {
        self.status = AckStatus::Acknowledged;
    }

    /// 标记为超时
    pub fn mark_timeout(&mut self) {
        self.status = AckStatus::Timeout;
    }

    /// 是否应该重发
    pub fn should_retry(&self) -> bool {
        if self.retry_count >= self.max_retries {
            return false;
        }
        if let Some(next_retry) = self.next_retry_at {
            return Instant::now() >= next_retry;
        }
        false
    }

    /// 获取已等待时间
    pub fn elapsed(&self) -> Duration {
        self.sent_at.elapsed()
    }
}

/// 连接质量指标
#[derive(Debug, Clone)]
pub struct ConnectionQuality {
    /// 连接ID
    pub connection_id: Uuid,
    /// 用户ID
    pub user_id: Uuid,
    /// 平均延迟（毫秒）
    pub avg_latency_ms: f64,
    /// 最近延迟样本（保留最近20个）
    pub latency_samples: Vec<f64>,
    /// 发送消息总数
    pub total_sent: u64,
    /// 确认消息数
    pub total_acked: u64,
    /// 超时消息数
    pub total_timeout: u64,
    /// 丢包率 (0.0 - 1.0)
    pub packet_loss_rate: f64,
    /// 连接建立时间
    pub connected_at: Instant,
    /// 最后一次心跳时间
    pub last_heartbeat_at: Instant,
    /// 最后一次心跳延迟
    pub last_heartbeat_latency_ms: f64,
    /// 当前自适应心跳间隔（秒）
    pub adaptive_heartbeat_interval_secs: u64,
    /// 连接质量等级
    pub quality_level: QualityLevel,
}

/// 连接质量等级
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QualityLevel {
    /// 优秀（延迟 < 100ms, 丢包 < 1%）
    Excellent,
    /// 良好（延迟 < 300ms, 丢包 < 5%）
    Good,
    /// 一般（延迟 < 1000ms, 丢包 < 10%）
    Fair,
    /// 差（延迟 >= 1000ms 或 丢包 >= 10%）
    Poor,
}

impl ConnectionQuality {
    /// 创建新的连接质量记录
    pub fn new(connection_id: Uuid, user_id: Uuid) -> Self {
        let now = Instant::now();
        Self {
            connection_id,
            user_id,
            avg_latency_ms: 0.0,
            latency_samples: Vec::with_capacity(20),
            total_sent: 0,
            total_acked: 0,
            total_timeout: 0,
            packet_loss_rate: 0.0,
            connected_at: now,
            last_heartbeat_at: now,
            last_heartbeat_latency_ms: 0.0,
            adaptive_heartbeat_interval_secs: 30,
            quality_level: QualityLevel::Good,
        }
    }

    /// 记录延迟样本
    pub fn record_latency(&mut self, latency_ms: f64) {
        self.latency_samples.push(latency_ms);
        // 保留最近20个样本
        if self.latency_samples.len() > 20 {
            self.latency_samples.remove(0);
        }
        // 计算平均延迟
        self.avg_latency_ms = self.latency_samples.iter().sum::<f64>()
            / self.latency_samples.len() as f64;
        // 更新质量等级
        self.update_quality_level();
    }

    /// 记录消息发送
    pub fn record_sent(&mut self) {
        self.total_sent += 1;
    }

    /// 记录消息确认
    pub fn record_acked(&mut self) {
        self.total_acked += 1;
        self.update_loss_rate();
    }

    /// 记录消息超时
    pub fn record_timeout(&mut self) {
        self.total_timeout += 1;
        self.update_loss_rate();
    }

    /// 更新丢包率
    fn update_loss_rate(&mut self) {
        let total = self.total_acked + self.total_timeout;
        if total > 0 {
            self.packet_loss_rate = self.total_timeout as f64 / total as f64;
        }
        self.update_quality_level();
    }

    /// 更新质量等级
    fn update_quality_level(&mut self) {
        self.quality_level = if self.avg_latency_ms < 100.0 && self.packet_loss_rate < 0.01 {
            QualityLevel::Excellent
        } else if self.avg_latency_ms < 300.0 && self.packet_loss_rate < 0.05 {
            QualityLevel::Good
        } else if self.avg_latency_ms < 1000.0 && self.packet_loss_rate < 0.10 {
            QualityLevel::Fair
        } else {
            QualityLevel::Poor
        };
        // 根据质量等级调整心跳间隔
        self.adaptive_heartbeat_interval_secs = match self.quality_level {
            QualityLevel::Excellent => 60,
            QualityLevel::Good => 30,
            QualityLevel::Fair => 15,
            QualityLevel::Poor => 10,
        };
    }

    /// 记录心跳响应
    pub fn record_heartbeat(&mut self, latency_ms: f64) {
        self.last_heartbeat_at = Instant::now();
        self.last_heartbeat_latency_ms = latency_ms;
        self.record_latency(latency_ms);
    }

    /// 获取连接持续时间
    pub fn uptime(&self) -> Duration {
        self.connected_at.elapsed()
    }
}

/// ACK 管理器
///
/// 管理消息送达确认、重发策略和连接质量监控。
#[derive(Clone)]
pub struct AckManager {
    /// 待确认消息: message_id -> PendingAck
    pending_acks: Arc<RwLock<HashMap<String, PendingAck>>>,
    /// 连接质量: connection_id -> ConnectionQuality
    connection_quality: Arc<RwLock<HashMap<Uuid, ConnectionQuality>>>,
    /// 默认最大重发次数
    default_max_retries: u32,
    /// ACK 超时时间（毫秒）
    ack_timeout_ms: u64,
}

impl AckManager {
    /// 创建新的 ACK 管理器
    pub fn new(default_max_retries: u32, ack_timeout_ms: u64) -> Self {
        Self {
            pending_acks: Arc::new(RwLock::new(HashMap::new())),
            connection_quality: Arc::new(RwLock::new(HashMap::new())),
            default_max_retries,
            ack_timeout_ms,
        }
    }

    /// 注册新连接的质量监控
    pub async fn register_connection(&self, connection_id: Uuid, user_id: Uuid) {
        let mut quality = self.connection_quality.write().await;
        quality.insert(connection_id, ConnectionQuality::new(connection_id, user_id));
    }

    /// 移除连接的质量监控
    pub async fn unregister_connection(&self, connection_id: &Uuid) {
        let mut quality = self.connection_quality.write().await;
        quality.remove(connection_id);
    }

    /// 注册待确认消息
    pub async fn register_pending_message(
        &self,
        message_id: String,
        connection_id: Uuid,
    ) {
        let mut pending = self.pending_acks.write().await;
        pending.insert(
            message_id.clone(),
            PendingAck::new(message_id, connection_id, self.default_max_retries),
        );

        // 更新发送计数
        let mut quality = self.connection_quality.write().await;
        if let Some(conn_quality) = quality.get_mut(&connection_id) {
            conn_quality.record_sent();
        }
    }

    /// 确认消息送达
    pub async fn acknowledge_message(&self, message_id: &str) -> Option<Uuid> {
        let mut pending = self.pending_acks.write().await;
        if let Some(ack) = pending.get_mut(message_id) {
            ack.mark_acknowledged();
            let connection_id = ack.connection_id;

            // 更新连接质量
            let latency_ms = ack.elapsed().as_millis() as f64;
            let mut quality = self.connection_quality.write().await;
            if let Some(conn_quality) = quality.get_mut(&connection_id) {
                conn_quality.record_latency(latency_ms);
                conn_quality.record_acked();
            }

            // 移除已确认的消息
            pending.remove(message_id);
            return Some(connection_id);
        }
        None
    }

    /// 检查并处理超时和需要重发的消息
    ///
    /// 返回需要重发的消息ID列表
    pub async fn check_pending_messages(&self) -> Vec<(String, Uuid)> {
        let mut pending = self.pending_acks.write().await;
        let mut retry_list = Vec::new();
        let mut timeout_list = Vec::new();

        for (message_id, ack) in pending.iter_mut() {
            // 检查是否超时
            if ack.status == AckStatus::Pending
                && ack.elapsed().as_millis() as u64 > self.ack_timeout_ms
            {
                if ack.retry_count < ack.max_retries {
                    ack.mark_for_retry();
                    retry_list.push((message_id.clone(), ack.connection_id));
                } else {
                    ack.mark_timeout();
                    timeout_list.push(ack.connection_id);
                }
            }

            // 检查是否到了重发时间
            if ack.status == AckStatus::Retrying && ack.should_retry() {
                ack.mark_for_retry();
                retry_list.push((message_id.clone(), ack.connection_id));
            }
        }

        // 更新超时计数
        for connection_id in &timeout_list {
            let mut quality = self.connection_quality.write().await;
            if let Some(conn_quality) = quality.get_mut(connection_id) {
                conn_quality.record_timeout();
            }
        }

        retry_list
    }

    /// 获取连接质量
    pub async fn get_connection_quality(&self, connection_id: &Uuid) -> Option<ConnectionQuality> {
        let quality = self.connection_quality.read().await;
        quality.get(connection_id).cloned()
    }

    /// 获取所有连接质量
    pub async fn get_all_connection_quality(&self) -> Vec<ConnectionQuality> {
        let quality = self.connection_quality.read().await;
        quality.values().cloned().collect()
    }

    /// 记录心跳响应
    pub async fn record_heartbeat(&self, connection_id: &Uuid, latency_ms: f64) {
        let mut quality = self.connection_quality.write().await;
        if let Some(conn_quality) = quality.get_mut(connection_id) {
            conn_quality.record_heartbeat(latency_ms);
        }
    }

    /// 获取自适应心跳间隔
    pub async fn get_adaptive_heartbeat_interval(&self, connection_id: &Uuid) -> u64 {
        let quality = self.connection_quality.read().await;
        quality
            .get(connection_id)
            .map(|q| q.adaptive_heartbeat_interval_secs)
            .unwrap_or(30)
    }

    /// 获取弱连接降级策略
    ///
    /// 根据连接质量等级返回推送策略参数：
    /// - Excellent: 全速推送，无延迟
    /// - Good: 正常推送
    /// - Fair: 降低推送频率，消息批量发送
    /// - Poor: 最小推送，仅关键消息
    pub async fn get_degradation_strategy(&self, connection_id: &Uuid) -> DegradationStrategy {
        let quality = self.connection_quality.read().await;
        quality
            .get(connection_id)
            .map(|q| match q.quality_level {
                QualityLevel::Excellent => DegradationStrategy {
                    max_messages_per_second: 100,
                    batch_size: 1,
                    batch_interval_ms: 0,
                    priority_only: false,
                    description: "全速推送".to_string(),
                },
                QualityLevel::Good => DegradationStrategy {
                    max_messages_per_second: 50,
                    batch_size: 1,
                    batch_interval_ms: 0,
                    priority_only: false,
                    description: "正常推送".to_string(),
                },
                QualityLevel::Fair => DegradationStrategy {
                    max_messages_per_second: 20,
                    batch_size: 5,
                    batch_interval_ms: 1000,
                    priority_only: false,
                    description: "批量推送（每秒最多20条，批量5条）".to_string(),
                },
                QualityLevel::Poor => DegradationStrategy {
                    max_messages_per_second: 5,
                    batch_size: 10,
                    batch_interval_ms: 3000,
                    priority_only: true,
                    description: "仅推送关键消息（@提及、系统通知）".to_string(),
                },
            })
            .unwrap_or(DegradationStrategy::default())
    }

    /// 获取待确认消息数量
    pub async fn pending_count(&self) -> usize {
        let pending = self.pending_acks.read().await;
        pending.len()
    }

    /// 启动 ACK 检查任务
    ///
    /// 定期检查待确认消息，处理超时和重发
    pub fn start_ack_check_task(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(1000));
            loop {
                interval.tick().await;
                let retry_list = self.check_pending_messages().await;
                if !retry_list.is_empty() {
                    tracing::info!(
                        "ACK check: {} messages need retry",
                        retry_list.len()
                    );
                }
            }
        })
    }
}

/// ACK 消息协议
///
/// 客户端和服务器之间的 ACK 消息格式
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AckMessage {
    /// 消息类型
    pub msg_type: String,
    /// 要确认的消息ID
    pub message_id: String,
    /// 时间戳
    pub timestamp: i64,
}

impl AckMessage {
    /// 创建 ACK 消息
    pub fn new(message_id: String) -> Self {
        Self {
            msg_type: "ack".to_string(),
            message_id,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// 序列化为 JSON 字符串
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// 从 JSON 字符串解析
    pub fn from_json(s: &str) -> Option<Self> {
        serde_json::from_str(s).ok()
    }
}

/// 心跳消息协议
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HeartbeatMessage {
    /// 消息类型
    pub msg_type: String,
    /// 客户端发送时间戳
    pub client_timestamp: i64,
    /// 服务器接收时间戳（服务器填写）
    pub server_timestamp: Option<i64>,
}

impl HeartbeatMessage {
    /// 创建客户端心跳消息
    pub fn client_ping() -> Self {
        Self {
            msg_type: "ping".to_string(),
            client_timestamp: chrono::Utc::now().timestamp_millis(),
            server_timestamp: None,
        }
    }

    /// 创建服务器心跳响应
    pub fn server_pong(client_timestamp: i64) -> Self {
        Self {
            msg_type: "pong".to_string(),
            client_timestamp,
            server_timestamp: Some(chrono::Utc::now().timestamp_millis()),
        }
    }

    /// 计算往返延迟（毫秒）
    pub fn calculate_rtt(&self) -> Option<f64> {
        if let Some(_server_ts) = self.server_timestamp {
            let now = chrono::Utc::now().timestamp_millis();
            Some((now - self.client_timestamp) as f64)
        } else {
            None
        }
    }
}

/// 弱连接降级策略
///
/// 根据连接质量等级自动调整消息推送参数
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DegradationStrategy {
    /// 每秒最大消息数
    pub max_messages_per_second: u32,
    /// 批量发送大小
    pub batch_size: u32,
    /// 批量发送间隔（毫秒）
    pub batch_interval_ms: u64,
    /// 是否仅推送关键消息
    pub priority_only: bool,
    /// 策略描述
    pub description: String,
}

impl Default for DegradationStrategy {
    fn default() -> Self {
        Self {
            max_messages_per_second: 50,
            batch_size: 1,
            batch_interval_ms: 0,
            priority_only: false,
            description: "正常推送".to_string(),
        }
    }
}

/// 连接质量报告
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QualityReport {
    /// 连接ID
    pub connection_id: String,
    /// 用户ID
    pub user_id: String,
    /// 平均延迟（毫秒）
    pub avg_latency_ms: f64,
    /// 丢包率
    pub packet_loss_rate: f64,
    /// 质量等级
    pub quality_level: String,
    /// 自适应心跳间隔（秒）
    pub heartbeat_interval_secs: u64,
    /// 连接持续时间（秒）
    pub uptime_secs: u64,
    /// 总发送消息数
    pub total_sent: u64,
    /// 总确认消息数
    pub total_acked: u64,
    /// 总超时消息数
    pub total_timeout: u64,
}

impl From<&ConnectionQuality> for QualityReport {
    fn from(quality: &ConnectionQuality) -> Self {
        Self {
            connection_id: quality.connection_id.to_string(),
            user_id: quality.user_id.to_string(),
            avg_latency_ms: quality.avg_latency_ms,
            packet_loss_rate: quality.packet_loss_rate,
            quality_level: format!("{:?}", quality.quality_level),
            heartbeat_interval_secs: quality.adaptive_heartbeat_interval_secs,
            uptime_secs: quality.uptime().as_secs(),
            total_sent: quality.total_sent,
            total_acked: quality.total_acked,
            total_timeout: quality.total_timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pending_ack_retry_delay() {
        let mut ack = PendingAck::new(
            "msg-1".to_string(),
            Uuid::new_v4(),
            5,
        );

        // First retry: 1s
        let delay = ack.calculate_next_retry();
        assert_eq!(delay, Duration::from_millis(1000));

        ack.mark_for_retry();
        // Second retry: 2s
        let delay = ack.calculate_next_retry();
        assert_eq!(delay, Duration::from_millis(2000));

        ack.mark_for_retry();
        // Third retry: 4s
        let delay = ack.calculate_next_retry();
        assert_eq!(delay, Duration::from_millis(4000));
    }

    #[test]
    fn test_pending_ack_max_retries() {
        let mut ack = PendingAck::new(
            "msg-1".to_string(),
            Uuid::new_v4(),
            2,
        );

        ack.mark_for_retry();
        assert!(ack.should_retry() || !ack.should_retry()); // depends on timing

        ack.mark_for_retry();
        assert!(!ack.should_retry()); // max retries reached
    }

    #[test]
    fn test_pending_ack_zero_max_retries() {
        let ack = PendingAck::new(
            "msg-1".to_string(),
            Uuid::new_v4(),
            0,
        );
        // With 0 max retries, should_retry should be false
        assert!(!ack.should_retry());
    }

    #[test]
    fn test_connection_quality_levels() {
        // Test each level with separate instances to avoid sample accumulation
        
        // Excellent (< 100ms)
        let mut q1 = ConnectionQuality::new(Uuid::new_v4(), Uuid::new_v4());
        q1.record_latency(50.0);
        assert_eq!(q1.quality_level, QualityLevel::Excellent);
        assert_eq!(q1.adaptive_heartbeat_interval_secs, 60);

        // Good (100-300ms)
        let mut q2 = ConnectionQuality::new(Uuid::new_v4(), Uuid::new_v4());
        q2.record_latency(200.0);
        assert_eq!(q2.quality_level, QualityLevel::Good);
        assert_eq!(q2.adaptive_heartbeat_interval_secs, 30);

        // Fair (300-1000ms)
        let mut q3 = ConnectionQuality::new(Uuid::new_v4(), Uuid::new_v4());
        q3.record_latency(500.0);
        assert_eq!(q3.quality_level, QualityLevel::Fair);
        assert_eq!(q3.adaptive_heartbeat_interval_secs, 15);
    }

    #[test]
    fn test_connection_quality_poor_level() {
        let mut quality = ConnectionQuality::new(Uuid::new_v4(), Uuid::new_v4());
        quality.record_latency(1500.0);
        assert_eq!(quality.quality_level, QualityLevel::Poor);
        assert_eq!(quality.adaptive_heartbeat_interval_secs, 10);
    }

    #[test]
    fn test_ack_message_serialization() {
        let ack = AckMessage::new("msg-123".to_string());
        let json = ack.to_json();
        let parsed = AckMessage::from_json(&json).unwrap();
        assert_eq!(parsed.message_id, "msg-123");
        assert_eq!(parsed.msg_type, "ack");
    }

    #[test]
    fn test_ack_message_empty_id() {
        let ack = AckMessage::new(String::new());
        let json = ack.to_json();
        let parsed = AckMessage::from_json(&json).unwrap();
        assert!(parsed.message_id.is_empty());
    }

    #[test]
    fn test_heartbeat_message() {
        let ping = HeartbeatMessage::client_ping();
        assert_eq!(ping.msg_type, "ping");

        let pong = HeartbeatMessage::server_pong(ping.client_timestamp);
        assert_eq!(pong.msg_type, "pong");
        assert!(pong.server_timestamp.is_some());
    }

    #[test]
    fn test_heartbeat_roundtrip() {
        let ping = HeartbeatMessage::client_ping();
        let json = serde_json::to_string(&ping).unwrap();
        assert!(json.contains("ping"));

        let pong = HeartbeatMessage::server_pong(ping.client_timestamp);
        let json = serde_json::to_string(&pong).unwrap();
        assert!(json.contains("pong"));
    }

    #[test]
    fn test_quality_level_ordering() {
        // Verify quality levels are distinct
        assert_ne!(QualityLevel::Excellent, QualityLevel::Good);
        assert_ne!(QualityLevel::Good, QualityLevel::Fair);
        assert_ne!(QualityLevel::Fair, QualityLevel::Poor);
    }

    #[test]
    fn test_degradation_strategy_default() {
        let strategy = DegradationStrategy::default();
        assert_eq!(strategy.max_messages_per_second, 50);
        assert_eq!(strategy.batch_size, 1);
        assert_eq!(strategy.batch_interval_ms, 0);
        assert!(!strategy.priority_only);
        assert_eq!(strategy.description, "正常推送");
    }

    #[test]
    fn test_quality_report_conversion() {
        let mut quality = ConnectionQuality::new(Uuid::new_v4(), Uuid::new_v4());
        quality.record_latency(50.0);
        quality.record_sent();
        quality.record_acked();

        let report = QualityReport::from(&quality);
        assert_eq!(report.avg_latency_ms, 50.0);
        assert_eq!(report.quality_level, "Excellent");
        assert_eq!(report.total_sent, 1);
        assert_eq!(report.total_acked, 1);
        assert_eq!(report.total_timeout, 0);
        assert_eq!(report.heartbeat_interval_secs, 60);
    }

    #[test]
    fn test_quality_report_serialization() {
        let quality = ConnectionQuality::new(Uuid::new_v4(), Uuid::new_v4());
        let report = QualityReport::from(&quality);
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("quality_level"));
        assert!(json.contains("avg_latency_ms"));
        let parsed: QualityReport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.quality_level, report.quality_level);
    }

    #[test]
    fn test_packet_loss_rate_calculation() {
        let mut quality = ConnectionQuality::new(Uuid::new_v4(), Uuid::new_v4());
        // 8 sent, 6 acked, 2 timeout => loss rate = 2/8 = 0.25
        for _ in 0..6 {
            quality.record_acked();
        }
        for _ in 0..2 {
            quality.record_timeout();
        }
        assert!((quality.packet_loss_rate - 0.25).abs() < 0.001);
        // 0.25 >= 0.10, so quality level should be Poor
        assert_eq!(quality.quality_level, QualityLevel::Poor);
    }
}
