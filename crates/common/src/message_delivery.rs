//! 消息投递可靠性模块
//!
//! 提供消息发送确认、重试策略和死信队列功能，确保消息投递的可靠性。
//!
//! 核心组件：
//! - `PendingMessageQueue`: 待确认消息队列，跟踪消息投递状态
//! - `DeadLetterQueue`: 死信队列，存储多次重试失败的消息
//! - `RetryStrategy`: 指数退避重试策略

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;
use serde::{Serialize, Deserialize};

/// 消息投递状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryStatus {
    /// 待发送
    Pending,
    /// 已发送，等待确认
    Sent,
    /// 已确认送达
    Acknowledged,
    /// 投递失败，将重试
    Failed,
    /// 重试次数耗尽，进入死信队列
    DeadLetter,
}

impl std::fmt::Display for DeliveryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeliveryStatus::Pending => write!(f, "pending"),
            DeliveryStatus::Sent => write!(f, "sent"),
            DeliveryStatus::Acknowledged => write!(f, "acknowledged"),
            DeliveryStatus::Failed => write!(f, "failed"),
            DeliveryStatus::DeadLetter => write!(f, "dead_letter"),
        }
    }
}

/// 待确认消息条目
#[derive(Debug, Clone)]
pub struct PendingMessage {
    /// 消息ID
    pub message_id: Uuid,
    /// 目标用户ID
    pub target_user_id: Uuid,
    /// 会话ID
    pub conversation_id: Uuid,
    /// 消息内容
    pub content: String,
    /// 消息类型
    pub message_type: String,
    /// 当前投递状态
    pub status: DeliveryStatus,
    /// 重试次数
    pub retry_count: u32,
    /// 最大重试次数
    pub max_retries: u32,
    /// 首次创建时间
    pub created_at: Instant,
    /// 最后一次尝试时间
    pub last_attempt_at: Option<Instant>,
    /// 下次重试时间
    pub next_retry_at: Option<Instant>,
    /// 错误信息
    pub last_error: Option<String>,
}

impl PendingMessage {
    /// 创建新的待确认消息
    pub fn new(
        message_id: Uuid,
        target_user_id: Uuid,
        conversation_id: Uuid,
        content: String,
        message_type: String,
        max_retries: u32,
    ) -> Self {
        Self {
            message_id,
            target_user_id,
            conversation_id,
            content,
            message_type,
            status: DeliveryStatus::Pending,
            retry_count: 0,
            max_retries,
            created_at: Instant::now(),
            last_attempt_at: None,
            next_retry_at: None,
            last_error: None,
        }
    }

    /// 标记为已发送
    pub fn mark_sent(&mut self) {
        self.status = DeliveryStatus::Sent;
        self.last_attempt_at = Some(Instant::now());
    }

    /// 标记为已确认
    pub fn mark_acknowledged(&mut self) {
        self.status = DeliveryStatus::Acknowledged;
    }

    /// 标记为失败并计算下次重试时间
    pub fn mark_failed(&mut self, error: String) -> bool {
        self.retry_count += 1;
        self.last_error = Some(error);
        self.last_attempt_at = Some(Instant::now());

        if self.retry_count >= self.max_retries {
            self.status = DeliveryStatus::DeadLetter;
            false // 不再重试
        } else {
            self.status = DeliveryStatus::Failed;
            // 指数退避：base_ms * 2^retry_count，最大30秒
            let base_ms = 1000u64;
            let delay_ms = (base_ms * 2u64.pow(self.retry_count)).min(30_000);
            self.next_retry_at = Some(Instant::now() + Duration::from_millis(delay_ms));
            true // 可以重试
        }
    }

    /// 检查是否应该重试
    pub fn should_retry(&self) -> bool {
        if self.status != DeliveryStatus::Failed {
            return false;
        }
        if let Some(next_retry) = self.next_retry_at {
            Instant::now() >= next_retry
        } else {
            true
        }
    }

    /// 获取当前重试延迟（毫秒）
    pub fn retry_delay_ms(&self) -> u64 {
        let base_ms = 1000u64;
        (base_ms * 2u64.pow(self.retry_count)).min(30_000)
    }
}

/// 重试策略配置
#[derive(Debug, Clone)]
pub struct RetryStrategy {
    /// 最大重试次数
    pub max_retries: u32,
    /// 基础重试延迟（毫秒）
    pub base_delay_ms: u64,
    /// 最大重试延迟（毫秒）
    pub max_delay_ms: u64,
    /// 消息确认超时时间（秒）
    pub ack_timeout_secs: u64,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
            max_delay_ms: 30_000,
            ack_timeout_secs: 30,
        }
    }
}

impl RetryStrategy {
    /// 计算指定重试次数的延迟
    pub fn calculate_delay(&self, retry_count: u32) -> Duration {
        let delay_ms = (self.base_delay_ms * 2u64.pow(retry_count)).min(self.max_delay_ms);
        Duration::from_millis(delay_ms)
    }
}

/// 待确认消息队列
///
/// 跟踪所有已发送但尚未确认的消息，支持重试和超时检测。
#[derive(Clone)]
pub struct PendingMessageQueue {
    /// 待确认消息存储 (message_id -> PendingMessage)
    messages: Arc<RwLock<HashMap<Uuid, PendingMessage>>>,
    /// 重试策略
    strategy: RetryStrategy,
}

impl PendingMessageQueue {
    /// 创建新的待确认消息队列
    pub fn new(strategy: RetryStrategy) -> Self {
        Self {
            messages: Arc::new(RwLock::new(HashMap::new())),
            strategy,
        }
    }

    /// 使用默认策略创建队列
    pub fn with_default_strategy() -> Self {
        Self::new(RetryStrategy::default())
    }

    /// 添加消息到待确认队列
    pub async fn enqueue(
        &self,
        message_id: Uuid,
        target_user_id: Uuid,
        conversation_id: Uuid,
        content: String,
        message_type: String,
    ) -> PendingMessage {
        let msg = PendingMessage::new(
            message_id,
            target_user_id,
            conversation_id,
            content,
            message_type,
            self.strategy.max_retries,
        );

        let mut messages = self.messages.write().await;
        messages.insert(message_id, msg.clone());

        tracing::debug!(
            "Enqueued pending message {} for user {} (max retries: {})",
            message_id,
            target_user_id,
            self.strategy.max_retries
        );

        msg
    }

    /// 标记消息为已发送
    pub async fn mark_sent(&self, message_id: &Uuid) -> bool {
        let mut messages = self.messages.write().await;
        if let Some(msg) = messages.get_mut(message_id) {
            msg.mark_sent();
            tracing::debug!("Marked message {} as sent", message_id);
            true
        } else {
            false
        }
    }

    /// 确认消息送达
    pub async fn acknowledge(&self, message_id: &Uuid) -> Option<PendingMessage> {
        let mut messages = self.messages.write().await;
        if let Some(msg) = messages.get_mut(message_id) {
            msg.mark_acknowledged();
            let result = msg.clone();
            messages.remove(message_id);
            tracing::info!("Message {} acknowledged and removed from queue", message_id);
            Some(result)
        } else {
            None
        }
    }

    /// 标记消息投递失败
    pub async fn mark_failed(&self, message_id: &Uuid, error: String) -> Option<bool> {
        let mut messages = self.messages.write().await;
        if let Some(msg) = messages.get_mut(message_id) {
            let can_retry = msg.mark_failed(error.clone());
            if !can_retry {
                tracing::warn!(
                    "Message {} exceeded max retries ({}), moving to dead letter",
                    message_id,
                    msg.max_retries
                );
            } else {
                tracing::info!(
                    "Message {} failed (attempt {}/{}): {}",
                    message_id,
                    msg.retry_count,
                    msg.max_retries,
                    error
                );
            }
            Some(can_retry)
        } else {
            None
        }
    }

    /// 获取所有需要重试的消息
    pub async fn get_retryable_messages(&self) -> Vec<PendingMessage> {
        let messages = self.messages.read().await;
        messages
            .values()
            .filter(|msg| msg.should_retry())
            .cloned()
            .collect()
    }

    /// 获取所有已进入死信状态的消息并从队列中移除
    pub async fn drain_dead_letters(&self) -> Vec<PendingMessage> {
        let mut messages = self.messages.write().await;
        let dead_letters: Vec<PendingMessage> = messages
            .values()
            .filter(|msg| msg.status == DeliveryStatus::DeadLetter)
            .cloned()
            .collect();

        for msg in &dead_letters {
            messages.remove(&msg.message_id);
        }

        dead_letters
    }

    /// 获取指定消息的状态
    pub async fn get_status(&self, message_id: &Uuid) -> Option<DeliveryStatus> {
        let messages = self.messages.read().await;
        messages.get(message_id).map(|msg| msg.status)
    }

    /// 获取队列中的消息数量
    pub async fn pending_count(&self) -> usize {
        let messages = self.messages.read().await;
        messages.len()
    }

    /// 获取所有待确认消息的摘要
    pub async fn get_summary(&self) -> PendingQueueSummary {
        let messages = self.messages.read().await;
        let total = messages.len();
        let pending = messages.values().filter(|m| m.status == DeliveryStatus::Pending).count();
        let sent = messages.values().filter(|m| m.status == DeliveryStatus::Sent).count();
        let failed = messages.values().filter(|m| m.status == DeliveryStatus::Failed).count();
        let dead_letter = messages.values().filter(|m| m.status == DeliveryStatus::DeadLetter).count();

        PendingQueueSummary {
            total,
            pending,
            sent,
            failed,
            dead_letter,
        }
    }
}

/// 队列摘要统计
#[derive(Debug, Clone, Serialize)]
pub struct PendingQueueSummary {
    pub total: usize,
    pub pending: usize,
    pub sent: usize,
    pub failed: usize,
    pub dead_letter: usize,
}

/// 死信队列条目
#[derive(Debug, Clone)]
pub struct DeadLetterEntry {
    /// 原始消息ID
    pub message_id: Uuid,
    /// 目标用户ID
    pub target_user_id: Uuid,
    /// 会话ID
    pub conversation_id: Uuid,
    /// 消息内容
    pub content: String,
    /// 消息类型
    pub message_type: String,
    /// 总重试次数
    pub total_retries: u32,
    /// 最后一次错误
    pub last_error: String,
    /// 进入死信队列的时间
    pub dead_letter_at: i64,
}

impl DeadLetterEntry {
    /// 从 PendingMessage 创建死信条目
    pub fn from_pending(msg: &PendingMessage) -> Self {
        Self {
            message_id: msg.message_id,
            target_user_id: msg.target_user_id,
            conversation_id: msg.conversation_id,
            content: msg.content.clone(),
            message_type: msg.message_type.clone(),
            total_retries: msg.retry_count,
            last_error: msg.last_error.clone().unwrap_or_default(),
            dead_letter_at: chrono::Utc::now().timestamp(),
        }
    }
}

/// 死信队列
///
/// 存储多次重试失败的消息，支持查询、重试和清理。
#[derive(Clone)]
pub struct DeadLetterQueue {
    /// 死信存储
    entries: Arc<RwLock<Vec<DeadLetterEntry>>>,
    /// 最大容量
    max_capacity: usize,
}

impl DeadLetterQueue {
    /// 创建新的死信队列
    pub fn new(max_capacity: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_capacity,
        }
    }

    /// 使用默认容量（1000）创建
    pub fn with_default_capacity() -> Self {
        Self::new(1000)
    }

    /// 添加消息到死信队列
    pub async fn push(&self, entry: DeadLetterEntry) {
        let mut entries = self.entries.write().await;

        // 如果超过容量，移除最旧的条目
        if entries.len() >= self.max_capacity {
            entries.remove(0);
            tracing::warn!("Dead letter queue full, removed oldest entry");
        }

        tracing::info!(
            "Added message {} to dead letter queue (retries: {}, error: {})",
            entry.message_id,
            entry.total_retries,
            entry.last_error
        );

        entries.push(entry);
    }

    /// 从 PendingMessage 批量添加到死信队列
    pub async fn push_from_pending(&self, messages: &[PendingMessage]) {
        for msg in messages {
            let entry = DeadLetterEntry::from_pending(msg);
            self.push(entry).await;
        }
    }

    /// 获取所有死信条目
    pub async fn get_all(&self) -> Vec<DeadLetterEntry> {
        let entries = self.entries.read().await;
        entries.clone()
    }

    /// 按用户ID过滤死信
    pub async fn get_by_user(&self, user_id: &Uuid) -> Vec<DeadLetterEntry> {
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| e.target_user_id == *user_id)
            .cloned()
            .collect()
    }

    /// 按会话ID过滤死信
    pub async fn get_by_conversation(&self, conversation_id: &Uuid) -> Vec<DeadLetterEntry> {
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| e.conversation_id == *conversation_id)
            .cloned()
            .collect()
    }

    /// 获取死信数量
    pub async fn count(&self) -> usize {
        let entries = self.entries.read().await;
        entries.len()
    }

    /// 清空死信队列
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
        tracing::info!("Dead letter queue cleared");
    }

    /// 移除指定消息的死信条目
    pub async fn remove(&self, message_id: &Uuid) -> Option<DeadLetterEntry> {
        let mut entries = self.entries.write().await;
        if let Some(pos) = entries.iter().position(|e| e.message_id == *message_id) {
            Some(entries.remove(pos))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delivery_status_display() {
        assert_eq!(DeliveryStatus::Pending.to_string(), "pending");
        assert_eq!(DeliveryStatus::Sent.to_string(), "sent");
        assert_eq!(DeliveryStatus::Acknowledged.to_string(), "acknowledged");
        assert_eq!(DeliveryStatus::Failed.to_string(), "failed");
        assert_eq!(DeliveryStatus::DeadLetter.to_string(), "dead_letter");
    }

    #[test]
    fn test_pending_message_new() {
        let msg_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let conv_id = Uuid::new_v4();

        let msg = PendingMessage::new(
            msg_id,
            user_id,
            conv_id,
            "Hello".to_string(),
            "text".to_string(),
            3,
        );

        assert_eq!(msg.message_id, msg_id);
        assert_eq!(msg.target_user_id, user_id);
        assert_eq!(msg.status, DeliveryStatus::Pending);
        assert_eq!(msg.retry_count, 0);
        assert_eq!(msg.max_retries, 3);
    }

    #[test]
    fn test_pending_message_mark_sent() {
        let mut msg = PendingMessage::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Hello".to_string(),
            "text".to_string(),
            3,
        );

        msg.mark_sent();
        assert_eq!(msg.status, DeliveryStatus::Sent);
        assert!(msg.last_attempt_at.is_some());
    }

    #[test]
    fn test_pending_message_mark_acknowledged() {
        let mut msg = PendingMessage::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Hello".to_string(),
            "text".to_string(),
            3,
        );

        msg.mark_sent();
        msg.mark_acknowledged();
        assert_eq!(msg.status, DeliveryStatus::Acknowledged);
    }

    #[test]
    fn test_pending_message_retry_exhaustion() {
        let mut msg = PendingMessage::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Hello".to_string(),
            "text".to_string(),
            2, // max 2 retries
        );

        // First failure
        let can_retry = msg.mark_failed("Error 1".to_string());
        assert!(can_retry);
        assert_eq!(msg.retry_count, 1);
        assert_eq!(msg.status, DeliveryStatus::Failed);

        // Second failure - should exhaust retries
        let can_retry = msg.mark_failed("Error 2".to_string());
        assert!(!can_retry);
        assert_eq!(msg.retry_count, 2);
        assert_eq!(msg.status, DeliveryStatus::DeadLetter);
    }

    #[test]
    fn test_retry_strategy_default() {
        let strategy = RetryStrategy::default();
        assert_eq!(strategy.max_retries, 3);
        assert_eq!(strategy.base_delay_ms, 1000);
        assert_eq!(strategy.max_delay_ms, 30_000);
    }

    #[test]
    fn test_retry_strategy_delay_calculation() {
        let strategy = RetryStrategy::default();

        // 1st retry: 1000 * 2^1 = 2000ms
        let delay = strategy.calculate_delay(1);
        assert_eq!(delay, Duration::from_millis(2000));

        // 2nd retry: 1000 * 2^2 = 4000ms
        let delay = strategy.calculate_delay(2);
        assert_eq!(delay, Duration::from_millis(4000));

        // 3rd retry: 1000 * 2^3 = 8000ms
        let delay = strategy.calculate_delay(3);
        assert_eq!(delay, Duration::from_millis(8000));

        // Large retry count should be capped at max_delay_ms
        let delay = strategy.calculate_delay(20);
        assert_eq!(delay, Duration::from_millis(30_000));
    }

    #[tokio::test]
    async fn test_pending_queue_enqueue_and_acknowledge() {
        let queue = PendingMessageQueue::with_default_strategy();
        let msg_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let conv_id = Uuid::new_v4();

        // Enqueue
        queue.enqueue(
            msg_id,
            user_id,
            conv_id,
            "Hello".to_string(),
            "text".to_string(),
        ).await;

        assert_eq!(queue.pending_count().await, 1);

        // Mark sent
        assert!(queue.mark_sent(&msg_id).await);

        // Acknowledge
        let acked = queue.acknowledge(&msg_id).await;
        assert!(acked.is_some());
        assert_eq!(acked.unwrap().status, DeliveryStatus::Acknowledged);
        assert_eq!(queue.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_pending_queue_get_status() {
        let queue = PendingMessageQueue::with_default_strategy();
        let msg_id = Uuid::new_v4();

        queue.enqueue(
            msg_id,
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Hello".to_string(),
            "text".to_string(),
        ).await;

        let status = queue.get_status(&msg_id).await;
        assert_eq!(status, Some(DeliveryStatus::Pending));

        let unknown = queue.get_status(&Uuid::new_v4()).await;
        assert_eq!(unknown, None);
    }

    #[tokio::test]
    async fn test_pending_queue_summary() {
        let queue = PendingMessageQueue::with_default_strategy();

        for _ in 0..5 {
            queue.enqueue(
                Uuid::new_v4(),
                Uuid::new_v4(),
                Uuid::new_v4(),
                "Hello".to_string(),
                "text".to_string(),
            ).await;
        }

        let summary = queue.get_summary().await;
        assert_eq!(summary.total, 5);
        assert_eq!(summary.pending, 5);
        assert_eq!(summary.sent, 0);
        assert_eq!(summary.failed, 0);
    }

    #[tokio::test]
    async fn test_dead_letter_queue_push_and_get() {
        let dlq = DeadLetterQueue::new(100);

        let entry = DeadLetterEntry {
            message_id: Uuid::new_v4(),
            target_user_id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            content: "Failed message".to_string(),
            message_type: "text".to_string(),
            total_retries: 3,
            last_error: "Connection timeout".to_string(),
            dead_letter_at: 1700000000,
        };

        dlq.push(entry.clone()).await;
        assert_eq!(dlq.count().await, 1);

        let all = dlq.get_all().await;
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].message_id, entry.message_id);
    }

    #[tokio::test]
    async fn test_dead_letter_queue_capacity() {
        let dlq = DeadLetterQueue::new(3);

        for i in 0..5 {
            let entry = DeadLetterEntry {
                message_id: Uuid::new_v4(),
                target_user_id: Uuid::new_v4(),
                conversation_id: Uuid::new_v4(),
                content: format!("Message {}", i),
                message_type: "text".to_string(),
                total_retries: 3,
                last_error: "Error".to_string(),
                dead_letter_at: 1700000000 + i as i64,
            };
            dlq.push(entry).await;
        }

        // Should only keep the last 3
        assert_eq!(dlq.count().await, 3);
    }

    #[tokio::test]
    async fn test_dead_letter_queue_filter_by_user() {
        let dlq = DeadLetterQueue::new(100);
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();

        for i in 0..3 {
            let entry = DeadLetterEntry {
                message_id: Uuid::new_v4(),
                target_user_id: if i < 2 { user1 } else { user2 },
                conversation_id: Uuid::new_v4(),
                content: format!("Message {}", i),
                message_type: "text".to_string(),
                total_retries: 3,
                last_error: "Error".to_string(),
                dead_letter_at: 1700000000,
            };
            dlq.push(entry).await;
        }

        let user1_msgs = dlq.get_by_user(&user1).await;
        assert_eq!(user1_msgs.len(), 2);

        let user2_msgs = dlq.get_by_user(&user2).await;
        assert_eq!(user2_msgs.len(), 1);
    }

    #[tokio::test]
    async fn test_dead_letter_queue_remove() {
        let dlq = DeadLetterQueue::new(100);
        let msg_id = Uuid::new_v4();

        let entry = DeadLetterEntry {
            message_id: msg_id,
            target_user_id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            content: "Test".to_string(),
            message_type: "text".to_string(),
            total_retries: 3,
            last_error: "Error".to_string(),
            dead_letter_at: 1700000000,
        };

        dlq.push(entry).await;
        assert_eq!(dlq.count().await, 1);

        let removed = dlq.remove(&msg_id).await;
        assert!(removed.is_some());
        assert_eq!(dlq.count().await, 0);

        let not_found = dlq.remove(&Uuid::new_v4()).await;
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_dead_letter_queue_clear() {
        let dlq = DeadLetterQueue::new(100);

        for _ in 0..5 {
            let entry = DeadLetterEntry {
                message_id: Uuid::new_v4(),
                target_user_id: Uuid::new_v4(),
                conversation_id: Uuid::new_v4(),
                content: "Test".to_string(),
                message_type: "text".to_string(),
                total_retries: 3,
                last_error: "Error".to_string(),
                dead_letter_at: 1700000000,
            };
            dlq.push(entry).await;
        }

        assert_eq!(dlq.count().await, 5);

        dlq.clear().await;
        assert_eq!(dlq.count().await, 0);
    }

    #[test]
    fn test_dead_letter_entry_from_pending() {
        let mut msg = PendingMessage::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Hello".to_string(),
            "text".to_string(),
            3,
        );

        msg.mark_failed("Error 1".to_string());
        msg.mark_failed("Error 2".to_string());
        msg.mark_failed("Error 3".to_string());

        let entry = DeadLetterEntry::from_pending(&msg);
        assert_eq!(entry.message_id, msg.message_id);
        assert_eq!(entry.total_retries, 3);
        assert_eq!(entry.last_error, "Error 3");
    }
}
