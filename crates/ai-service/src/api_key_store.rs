//! API 密钥管理模块
//!
//! 提供 API 密钥的运行时管理和轮换功能：
//! - 从环境变量加载初始密钥
//! - 支持运行时更新密钥（无需重启服务）
//! - 密钥版本管理（保留上一个密钥用于过渡）
//! - 密钥状态查询

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

/// 单个 API 密钥条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyEntry {
    /// 密钥值（存储时脱敏，完整密钥仅在内存中）
    pub key: String,
    /// 密钥创建/更新时间
    pub updated_at: DateTime<Utc>,
    /// 密钥来源（env: 环境变量, api: API更新, rotation: 轮换）
    pub source: String,
    /// 是否激活
    pub active: bool,
}

/// API 密钥存储管理器
///
/// 支持运行时密钥轮换，所有 AI 提供商的密钥统一管理。
/// 线程安全，支持并发读写。
pub struct ApiKeyStore {
    /// 当前活跃密钥: provider_name -> ApiKeyEntry
    keys: Arc<RwLock<HashMap<String, ApiKeyEntry>>>,
    /// 密钥历史（保留上一个版本用于过渡）: provider_name -> ApiKeyEntry
    previous_keys: Arc<RwLock<HashMap<String, ApiKeyEntry>>>,
}

impl ApiKeyStore {
    /// 创建空的密钥存储
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            previous_keys: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 从环境变量加载 API 密钥
    ///
    /// 支持的环境变量：
    /// - OPENAI_API_KEY
    /// - ANTHROPIC_API_KEY
    /// - GOOGLE_API_KEY
    /// - QWEN_API_KEY
    /// - ZHIPU_API_KEY
    /// - ERNIE_API_KEY
    pub async fn load_from_env(&self) -> HashMap<String, String> {
        let env_mappings = vec![
            ("openai", "OPENAI_API_KEY"),
            ("anthropic", "ANTHROPIC_API_KEY"),
            ("google", "GOOGLE_API_KEY"),
            ("qwen", "QWEN_API_KEY"),
            ("zhipu", "ZHIPU_API_KEY"),
            ("ernie", "ERNIE_API_KEY"),
            ("ernie_secret", "ERNIE_SECRET_KEY"),
        ];

        let mut result = HashMap::new();
        let mut keys = self.keys.write().await;

        for (provider, env_var) in env_mappings {
            if let Ok(key) = std::env::var(env_var) {
                if !key.is_empty() {
                    keys.insert(provider.to_string(), ApiKeyEntry {
                        key: key.clone(),
                        updated_at: Utc::now(),
                        source: "env".to_string(),
                        active: true,
                    });
                    result.insert(provider.to_string(), key);
                }
            }
        }

        tracing::info!("Loaded {} API keys from environment variables", result.len());
        result
    }

    /// 获取指定提供商的密钥
    pub async fn get_key(&self, provider: &str) -> Option<String> {
        let keys = self.keys.read().await;
        keys.get(provider)
            .filter(|entry| entry.active)
            .map(|entry| entry.key.clone())
    }

    /// 获取所有活跃的密钥映射（用于初始化 providers）
    pub async fn get_all_keys(&self) -> HashMap<String, String> {
        let keys = self.keys.read().await;
        keys.iter()
            .filter(|(_, entry)| entry.active)
            .map(|(provider, entry)| (provider.clone(), entry.key.clone()))
            .collect()
    }

    /// 轮换（更新）指定提供商的 API 密钥
    ///
    /// 旧密钥会被保留在 previous_keys 中，以便在新密钥失效时快速回滚。
    ///
    /// # 参数
    /// - `provider`: 提供商名称（如 "openai", "anthropic"）
    /// - `new_key`: 新的 API 密钥
    ///
    /// # 返回
    /// 旧密钥条目（如果存在）
    pub async fn rotate_key(&self, provider: &str, new_key: String) -> Option<ApiKeyEntry> {
        let mut keys = self.keys.write().await;
        let mut previous = self.previous_keys.write().await;

        // 将当前密钥移入历史
        let old_entry = keys.insert(provider.to_string(), ApiKeyEntry {
            key: new_key,
            updated_at: Utc::now(),
            source: "rotation".to_string(),
            active: true,
        });

        if let Some(ref old) = old_entry {
            previous.insert(provider.to_string(), old.clone());
            tracing::info!("Rotated API key for provider: {}", provider);
        } else {
            tracing::info!("Set new API key for provider: {}", provider);
        }

        old_entry
    }

    /// 回滚到上一个密钥
    ///
    /// 当新密钥失效时，可以快速回滚到上一个密钥。
    ///
    /// # 返回
    /// true 如果回滚成功，false 如果没有历史密钥
    pub async fn rollback_key(&self, provider: &str) -> bool {
        let mut keys = self.keys.write().await;
        let mut previous = self.previous_keys.write().await;

        if let Some(prev_entry) = previous.remove(provider) {
            keys.insert(provider.to_string(), ApiKeyEntry {
                key: prev_entry.key,
                updated_at: Utc::now(),
                source: "rollback".to_string(),
                active: true,
            });
            tracing::info!("Rolled back API key for provider: {}", provider);
            true
        } else {
            tracing::warn!("No previous key to rollback for provider: {}", provider);
            false
        }
    }

    /// 禁用指定提供商的密钥
    pub async fn disable_key(&self, provider: &str) -> bool {
        let mut keys = self.keys.write().await;
        if let Some(entry) = keys.get_mut(provider) {
            entry.active = false;
            tracing::info!("Disabled API key for provider: {}", provider);
            true
        } else {
            false
        }
    }

    /// 启用指定提供商的密钥
    pub async fn enable_key(&self, provider: &str) -> bool {
        let mut keys = self.keys.write().await;
        if let Some(entry) = keys.get_mut(provider) {
            entry.active = true;
            tracing::info!("Enabled API key for provider: {}", provider);
            true
        } else {
            false
        }
    }

    /// 获取所有提供商的密钥状态（脱敏）
    pub async fn list_keys(&self) -> Vec<ApiKeyStatus> {
        let keys = self.keys.read().await;
        let previous = self.previous_keys.read().await;

        keys.iter().map(|(provider, entry)| {
            ApiKeyStatus {
                provider: provider.clone(),
                masked_key: mask_key(&entry.key),
                updated_at: entry.updated_at.to_rfc3339(),
                source: entry.source.clone(),
                active: entry.active,
                has_previous: previous.contains_key(provider),
            }
        }).collect()
    }
}

impl Default for ApiKeyStore {
    fn default() -> Self {
        Self::new()
    }
}

/// API 密钥状态（用于 API 响应，脱敏显示）
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyStatus {
    pub provider: String,
    /// 脱敏后的密钥（如 sk-abc***xyz）
    pub masked_key: String,
    pub updated_at: String,
    pub source: String,
    pub active: bool,
    /// 是否有上一个版本的密钥（可用于回滚）
    pub has_previous: bool,
}

/// 密钥轮换请求
#[derive(Debug, Deserialize)]
pub struct RotateKeyRequest {
    pub provider: String,
    pub new_key: String,
}

/// 密钥操作请求
#[derive(Debug, Deserialize)]
pub struct KeyActionRequest {
    pub provider: String,
}

/// 对密钥进行脱敏处理
///
/// 保留前6个字符和后4个字符，中间用 *** 替代
fn mask_key(key: &str) -> String {
    if key.len() <= 10 {
        "***".to_string()
    } else {
        format!("{}***{}", &key[..6], &key[key.len()-4..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_key() {
        assert_eq!(mask_key("sk-abcdefghijklmnop"), "sk-abc***mnop");
        assert_eq!(mask_key("short"), "***");
        assert_eq!(mask_key("1234567890"), "123456***7890");
    }

    #[tokio::test]
    async fn test_key_store_operations() {
        let store = ApiKeyStore::new();

        // 初始为空
        assert!(store.get_key("openai").await.is_none());

        // 轮换密钥
        store.rotate_key("openai", "sk-test-key-12345".to_string()).await;
        assert_eq!(store.get_key("openai").await, Some("sk-test-key-12345".to_string()));

        // 再次轮换
        store.rotate_key("openai", "sk-new-key-67890".to_string()).await;
        assert_eq!(store.get_key("openai").await, Some("sk-new-key-67890".to_string()));

        // 回滚
        assert!(store.rollback_key("openai").await);
        assert_eq!(store.get_key("openai").await, Some("sk-test-key-12345".to_string()));

        // 列出密钥状态
        let statuses = store.list_keys().await;
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].provider, "openai");
        assert!(statuses[0].has_previous);
    }

    #[tokio::test]
    async fn test_disable_enable() {
        let store = ApiKeyStore::new();
        store.rotate_key("openai", "sk-test".to_string()).await;

        // 禁用
        assert!(store.disable_key("openai").await);
        assert!(store.get_key("openai").await.is_none());

        // 启用
        assert!(store.enable_key("openai").await);
        assert_eq!(store.get_key("openai").await, Some("sk-test".to_string()));
    }
}
