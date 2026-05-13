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
