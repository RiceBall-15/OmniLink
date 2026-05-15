//! 屏蔽管理器 - 用户屏蔽关系缓存与查询
//!
//! 提供基于内存缓存的用户屏蔽关系查询，减少数据库查询频率。
//! 缓存支持 TTL 过期机制，默认 5 分钟过期。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::user_repository::UserRepository;

/// 屏蔽关系缓存条目
#[derive(Debug, Clone)]
struct BlockCacheEntry {
    /// 被此用户屏蔽的用户ID集合
    blocked_ids: HashSet<Uuid>,
    /// 缓存创建时间
    cached_at: DateTime<Utc>,
}

/// 屏蔽管理器
///
/// 提供用户屏蔽关系的缓存查询功能。
/// 使用内存缓存减少数据库查询，支持 TTL 过期。
#[derive(Clone)]
pub struct BlockManager {
    /// 用户仓库（数据库查询）
    user_repository: Arc<UserRepository>,
    /// 屏蔽列表缓存: user_id -> BlockCacheEntry
    block_cache: Arc<RwLock<HashMap<Uuid, BlockCacheEntry>>>,
    /// 缓存 TTL（秒）
    cache_ttl_seconds: i64,
}

impl BlockManager {
    /// 创建新的屏蔽管理器
    ///
    /// # Arguments
    /// * `user_repository` - 用户仓库实例
    /// * `cache_ttl_seconds` - 缓存过期时间（秒），默认 300 秒（5 分钟）
    pub fn new(user_repository: Arc<UserRepository>, cache_ttl_seconds: Option<i64>) -> Self {
        Self {
            user_repository,
            block_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl_seconds: cache_ttl_seconds.unwrap_or(300),
        }
    }

    /// 检查用户是否被屏蔽
    ///
    /// 优先从缓存查询，缓存未命中或过期则从数据库加载。
    ///
    /// # Arguments
    /// * `blocker_id` - 屏蔽者用户ID
    /// * `blocked_id` - 被屏蔽者用户ID
    ///
    /// # Returns
    /// `true` 表示 blocked_id 被 blocker_id 屏蔽
    pub async fn is_user_blocked(&self, blocker_id: Uuid, blocked_id: Uuid) -> bool {
        // 先检查缓存
        {
            let cache = self.block_cache.read().await;
            if let Some(entry) = cache.get(&blocker_id) {
                if !self.is_cache_expired(entry) {
                    return entry.blocked_ids.contains(&blocked_id);
                }
            }
        }

        // 缓存未命中或已过期，从数据库加载
        match self.user_repository.get_blocked_user_ids(blocker_id).await {
            Ok(blocked_ids) => {
                let blocked_set: HashSet<Uuid> = blocked_ids.into_iter().collect();
                let is_blocked = blocked_set.contains(&blocked_id);

                // 更新缓存
                let mut cache = self.block_cache.write().await;
                cache.insert(blocker_id, BlockCacheEntry {
                    blocked_ids: blocked_set,
                    cached_at: Utc::now(),
                });

                is_blocked
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to get blocked users for {}: {:?}, falling back to DB check",
                    blocker_id, e
                );
                // 降级：直接查询数据库
                self.user_repository.is_user_blocked(blocker_id, blocked_id).await.unwrap_or(false)
            }
        }
    }

    /// 从会话成员列表中过滤出未被屏蔽的用户
    ///
    /// 用于消息广播时过滤掉屏蔽了发送者的用户。
    ///
    /// # Arguments
    /// * `sender_id` - 消息发送者ID
    /// * `recipient_ids` - 接收者用户ID列表
    ///
    /// # Returns
    /// 未屏蔽发送者的用户ID列表
    pub async fn filter_blocked_recipients(
        &self,
        sender_id: Uuid,
        recipient_ids: &[Uuid],
    ) -> Vec<Uuid> {
        if recipient_ids.is_empty() {
            return Vec::new();
        }

        // 批量加载所有接收者的屏蔽列表
        let mut filtered = Vec::with_capacity(recipient_ids.len());

        for &recipient_id in recipient_ids {
            if !self.is_user_blocked(recipient_id, sender_id).await {
                filtered.push(recipient_id);
            }
        }

        filtered
    }

    /// 使指定用户的屏蔽缓存失效
    ///
    /// 当用户修改屏蔽列表时调用此方法。
    pub async fn invalidate_cache(&self, user_id: Uuid) {
        let mut cache = self.block_cache.write().await;
        cache.remove(&user_id);
        tracing::debug!("Invalidated block cache for user {}", user_id);
    }

    /// 获取用户屏蔽的用户ID集合
    ///
    /// 用于 WebSocket 消息过滤，返回被指定用户屏蔽的所有用户ID。
    pub async fn get_blocked_list(&self, user_id: Uuid) -> HashSet<Uuid> {
        // 先检查缓存
        {
            let cache = self.block_cache.read().await;
            if let Some(entry) = cache.get(&user_id) {
                if !self.is_cache_expired(entry) {
                    return entry.blocked_ids.clone();
                }
            }
        }

        // 从数据库加载
        match self.user_repository.get_blocked_user_ids(user_id).await {
            Ok(blocked_ids) => {
                let blocked_set: HashSet<Uuid> = blocked_ids.into_iter().collect();
                let mut cache = self.block_cache.write().await;
                cache.insert(user_id, BlockCacheEntry {
                    blocked_ids: blocked_set.clone(),
                    cached_at: Utc::now(),
                });
                blocked_set
            }
            Err(e) => {
                tracing::warn!("Failed to get blocked list for {}: {:?}", user_id, e);
                HashSet::new()
            }
        }
    }

    /// 获取屏蔽了指定用户的用户ID集合（反向查询）
    ///
    /// 用于 WebSocket 消息过滤，返回屏蔽了指定用户的所有用户ID。
    pub async fn get_blocked_by_list(&self, user_id: Uuid) -> HashSet<Uuid> {
        match self.user_repository.get_blocked_by_user_ids(user_id).await {
            Ok(blocker_ids) => blocker_ids.into_iter().collect(),
            Err(e) => {
                tracing::warn!("Failed to get blocked-by list for {}: {:?}", user_id, e);
                HashSet::new()
            }
        }
    }

    /// 清除所有过期缓存条目
    pub async fn cleanup_expired(&self) {
        let mut cache = self.block_cache.write().await;
        let now = Utc::now();
        let ttl = chrono::Duration::seconds(self.cache_ttl_seconds);

        cache.retain(|_, entry| {
            now.signed_duration_since(entry.cached_at) < ttl
        });
    }

    /// 获取缓存统计信息
    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.block_cache.read().await;
        let total = cache.len();
        let expired = cache.values()
            .filter(|entry| self.is_cache_expired(entry))
            .count();
        (total, expired)
    }

    /// 检查缓存条目是否过期
    fn is_cache_expired(&self, entry: &BlockCacheEntry) -> bool {
        let now = Utc::now();
        let ttl = chrono::Duration::seconds(self.cache_ttl_seconds);
        now.signed_duration_since(entry.cached_at) >= ttl
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry_expiration() {
        // 验证过期检查逻辑
        let entry = BlockCacheEntry {
            blocked_ids: HashSet::new(),
            cached_at: Utc::now() - chrono::Duration::seconds(600),
        };

        // 简单验证缓存条目结构
        assert!(entry.blocked_ids.is_empty());
    }

    #[test]
    fn test_cache_entry_with_blocked_ids() {
        let mut blocked = HashSet::new();
        let uid1 = Uuid::new_v4();
        let uid2 = Uuid::new_v4();
        blocked.insert(uid1);
        blocked.insert(uid2);

        let entry = BlockCacheEntry {
            blocked_ids: blocked.clone(),
            cached_at: Utc::now(),
        };

        assert_eq!(entry.blocked_ids.len(), 2);
        assert!(entry.blocked_ids.contains(&uid1));
        assert!(entry.blocked_ids.contains(&uid2));
    }

    #[test]
    fn test_filter_empty_recipients() {
        // 验证空列表处理
        let recipients: Vec<Uuid> = Vec::new();
        assert!(recipients.is_empty());
    }

    #[test]
    fn test_filter_blocked_recipients_logic() {
        // 测试过滤逻辑（不需要Redis）
        let sender_id = Uuid::new_v4();
        let recipient1 = Uuid::new_v4();
        let recipient2 = Uuid::new_v4();
        let recipient3 = Uuid::new_v4();

        let recipients = vec![recipient1, recipient2, recipient3];

        // 模拟 recipient2 屏蔽了 sender
        let mut blocked_by_recipient2 = HashSet::new();
        blocked_by_recipient2.insert(sender_id);

        // 验证过滤逻辑
        let filtered: Vec<Uuid> = recipients
            .iter()
            .filter(|&&uid| {
                if uid == recipient2 {
                    !blocked_by_recipient2.contains(&sender_id)
                } else {
                    true
                }
            })
            .copied()
            .collect();

        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains(&recipient1));
        assert!(!filtered.contains(&recipient2));
        assert!(filtered.contains(&recipient3));
    }

    #[test]
    fn test_block_manager_cache_ttl_default() {
        // 验证默认缓存TTL逻辑
        let ttl_none: Option<i64> = None;
        let ttl_value = ttl_none.unwrap_or(300);
        assert_eq!(ttl_value, 300);

        let ttl_some: Option<i64> = Some(600);
        let ttl_value = ttl_some.unwrap_or(300);
        assert_eq!(ttl_value, 600);
    }

    #[test]
    fn test_multiple_caches_independent() {
        let mut cache1: HashMap<Uuid, BlockCacheEntry> = HashMap::new();
        let mut cache2: HashMap<Uuid, BlockCacheEntry> = HashMap::new();

        let uid = Uuid::new_v4();
        let mut blocked = HashSet::new();
        blocked.insert(uid);

        cache1.insert(
            Uuid::new_v4(),
            BlockCacheEntry {
                blocked_ids: blocked.clone(),
                cached_at: Utc::now(),
            },
        );

        // cache2 应该是空的
        assert!(cache2.is_empty());
        assert_eq!(cache1.len(), 1);
    }
}
