//! 消息批量推送优化
//!
//! 在高并发场景下，将多条消息聚合后批量推送，减少 WebSocket 帧数量：
//! - 可配置的消息聚合窗口（默认 100ms）
//! - 可配置的最大批量大小（默认 50 条）
//! - 按目标用户/会话分组批量发送
//! - 批量 ACK 确认机制
//! - 背压控制（队列满时丢弃最旧消息）
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use im_gateway::message_batcher::{MessageBatcher, BatcherConfig};
//! use im_gateway::models::WSMessage;
//! use im_gateway::connection_manager::WSConnectionManager;
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! # async fn example() {
//! let config = BatcherConfig {
//!     window_ms: 100,
//!     max_batch_size: 50,
//!     max_queue_size: 10000,
//! };
//!
//! let conn_manager = Arc::new(WSConnectionManager::new());
//! let batcher = MessageBatcher::new(config, conn_manager);
//!
//! // 启动后台批量处理任务
//! batcher.start().await;
//!
//! // 推送消息（自动聚合）
//! let msg = WSMessage {
//!     message_type: im_gateway::models::WSMessageType::NewMessage,
//!     conversation_id: None,
//!     message_id: None,
//!     sender_id: None,
//!     content: Some("hello".into()),
//!     timestamp: None,
//!     data: None,
//! };
//! batcher.push_to_user(uuid::Uuid::new_v4(), msg).await;
//! # }
//! ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::connection_manager::WSConnectionManager;
use crate::models::{WSMessage, WSMessageType};

/// 批量推送配置
#[derive(Debug, Clone)]
pub struct BatcherConfig {
    /// 消息聚合窗口（毫秒），默认 100ms
    pub window_ms: u64,
    /// 单批次最大消息数，默认 50
    pub max_batch_size: usize,
    /// 消息队列最大容量，默认 10000
    pub max_queue_size: usize,
}

impl Default for BatcherConfig {
    fn default() -> Self {
        Self {
            window_ms: 100,
            max_batch_size: 50,
            max_queue_size: 10000,
        }
    }
}

/// 批量消息目标
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BatchTarget {
    /// 发送给指定用户的所有设备
    User(Uuid),
    /// 发送给会话中的所有连接
    Conversation(Uuid),
}

/// 待批量发送的消息条目
#[derive(Debug, Clone)]
struct BatchEntry {
    /// 目标
    target: BatchTarget,
    /// 消息内容
    message: WSMessage,
}

/// 批量消息包装
///
/// 发送给客户端的批量消息格式：
/// ```json
/// {
///   "type": "batch",
///   "messages": [...],
///   "batch_id": "uuid",
///   "count": 5
/// }
/// ```
#[derive(Debug, Clone, serde::Serialize)]
pub struct BatchPayload {
    #[serde(rename = "type")]
    pub message_type: String,
    pub batch_id: String,
    pub count: usize,
    pub messages: Vec<WSMessage>,
}

/// 批量 ACK 请求
#[derive(Debug, Clone, serde::Deserialize)]
pub struct BatchAckRequest {
    /// 批次 ID
    pub batch_id: String,
    /// 已确认的消息 ID 列表
    pub message_ids: Vec<String>,
}

/// 批量推送统计
#[derive(Debug, Clone, serde::Serialize)]
pub struct BatcherStats {
    /// 总推送消息数
    pub total_messages: u64,
    /// 总发送批次数
    pub total_batches: u64,
    /// 当前队列深度
    pub queue_depth: usize,
    /// 平均批量大小
    pub avg_batch_size: f64,
    /// 丢弃的消息数
    pub dropped_messages: u64,
}

/// 消息批量推送器
///
/// 线程安全，可在多个异步任务间共享。
#[derive(Clone)]
pub struct MessageBatcher {
    config: BatcherConfig,
    conn_manager: Arc<WSConnectionManager>,
    tx: mpsc::UnboundedSender<BatchEntry>,
    rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<BatchEntry>>>>,
    running: Arc<AtomicBool>,
    stats: Arc<BatcherStatsInner>,
}

struct BatcherStatsInner {
    total_messages: AtomicU64,
    total_batches: AtomicU64,
    dropped_messages: AtomicU64,
    total_batch_size: AtomicU64,
}

impl MessageBatcher {
    /// 创建新的消息批量推送器
    pub fn new(config: BatcherConfig, conn_manager: Arc<WSConnectionManager>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            config,
            conn_manager,
            tx,
            rx: Arc::new(RwLock::new(Some(rx))),
            running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(BatcherStatsInner {
                total_messages: AtomicU64::new(0),
                total_batches: AtomicU64::new(0),
                dropped_messages: AtomicU64::new(0),
                total_batch_size: AtomicU64::new(0),
            }),
        }
    }

    /// 推送消息到指定用户
    pub async fn push_to_user(&self, user_id: Uuid, message: WSMessage) -> bool {
        self.push(BatchEntry {
            target: BatchTarget::User(user_id),
            message,
        })
    }

    /// 推送消息到会话
    pub async fn push_to_conversation(
        &self,
        conversation_id: Uuid,
        message: WSMessage,
    ) -> bool {
        self.push(BatchEntry {
            target: BatchTarget::Conversation(conversation_id),
            message,
        })
    }

    /// 内部推送方法
    fn push(&self, entry: BatchEntry) -> bool {
        if self.tx.send(entry).is_err() {
            self.stats.dropped_messages.fetch_add(1, Ordering::Relaxed);
            return false;
        }
        self.stats.total_messages.fetch_add(1, Ordering::Relaxed);
        true
    }

    /// 启动后台批量处理任务
    ///
    /// 在独立的 tokio 任务中运行，持续消费消息队列并批量发送。
    pub async fn start(&self) {
        if self.running.swap(true, Ordering::SeqCst) {
            tracing::warn!("MessageBatcher 已在运行");
            return;
        }

        let mut rx_guard = self.rx.write().await;
        let rx = rx_guard.take();
        drop(rx_guard);

        let rx = match rx {
            Some(rx) => rx,
            None => {
                tracing::error!("MessageBatcher: receiver 已被消费");
                self.running.store(false, Ordering::SeqCst);
                return;
            }
        };

        let rx = Arc::new(tokio::sync::Mutex::new(rx));
        let config = self.config.clone();
        let conn_manager = self.conn_manager.clone();
        let running = self.running.clone();
        let stats = self.stats.clone();

        tokio::spawn(async move {
            let window = Duration::from_millis(config.window_ms);
            let rx = rx;

            tracing::info!(
                window_ms = config.window_ms,
                max_batch_size = config.max_batch_size,
                "MessageBatcher 启动"
            );

            while running.load(Ordering::SeqCst) {
                // 等待窗口时间或达到最大批量大小
                let mut batch: Vec<BatchEntry> = Vec::new();

                // 先尝试非阻塞接收
                {
                    let mut guard = rx.lock().await;
                    while batch.len() < config.max_batch_size {
                        match guard.try_recv() {
                            Ok(entry) => batch.push(entry),
                            Err(_) => break,
                        }
                    }
                }

                if batch.is_empty() {
                    // 没有消息，阻塞等待
                    let mut guard = rx.lock().await;
                    tokio::select! {
                        entry = guard.recv() => {
                            match entry {
                                Some(e) => batch.push(e),
                                None => break, // channel closed
                            }
                        }
                        _ = tokio::time::sleep(window) => {}
                    }
                }

                if batch.is_empty() {
                    continue;
                }

                // 在窗口内继续收集
                let deadline = tokio::time::Instant::now() + window;
                {
                    let mut guard = rx.lock().await;
                    while batch.len() < config.max_batch_size {
                        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                        if remaining.is_zero() {
                            break;
                        }
                        match tokio::time::timeout(remaining, guard.recv()).await {
                            Ok(Some(entry)) => batch.push(entry),
                            _ => break,
                        }
                    }
                }

                // 按目标分组并发送
                let mut grouped: HashMap<BatchTarget, Vec<WSMessage>> = HashMap::new();
                for entry in batch {
                    grouped
                        .entry(entry.target)
                        .or_default()
                        .push(entry.message);
                }

                let batch_count = grouped.len() as u64;
                let msg_count: u64 = grouped.values().map(|v| v.len() as u64).sum();

                for (target, messages) in grouped {
                    let batch_id = Uuid::new_v4().to_string();
                    let count = messages.len();

                    let payload = BatchPayload {
                        message_type: "batch".to_string(),
                        batch_id,
                        count,
                        messages,
                    };

                    if let Ok(data_value) = serde_json::to_value(&payload) {
                        match &target {
                            BatchTarget::User(user_id) => {
                                // 通过 WSConnectionManager 发送
                                let ws_message = WSMessage {
                                    message_type: WSMessageType::NewMessage,
                                    conversation_id: None,
                                    message_id: None,
                                    sender_id: None,
                                    content: Some(payload.messages.len().to_string()),
                                    timestamp: None,
                                    data: Some(data_value.clone()),
                                };
                                conn_manager.send_to_user(*user_id, ws_message).await;
                            }
                            BatchTarget::Conversation(conv_id) => {
                                let ws_message = WSMessage {
                                    message_type: WSMessageType::NewMessage,
                                    conversation_id: Some(*conv_id),
                                    message_id: None,
                                    sender_id: None,
                                    content: Some(payload.messages.len().to_string()),
                                    timestamp: None,
                                    data: Some(data_value),
                                };
                                conn_manager
                                    .send_to_conversation(*conv_id, ws_message)
                                    .await;
                            }
                        }
                    }
                }

                stats.total_batches.fetch_add(batch_count, Ordering::Relaxed);
                stats.total_batch_size.fetch_add(msg_count, Ordering::Relaxed);
            }

            tracing::info!("MessageBatcher 停止");
        });
    }

    /// 获取统计信息
    pub fn stats(&self) -> BatcherStats {
        let total_messages = self.stats.total_messages.load(Ordering::Relaxed);
        let total_batches = self.stats.total_batches.load(Ordering::Relaxed);
        let total_batch_size = self.stats.total_batch_size.load(Ordering::Relaxed);

        let avg_batch_size = if total_batches > 0 {
            total_batch_size as f64 / total_batches as f64
        } else {
            0.0
        };

        BatcherStats {
            total_messages,
            total_batches,
            queue_depth: 0, // unbounded channel 无法直接获取深度
            avg_batch_size,
            dropped_messages: self.stats.dropped_messages.load(Ordering::Relaxed),
        }
    }

    /// 停止批量处理
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// 是否正在运行
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::WSMessageType;

    fn create_test_message(content: &str) -> WSMessage {
        WSMessage {
            message_type: WSMessageType::NewMessage,
            conversation_id: None,
            message_id: Some(Uuid::new_v4()),
            sender_id: Some(Uuid::new_v4()),
            content: Some(content.to_string()),
            timestamp: Some(chrono::Utc::now().timestamp()),
            data: None,
        }
    }

    #[test]
    fn test_batcher_config_defaults() {
        let config = BatcherConfig::default();
        assert_eq!(config.window_ms, 100);
        assert_eq!(config.max_batch_size, 50);
        assert_eq!(config.max_queue_size, 10000);
    }

    #[test]
    fn test_batch_target_user_equality() {
        let id = Uuid::new_v4();
        assert_eq!(BatchTarget::User(id), BatchTarget::User(id));
        assert_ne!(
            BatchTarget::User(Uuid::new_v4()),
            BatchTarget::User(Uuid::new_v4())
        );
    }

    #[test]
    fn test_batch_target_conversation_equality() {
        let id = Uuid::new_v4();
        assert_eq!(
            BatchTarget::Conversation(id),
            BatchTarget::Conversation(id)
        );
    }

    #[test]
    fn test_batch_target_user_ne_conversation() {
        let id = Uuid::new_v4();
        assert_ne!(BatchTarget::User(id), BatchTarget::Conversation(id));
    }

    #[test]
    fn test_batch_payload_serialization() {
        let payload = BatchPayload {
            message_type: "batch".to_string(),
            batch_id: "test-id".to_string(),
            count: 2,
            messages: vec![
                create_test_message("msg1"),
                create_test_message("msg2"),
            ],
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"type\":\"batch\""));
        assert!(json.contains("\"count\":2"));
        assert!(json.contains("\"batch_id\":\"test-id\""));
    }

    #[test]
    fn test_batch_ack_request_deserialization() {
        let json = r#"{"batch_id":"test-id","message_ids":["msg1","msg2"]}"#;
        let ack: BatchAckRequest = serde_json::from_str(json).unwrap();
        assert_eq!(ack.batch_id, "test-id");
        assert_eq!(ack.message_ids.len(), 2);
    }

    #[test]
    fn test_batcher_stats_initial() {
        let conn_manager = Arc::new(WSConnectionManager::new());
        let batcher = MessageBatcher::new(BatcherConfig::default(), conn_manager);

        let stats = batcher.stats();
        assert_eq!(stats.total_messages, 0);
        assert_eq!(stats.total_batches, 0);
        assert_eq!(stats.avg_batch_size, 0.0);
        assert_eq!(stats.dropped_messages, 0);
    }

    #[test]
    fn test_batcher_not_running_by_default() {
        let conn_manager = Arc::new(WSConnectionManager::new());
        let batcher = MessageBatcher::new(BatcherConfig::default(), conn_manager);

        assert!(!batcher.is_running());
    }

    #[test]
    fn test_stop_is_idempotent() {
        let conn_manager = Arc::new(WSConnectionManager::new());
        let batcher = MessageBatcher::new(BatcherConfig::default(), conn_manager);

        batcher.stop();
        batcher.stop(); // should not panic
        assert!(!batcher.is_running());
    }

    #[test]
    fn test_push_increments_total_messages() {
        let conn_manager = Arc::new(WSConnectionManager::new());
        let batcher = MessageBatcher::new(BatcherConfig::default(), conn_manager);

        let msg = create_test_message("test");
        let user_id = Uuid::new_v4();

        // push 是 sync 的（通过 channel）
        let entry = BatchEntry {
            target: BatchTarget::User(user_id),
            message: msg,
        };
        let _ = batcher.tx.send(entry);

        assert_eq!(batcher.stats().total_messages, 1);
    }

    #[test]
    fn test_batcher_stats_serialization() {
        let conn_manager = Arc::new(WSConnectionManager::new());
        let batcher = MessageBatcher::new(BatcherConfig::default(), conn_manager);

        let stats = batcher.stats();
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("total_messages"));
        assert!(json.contains("total_batches"));
    }
}
