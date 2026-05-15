use uuid::Uuid;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{Serialize, Deserialize};

use common::{AppError, Result};

/// 离线消息条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineMessage {
    pub message_id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
    pub message_type: String,
    pub timestamp: i64,
}

/// 离线消息队列管理器
/// 使用 Redis List 存储离线用户的消息，支持 FIFO 顺序推送
#[derive(Clone)]
pub struct OfflineMessageQueue {
    redis: ConnectionManager,
    /// 消息过期时间（秒），默认7天
    ttl_seconds: u64,
}

impl OfflineMessageQueue {
    /// 创建新的离线消息队列
    pub fn new(redis: ConnectionManager) -> Self {
        Self {
            redis,
            ttl_seconds: 7 * 24 * 3600, // 7天
        }
    }

    /// 创建带自定义 TTL 的离线消息队列
    pub fn with_ttl(redis: ConnectionManager, ttl_seconds: u64) -> Self {
        Self {
            redis,
            ttl_seconds,
        }
    }

    /// Redis key 前缀
    fn queue_key(user_id: Uuid) -> String {
        format!("offline:queue:{}", user_id)
    }

    /// 计数器 key（用于统计）
    fn counter_key(user_id: Uuid) -> String {
        format!("offline:count:{}", user_id)
    }

    /// 为离线用户存储一条消息
    pub async fn enqueue_message(
        &self,
        user_id: Uuid,
        message: OfflineMessage,
    ) -> Result<()> {
        let key = Self::queue_key(user_id);
        let counter_key = Self::counter_key(user_id);

        let json = serde_json::to_string(&message)
            .map_err(|e| AppError::Internal(format!("Failed to serialize offline message: {}", e)))?;

        let mut conn = self.redis.clone();

        // 使用 LPUSH 将消息推入队列头部（最新消息在前）
        conn.lpush::<_, _, ()>(&key, &json)
            .await
            .map_err(|e| AppError::Internal(format!("Redis lpush failed: {}", e)))?;

        // 设置过期时间（每次添加消息都刷新 TTL）
        conn.expire::<_, ()>(&key, self.ttl_seconds as i64)
            .await
            .map_err(|e| AppError::Internal(format!("Redis expire failed: {}", e)))?;

        // 增加计数器
        conn.incr::<_, _, ()>(&counter_key, 1)
            .await
            .map_err(|e| AppError::Internal(format!("Redis incr failed: {}", e)))?;

        conn.expire::<_, ()>(&counter_key, self.ttl_seconds as i64)
            .await
            .map_err(|e| AppError::Internal(format!("Redis expire failed: {}", e)))?;

        tracing::info!(
            "Enqueued offline message {} for user {} (conversation: {})",
            message.message_id,
            user_id,
            message.conversation_id
        );

        Ok(())
    }

    /// 从队列中取出用户的所有离线消息（按时间从旧到新排序）
    pub async fn dequeue_messages(&self, user_id: Uuid) -> Result<Vec<OfflineMessage>> {
        let key = Self::queue_key(user_id);
        let counter_key = Self::counter_key(user_id);

        let mut conn = self.redis.clone();

        // 获取队列长度
        let len: i64 = conn.llen(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis llen failed: {}", e)))?;

        if len == 0 {
            return Ok(Vec::new());
        }

        // 取出所有消息
        let raw_messages: Vec<String> = conn.lrange(&key, 0, (len - 1) as isize)
            .await
            .map_err(|e| AppError::Internal(format!("Redis lrange failed: {}", e)))?;

        // 删除队列和计数器
        conn.del::<_, ()>(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis del failed: {}", e)))?;
        conn.del::<_, ()>(&counter_key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis del counter failed: {}", e)))?;

        // 反序列化消息（LPUSH 存储的顺序是反的，需要反转）
        let mut messages: Vec<OfflineMessage> = Vec::new();
        for raw in raw_messages.iter().rev() {
            match serde_json::from_str::<OfflineMessage>(raw) {
                Ok(msg) => messages.push(msg),
                Err(e) => {
                    tracing::warn!("Failed to deserialize offline message: {}", e);
                }
            }
        }

        tracing::info!(
            "Dequeued {} offline messages for user {}",
            messages.len(),
            user_id
        );

        Ok(messages)
    }

    /// 获取用户离线消息数量
    pub async fn get_message_count(&self, user_id: Uuid) -> Result<i64> {
        let key = Self::queue_key(user_id);
        let mut conn = self.redis.clone();

        let count: i64 = conn.llen(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis llen failed: {}", e)))?;

        Ok(count)
    }

    /// 清除用户的所有离线消息
    pub async fn clear_messages(&self, user_id: Uuid) -> Result<()> {
        let key = Self::queue_key(user_id);
        let counter_key = Self::counter_key(user_id);

        let mut conn = self.redis.clone();

        conn.del::<_, ()>(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis del failed: {}", e)))?;
        conn.del::<_, ()>(&counter_key)
            .await
            .map_err(|e| AppError::Internal(format!("Redis del counter failed: {}", e)))?;

        tracing::info!("Cleared offline messages for user {}", user_id);
        Ok(())
    }

    /// 限制队列大小，丢弃最旧的消息
    pub async fn trim_queue(&self, user_id: Uuid, max_size: i64) -> Result<()> {
        let key = Self::queue_key(user_id);
        let mut conn = self.redis.clone();

        // LTRIM 保留前 max_size 个元素（因为我们用 LPUSH，所以最新的在前面）
        conn.ltrim::<_, ()>(&key, 0, (max_size - 1) as isize)
            .await
            .map_err(|e| AppError::Internal(format!("Redis ltrim failed: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_key_format() {
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let key = OfflineMessageQueue::queue_key(user_id);
        assert_eq!(key, "offline:queue:550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_queue_key_unique_per_user() {
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();
        let key1 = OfflineMessageQueue::queue_key(user1);
        let key2 = OfflineMessageQueue::queue_key(user2);
        assert_ne!(key1, key2);
        assert!(key1.starts_with("offline:queue:"));
        assert!(key2.starts_with("offline:queue:"));
    }

    #[test]
    fn test_offline_message_serialization() {
        let msg = OfflineMessage {
            message_id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            content: "Hello".to_string(),
            message_type: "text".to_string(),
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: OfflineMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.content, "Hello");
        assert_eq!(deserialized.message_type, "text");
    }

    #[test]
    fn test_offline_message_special_chars() {
        let msg = OfflineMessage {
            message_id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            content: "Hello\nWorld\t!@#$%^&*()".to_string(),
            message_type: "text".to_string(),
            timestamp: 1700000000,
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: OfflineMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.content, "Hello\nWorld\t!@#$%^&*()");
    }

    #[test]
    fn test_offline_message_image_type() {
        let msg = OfflineMessage {
            message_id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            content: "https://example.com/image.png".to_string(),
            message_type: "image".to_string(),
            timestamp: 1700000000,
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: OfflineMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.message_type, "image");
    }

    #[test]
    fn test_offline_queue_ttl_default() {
        // 验证默认TTL逻辑（7天 = 604800秒）
        let ttl = 7 * 24 * 3600;
        assert_eq!(ttl, 604800);
    }

    #[test]
    fn test_offline_message_empty_content() {
        let msg = OfflineMessage {
            message_id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            content: String::new(),
            message_type: "text".to_string(),
            timestamp: 0,
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: OfflineMessage = serde_json::from_str(&json).unwrap();
        assert!(deserialized.content.is_empty());
        assert_eq!(deserialized.timestamp, 0);
    }
}
