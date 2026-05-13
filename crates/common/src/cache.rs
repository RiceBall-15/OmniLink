//! Redis 缓存层模块
//!
//! 提供基于 Redis 的通用缓存功能，用于缓存频繁访问的数据，
//! 减少数据库查询压力，提高系统响应速度。

use anyhow::Result;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

/// Redis 缓存管理器
///
/// 封装 Redis 连接和常用的缓存操作，支持 TTL 过期、
/// JSON 序列化/反序列化、批量操作等。
#[derive(Clone)]
pub struct CacheManager {
    conn: ConnectionManager,
}

impl CacheManager {
    /// 创建新的缓存管理器
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let conn = ConnectionManager::new(client).await?;
        Ok(Self { conn })
    }

    /// 获取缓存值（反序列化为指定类型）
    pub async fn get<T: DeserializeOwned>(&mut self, key: &str) -> Result<Option<T>> {
        let result: Option<String> = self.conn.get(key).await?;
        match result {
            Some(value) => {
                let parsed: T = serde_json::from_str(&value)?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    /// 设置缓存值（带 TTL）
    pub async fn set<T: Serialize>(&mut self, key: &str, value: &T, ttl: Duration) -> Result<()> {
        let serialized = serde_json::to_string(value)?;
        self.conn.set_ex::<_, _, ()>(key, &serialized, ttl.as_secs() as u64).await?;
        Ok(())
    }

    /// 设置缓存值（不带 TTL，永久有效）
    pub async fn set_persistent<T: Serialize>(&mut self, key: &str, value: &T) -> Result<()> {
        let serialized = serde_json::to_string(value)?;
        self.conn.set::<_, _, ()>(key, &serialized).await?;
        Ok(())
    }

    /// 删除缓存
    pub async fn delete(&mut self, key: &str) -> Result<()> {
        self.conn.del::<_, ()>(key).await?;
        Ok(())
    }

    /// 批量删除缓存（支持模式匹配）
    pub async fn delete_pattern(&mut self, pattern: &str) -> Result<()> {
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut self.conn)
            .await?;

        if !keys.is_empty() {
            self.conn.del::<_, ()>(&keys).await?;
        }

        Ok(())
    }

    /// 检查缓存是否存在
    pub async fn exists(&mut self, key: &str) -> Result<bool> {
        let result: bool = self.conn.exists(key).await?;
        Ok(result)
    }

    /// 设置缓存过期时间
    pub async fn expire(&mut self, key: &str, ttl: Duration) -> Result<()> {
        self.conn.expire::<_, ()>(key, ttl.as_secs() as i64).await?;
        Ok(())
    }

    /// 获取缓存剩余 TTL
    pub async fn ttl(&mut self, key: &str) -> Result<i64> {
        let result: i64 = self.conn.ttl(key).await?;
        Ok(result)
    }

    /// 递增计数器
    pub async fn incr(&mut self, key: &str, delta: i64) -> Result<i64> {
        let result: i64 = self.conn.incr(key, delta).await?;
        Ok(result)
    }

    /// 批量获取（MGET）
    pub async fn mget<T: DeserializeOwned>(&mut self, keys: &[&str]) -> Result<Vec<Option<T>>> {
        let values: Vec<Option<String>> = self.conn.get(keys).await?;

        let mut results = Vec::with_capacity(values.len());
        for value in values {
            match value {
                Some(v) => {
                    let parsed: T = serde_json::from_str(&v)?;
                    results.push(Some(parsed));
                }
                None => results.push(None),
            }
        }

        Ok(results)
    }

    /// 获取底层 Redis 连接管理器（用于高级操作）
    pub fn connection(&self) -> &ConnectionManager {
        &self.conn
    }

    /// 获取底层 Redis 连接管理器的可变引用
    pub fn connection_mut(&mut self) -> &mut ConnectionManager {
        &mut self.conn
    }

    /// 记录缓存命中（用于统计）
    pub async fn record_hit(&mut self, key: &str) -> Result<()> {
        let stats_key = format!("omnilink:cache_stats:hits");
        self.conn.incr::<_, _, i64>(&stats_key, 1).await?;
        let specific_key = format!("omnilink:cache_stats:hits:{}", key);
        self.conn.incr::<_, _, i64>(&specific_key, 1).await?;
        Ok(())
    }

    /// 记录缓存未命中（用于统计）
    pub async fn record_miss(&mut self, key: &str) -> Result<()> {
        let stats_key = format!("omnilink:cache_stats:misses");
        self.conn.incr::<_, _, i64>(&stats_key, 1).await?;
        let specific_key = format!("omnilink:cache_stats:misses:{}", key);
        self.conn.incr::<_, _, i64>(&specific_key, 1).await?;
        Ok(())
    }

    /// 获取缓存命中率统计
    pub async fn get_hit_rate(&mut self) -> Result<CacheHitRate> {
        let hits: i64 = self.conn.get("omnilink:cache_stats:hits").await.unwrap_or(0);
        let misses: i64 = self.conn.get("omnilink:cache_stats:misses").await.unwrap_or(0);
        let total = hits + misses;
        let hit_rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };

        Ok(CacheHitRate {
            hits,
            misses,
            total,
            hit_rate,
        })
    }

    /// 重置缓存统计
    pub async fn reset_stats(&mut self) -> Result<()> {
        self.conn.del::<_, ()>("omnilink:cache_stats:hits").await?;
        self.conn.del::<_, ()>("omnilink:cache_stats:misses").await?;
        Ok(())
    }
}

/// 缓存命中率统计
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheHitRate {
    /// 命中次数
    pub hits: i64,
    /// 未命中次数
    pub misses: i64,
    /// 总请求次数
    pub total: i64,
    /// 命中率 (0.0 - 1.0)
    pub hit_rate: f64,
}

/// ETag 生成工具
///
/// 基于内容生成 ETag，用于 HTTP 缓存控制。
pub struct ETagGenerator;

impl ETagGenerator {
    /// 基于内容生成 ETag
    ///
    /// 使用内容的哈希值生成 ETag，确保内容变化时 ETag 也会变化。
    pub fn generate(content: &[u8]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        let hash = hasher.finish();
        format!("\"{:x}\"", hash)
    }

    /// 基于字符串内容生成 ETag
    pub fn generate_from_str(content: &str) -> String {
        Self::generate(content.as_bytes())
    }

    /// 基于时间戳和内容生成强 ETag
    pub fn generate_strong(content: &[u8], timestamp: i64) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        timestamp.hash(&mut hasher);
        let hash = hasher.finish();
        format!("\"{:x}\"", hash)
    }

    /// 验证 If-None-Match 头部是否匹配
    ///
    /// 返回 true 表示资源未变化（304 Not Modified）
    pub fn matches(if_none_match: &str, etag: &str) -> bool {
        // 支持 * 和逗号分隔的多个 ETag
        if if_none_match == "*" {
            return true;
        }
        if_none_match
            .split(',')
            .map(|s| s.trim())
            .any(|s| s == etag)
    }
}

/// 缓存控制头部
#[derive(Debug, Clone)]
pub struct CacheControl {
    /// max-age（秒）
    pub max_age: Option<u64>,
    /// s-maxage（秒，共享缓存）
    pub s_maxage: Option<u64>,
    /// no-cache
    pub no_cache: bool,
    /// no-store
    pub no_store: bool,
    /// must-revalidate
    pub must_revalidate: bool,
    /// private
    pub private: bool,
    /// public
    pub public: bool,
}

impl CacheControl {
    /// 创建默认的 API 缓存控制
    pub fn api_default() -> Self {
        Self {
            max_age: Some(0),
            s_maxage: None,
            no_cache: true,
            no_store: false,
            must_revalidate: true,
            private: true,
            public: false,
        }
    }

    /// 创建静态资源缓存控制
    pub fn static_asset(max_age: u64) -> Self {
        Self {
            max_age: Some(max_age),
            s_maxage: None,
            no_cache: false,
            no_store: false,
            must_revalidate: false,
            private: false,
            public: true,
        }
    }

    /// 转换为 Cache-Control 头部值
    pub fn to_header_value(&self) -> String {
        let mut parts = Vec::new();

        if self.no_cache {
            parts.push("no-cache".to_string());
        }
        if self.no_store {
            parts.push("no-store".to_string());
        }
        if self.must_revalidate {
            parts.push("must-revalidate".to_string());
        }
        if self.private {
            parts.push("private".to_string());
        }
        if self.public {
            parts.push("public".to_string());
        }
        if let Some(max_age) = self.max_age {
            parts.push(format!("max-age={}", max_age));
        }
        if let Some(s_maxage) = self.s_maxage {
            parts.push(format!("s-maxage={}", s_maxage));
        }

        parts.join(", ")
    }
}

/// 缓存键前缀常量
pub mod cache_keys {
    /// 用户信息前缀
    pub const USER_PREFIX: &str = "omnilink:user:";
    /// 会话信息前缀
    pub const CONVERSATION_PREFIX: &str = "omnilink:conv:";
    /// 消息前缀
    pub const MESSAGE_PREFIX: &str = "omnilink:msg:";
    /// 用户会话列表前缀
    pub const USER_CONVERSATIONS_PREFIX: &str = "omnilink:user_convs:";
    /// 在线状态前缀
    pub const ONLINE_PREFIX: &str = "omnilink:online:";
    /// 会话成员前缀
    pub const CONVERSATION_MEMBERS_PREFIX: &str = "omnilink:conv_members:";

    /// 生成用户缓存键
    pub fn user_key(user_id: &str) -> String {
        format!("{}{}", USER_PREFIX, user_id)
    }

    /// 生成会话缓存键
    pub fn conversation_key(conv_id: &str) -> String {
        format!("{}{}", CONVERSATION_PREFIX, conv_id)
    }

    /// 生成消息缓存键
    pub fn message_key(msg_id: &str) -> String {
        format!("{}{}", MESSAGE_PREFIX, msg_id)
    }

    /// 生成用户会话列表缓存键
    pub fn user_conversations_key(user_id: &str) -> String {
        format!("{}{}", USER_CONVERSATIONS_PREFIX, user_id)
    }

    /// 生成在线状态缓存键
    pub fn online_key(user_id: &str) -> String {
        format!("{}{}", ONLINE_PREFIX, user_id)
    }

    /// 生成会话成员缓存键
    pub fn conversation_members_key(conv_id: &str) -> String {
        format!("{}{}", CONVERSATION_MEMBERS_PREFIX, conv_id)
    }
}

/// 默认缓存 TTL 常量
pub mod cache_ttl {
    use std::time::Duration;

    /// 用户信息缓存时间：30 分钟
    pub const USER_TTL: Duration = Duration::from_secs(1800);
    /// 会话信息缓存时间：10 分钟
    pub const CONVERSATION_TTL: Duration = Duration::from_secs(600);
    /// 消息缓存时间：5 分钟
    pub const MESSAGE_TTL: Duration = Duration::from_secs(300);
    /// 会话列表缓存时间：5 分钟
    pub const CONVERSATION_LIST_TTL: Duration = Duration::from_secs(300);
    /// 在线状态缓存时间：60 秒
    pub const ONLINE_TTL: Duration = Duration::from_secs(60);
}

#[cfg(test)]
mod tests {
    use super::cache_keys::*;

    #[test]
    fn test_cache_key_generation() {
        assert_eq!(user_key("123"), "omnilink:user:123");
        assert_eq!(conversation_key("456"), "omnilink:conv:456");
        assert_eq!(message_key("789"), "omnilink:msg:789");
        assert_eq!(user_conversations_key("123"), "omnilink:user_convs:123");
        assert_eq!(online_key("123"), "omnilink:online:123");
        assert_eq!(conversation_members_key("456"), "omnilink:conv_members:456");
    }
}
