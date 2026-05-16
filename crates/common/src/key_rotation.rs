//! 会话加密密钥轮换机制
//!
//! 提供会话密钥版本管理、密钥轮换和旧密钥兼容功能。
//!
//! 核心组件：
//! - `KeyVersion`: 密钥版本信息
//! - `KeyRotationManager`: 密钥轮换管理器
//! - `ConversationKeyStore`: 会话密钥存储

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

/// 密钥版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyVersion {
    /// 密钥版本号（递增）
    pub version: u32,
    /// 密钥ID
    pub key_id: Uuid,
    /// 加密后的密钥内容（Base64编码）
    pub encrypted_key: String,
    /// 密钥创建时间
    pub created_at: DateTime<Utc>,
    /// 密钥是否已停用
    pub is_deprecated: bool,
    /// 停用时间
    pub deprecated_at: Option<DateTime<Utc>>,
    /// 密钥创建者
    pub created_by: Uuid,
}

impl KeyVersion {
    /// 创建新的密钥版本
    pub fn new(
        version: u32,
        encrypted_key: String,
        created_by: Uuid,
    ) -> Self {
        Self {
            version,
            key_id: Uuid::new_v4(),
            encrypted_key,
            created_at: Utc::now(),
            is_deprecated: false,
            deprecated_at: None,
            created_by,
        }
    }

    /// 标记密钥为已停用
    pub fn deprecate(&mut self) {
        self.is_deprecated = true;
        self.deprecated_at = Some(Utc::now());
    }

    /// 检查密钥是否有效（未停用）
    pub fn is_valid(&self) -> bool {
        !self.is_deprecated
    }
}

/// 会话密钥存储
///
/// 管理单个会话的所有密钥版本，支持保留最近N个版本。
#[derive(Debug, Clone)]
pub struct ConversationKeyStore {
    /// 会话ID
    pub conversation_id: Uuid,
    /// 密钥版本列表（按版本号排序）
    pub versions: Vec<KeyVersion>,
    /// 当前活跃密钥版本号
    pub current_version: u32,
    /// 最大保留版本数
    pub max_versions: usize,
}

impl ConversationKeyStore {
    /// 创建新的会话密钥存储
    pub fn new(conversation_id: Uuid, max_versions: usize) -> Self {
        Self {
            conversation_id,
            versions: Vec::new(),
            current_version: 0,
            max_versions,
        }
    }

    /// 添加新密钥版本
    pub fn add_version(&mut self, encrypted_key: String, created_by: Uuid) -> &KeyVersion {
        // 停用当前版本
        if let Some(current) = self.versions.last_mut() {
            current.deprecate();
        }

        // 创建新版本
        self.current_version += 1;
        let version = KeyVersion::new(self.current_version, encrypted_key, created_by);
        self.versions.push(version);

        // 清理旧版本（保留最近 max_versions 个）
        self.cleanup_old_versions();

        self.versions.last().unwrap()
    }

    /// 获取当前活跃密钥
    pub fn get_current_key(&self) -> Option<&KeyVersion> {
        self.versions.last().filter(|v| v.is_valid())
    }

    /// 根据版本号获取密钥
    pub fn get_key_by_version(&self, version: u32) -> Option<&KeyVersion> {
        self.versions.iter().find(|v| v.version == version)
    }

    /// 根据密钥ID获取密钥
    pub fn get_key_by_id(&self, key_id: &Uuid) -> Option<&KeyVersion> {
        self.versions.iter().find(|v| v.key_id == *key_id)
    }

    /// 获取所有有效密钥版本（用于解密历史消息）
    pub fn get_all_valid_keys(&self) -> Vec<&KeyVersion> {
        self.versions.iter().collect()
    }

    /// 清理旧版本，保留最近 max_versions 个
    fn cleanup_old_versions(&mut self) {
        if self.versions.len() > self.max_versions {
            let remove_count = self.versions.len() - self.max_versions;
            self.versions.drain(0..remove_count);
        }
    }

    /// 获取密钥版本数量
    pub fn version_count(&self) -> usize {
        self.versions.len()
    }
}

/// 密钥轮换管理器
///
/// 管理所有会话的密钥轮换，提供统一的密钥管理接口。
#[derive(Clone)]
pub struct KeyRotationManager {
    /// 会话密钥存储 (conversation_id -> KeyStore)
    stores: Arc<RwLock<HashMap<Uuid, ConversationKeyStore>>>,
    /// 默认最大保留版本数
    default_max_versions: usize,
}

impl KeyRotationManager {
    /// 创建新的密钥轮换管理器
    pub fn new(default_max_versions: usize) -> Self {
        Self {
            stores: Arc::new(RwLock::new(HashMap::new())),
            default_max_versions,
        }
    }

    /// 使用默认配置创建（保留最近3个版本）
    pub fn with_default_config() -> Self {
        Self::new(3)
    }

    /// 为会话创建初始密钥
    pub async fn create_initial_key(
        &self,
        conversation_id: Uuid,
        encrypted_key: String,
        created_by: Uuid,
    ) -> KeyVersion {
        let mut stores = self.stores.write().await;
        let store = stores
            .entry(conversation_id)
            .or_insert_with(|| ConversationKeyStore::new(conversation_id, self.default_max_versions));

        let version = store.add_version(encrypted_key, created_by);
        version.clone()
    }

    /// 轮换会话密钥
    ///
    /// 停用当前密钥，创建新密钥版本。
    pub async fn rotate_key(
        &self,
        conversation_id: Uuid,
        new_encrypted_key: String,
        rotated_by: Uuid,
    ) -> Result<KeyRotationResult, KeyRotationError> {
        let mut stores = self.stores.write().await;
        let store = stores
            .entry(conversation_id)
            .or_insert_with(|| ConversationKeyStore::new(conversation_id, self.default_max_versions));

        let previous_version = store.current_version;
        let new_version = store.add_version(new_encrypted_key, rotated_by);

        Ok(KeyRotationResult {
            conversation_id,
            previous_version,
            new_version: new_version.version,
            key_id: new_version.key_id,
            rotated_at: new_version.created_at,
            rotated_by,
        })
    }

    /// 获取会话当前密钥
    pub async fn get_current_key(&self, conversation_id: Uuid) -> Option<KeyVersion> {
        let stores = self.stores.read().await;
        stores
            .get(&conversation_id)
            .and_then(|store| store.get_current_key())
            .cloned()
    }

    /// 根据版本号获取密钥（用于解密历史消息）
    pub async fn get_key_by_version(
        &self,
        conversation_id: Uuid,
        version: u32,
    ) -> Option<KeyVersion> {
        let stores = self.stores.read().await;
        stores
            .get(&conversation_id)
            .and_then(|store| store.get_key_by_version(version))
            .cloned()
    }

    /// 获取会话所有密钥版本
    pub async fn get_all_versions(&self, conversation_id: Uuid) -> Vec<KeyVersion> {
        let stores = self.stores.read().await;
        stores
            .get(&conversation_id)
            .map(|store| store.versions.clone())
            .unwrap_or_default()
    }

    /// 获取会话密钥版本数量
    pub async fn get_version_count(&self, conversation_id: Uuid) -> usize {
        let stores = self.stores.read().await;
        stores
            .get(&conversation_id)
            .map(|store| store.version_count())
            .unwrap_or_default()
    }

    /// 检查会话是否有密钥
    pub async fn has_key(&self, conversation_id: Uuid) -> bool {
        let stores = self.stores.read().await;
        stores
            .get(&conversation_id)
            .map(|store| store.get_current_key().is_some())
            .unwrap_or(false)
    }

    /// 获取所有会话的密钥状态摘要
    pub async fn get_summary(&self) -> KeyRotationSummary {
        let stores = self.stores.read().await;
        let total_conversations = stores.len();
        let mut total_versions = 0;
        let mut conversations_with_active_key = 0;

        for store in stores.values() {
            total_versions += store.version_count();
            if store.get_current_key().is_some() {
                conversations_with_active_key += 1;
            }
        }

        KeyRotationSummary {
            total_conversations,
            total_versions,
            conversations_with_active_key,
            default_max_versions: self.default_max_versions,
        }
    }
}

/// 密钥轮换结果
#[derive(Debug, Clone, Serialize)]
pub struct KeyRotationResult {
    pub conversation_id: Uuid,
    pub previous_version: u32,
    pub new_version: u32,
    pub key_id: Uuid,
    pub rotated_at: DateTime<Utc>,
    pub rotated_by: Uuid,
}

/// 密钥轮换摘要
#[derive(Debug, Clone, Serialize)]
pub struct KeyRotationSummary {
    pub total_conversations: usize,
    pub total_versions: usize,
    pub conversations_with_active_key: usize,
    pub default_max_versions: usize,
}

/// 密钥轮换错误
#[derive(Debug, Clone)]
pub enum KeyRotationError {
    /// 会话不存在
    ConversationNotFound(Uuid),
    /// 密钥生成失败
    KeyGenerationFailed(String),
    /// 内部错误
    InternalError(String),
}

impl std::fmt::Display for KeyRotationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyRotationError::ConversationNotFound(id) => {
                write!(f, "Conversation not found: {}", id)
            }
            KeyRotationError::KeyGenerationFailed(msg) => {
                write!(f, "Key generation failed: {}", msg)
            }
            KeyRotationError::InternalError(msg) => {
                write!(f, "Internal error: {}", msg)
            }
        }
    }
}

impl std::error::Error for KeyRotationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_version_new() {
        let version = KeyVersion::new(
            1,
            "encrypted_key_base64".to_string(),
            Uuid::new_v4(),
        );

        assert_eq!(version.version, 1);
        assert!(!version.is_deprecated);
        assert!(version.is_valid());
        assert!(version.deprecated_at.is_none());
    }

    #[test]
    fn test_key_version_deprecate() {
        let mut version = KeyVersion::new(
            1,
            "key".to_string(),
            Uuid::new_v4(),
        );

        assert!(version.is_valid());

        version.deprecate();
        assert!(!version.is_valid());
        assert!(version.is_deprecated);
        assert!(version.deprecated_at.is_some());
    }

    #[test]
    fn test_conversation_key_store_new() {
        let conv_id = Uuid::new_v4();
        let store = ConversationKeyStore::new(conv_id, 3);

        assert_eq!(store.conversation_id, conv_id);
        assert_eq!(store.current_version, 0);
        assert_eq!(store.max_versions, 3);
        assert!(store.versions.is_empty());
    }

    #[test]
    fn test_conversation_key_store_add_version() {
        let conv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let mut store = ConversationKeyStore::new(conv_id, 3);

        let v1 = store.add_version("key1".to_string(), user_id);
        assert_eq!(v1.version, 1);
        assert!(v1.is_valid());
        assert_eq!(store.current_version, 1);

        let v2 = store.add_version("key2".to_string(), user_id);
        assert_eq!(v2.version, 2);
        assert!(v2.is_valid());
        assert_eq!(store.current_version, 2);

        // v1 should now be deprecated
        let v1_ref = store.get_key_by_version(1).unwrap();
        assert!(!v1_ref.is_valid());
    }

    #[test]
    fn test_conversation_key_store_max_versions() {
        let conv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let mut store = ConversationKeyStore::new(conv_id, 3);

        // Add 5 versions
        for i in 1..=5 {
            store.add_version(format!("key{}", i), user_id);
        }

        // Should only keep the last 3
        assert_eq!(store.version_count(), 3);
        assert_eq!(store.current_version, 5);

        // Versions 1 and 2 should be removed
        assert!(store.get_key_by_version(1).is_none());
        assert!(store.get_key_by_version(2).is_none());

        // Versions 3, 4, 5 should exist
        assert!(store.get_key_by_version(3).is_some());
        assert!(store.get_key_by_version(4).is_some());
        assert!(store.get_key_by_version(5).is_some());
    }

    #[test]
    fn test_conversation_key_store_get_current() {
        let conv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let mut store = ConversationKeyStore::new(conv_id, 3);

        // No current key initially
        assert!(store.get_current_key().is_none());

        store.add_version("key1".to_string(), user_id);
        let current = store.get_current_key().unwrap();
        assert_eq!(current.version, 1);
        assert_eq!(current.encrypted_key, "key1");

        store.add_version("key2".to_string(), user_id);
        let current = store.get_current_key().unwrap();
        assert_eq!(current.version, 2);
        assert_eq!(current.encrypted_key, "key2");
    }

    #[test]
    fn test_conversation_key_store_get_by_id() {
        let conv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let mut store = ConversationKeyStore::new(conv_id, 3);

        let v1 = store.add_version("key1".to_string(), user_id);
        let key_id = v1.key_id;

        let found = store.get_key_by_id(&key_id).unwrap();
        assert_eq!(found.version, 1);
        assert_eq!(found.key_id, key_id);

        let not_found = store.get_key_by_id(&Uuid::new_v4());
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_key_rotation_manager_create_initial_key() {
        let manager = KeyRotationManager::with_default_config();
        let conv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let version = manager.create_initial_key(
            conv_id,
            "initial_key".to_string(),
            user_id,
        ).await;

        assert_eq!(version.version, 1);
        assert!(version.is_valid());
        assert_eq!(version.encrypted_key, "initial_key");

        // Should be able to retrieve it
        let current = manager.get_current_key(conv_id).await.unwrap();
        assert_eq!(current.version, 1);
    }

    #[tokio::test]
    async fn test_key_rotation_manager_rotate_key() {
        let manager = KeyRotationManager::with_default_config();
        let conv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        // Create initial key
        manager.create_initial_key(conv_id, "key1".to_string(), user_id).await;

        // Rotate key
        let result = manager.rotate_key(conv_id, "key2".to_string(), user_id).await.unwrap();
        assert_eq!(result.previous_version, 1);
        assert_eq!(result.new_version, 2);
        assert_eq!(result.conversation_id, conv_id);
        assert_eq!(result.rotated_by, user_id);

        // Current key should be version 2
        let current = manager.get_current_key(conv_id).await.unwrap();
        assert_eq!(current.version, 2);
        assert_eq!(current.encrypted_key, "key2");
    }

    #[tokio::test]
    async fn test_key_rotation_manager_get_by_version() {
        let manager = KeyRotationManager::with_default_config();
        let conv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        manager.create_initial_key(conv_id, "key1".to_string(), user_id).await;
        manager.rotate_key(conv_id, "key2".to_string(), user_id).await.unwrap();
        manager.rotate_key(conv_id, "key3".to_string(), user_id).await.unwrap();

        // Should be able to get all versions (within max_versions)
        let v1 = manager.get_key_by_version(conv_id, 1).await;
        let v2 = manager.get_key_by_version(conv_id, 2).await;
        let v3 = manager.get_key_by_version(conv_id, 3).await;

        // v1 might be removed due to max_versions=3
        assert!(v2.is_some());
        assert!(v3.is_some());
        assert_eq!(v2.unwrap().encrypted_key, "key2");
        assert_eq!(v3.unwrap().encrypted_key, "key3");
    }

    #[tokio::test]
    async fn test_key_rotation_manager_max_versions_cleanup() {
        let manager = KeyRotationManager::new(2); // Only keep 2 versions
        let conv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        manager.create_initial_key(conv_id, "key1".to_string(), user_id).await;
        manager.rotate_key(conv_id, "key2".to_string(), user_id).await.unwrap();
        manager.rotate_key(conv_id, "key3".to_string(), user_id).await.unwrap();

        let versions = manager.get_all_versions(conv_id).await;
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].version, 2);
        assert_eq!(versions[1].version, 3);
    }

    #[tokio::test]
    async fn test_key_rotation_manager_get_all_versions() {
        let manager = KeyRotationManager::with_default_config();
        let conv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        manager.create_initial_key(conv_id, "key1".to_string(), user_id).await;
        manager.rotate_key(conv_id, "key2".to_string(), user_id).await.unwrap();

        let versions = manager.get_all_versions(conv_id).await;
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].version, 1);
        assert_eq!(versions[1].version, 2);
    }

    #[tokio::test]
    async fn test_key_rotation_manager_has_key() {
        let manager = KeyRotationManager::with_default_config();
        let conv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        assert!(!manager.has_key(conv_id).await);

        manager.create_initial_key(conv_id, "key1".to_string(), user_id).await;
        assert!(manager.has_key(conv_id).await);
    }

    #[tokio::test]
    async fn test_key_rotation_manager_summary() {
        let manager = KeyRotationManager::with_default_config();
        let user_id = Uuid::new_v4();

        manager.create_initial_key(Uuid::new_v4(), "key1".to_string(), user_id).await;
        manager.create_initial_key(Uuid::new_v4(), "key2".to_string(), user_id).await;

        let summary = manager.get_summary().await;
        assert_eq!(summary.total_conversations, 2);
        assert_eq!(summary.total_versions, 2);
        assert_eq!(summary.conversations_with_active_key, 2);
        assert_eq!(summary.default_max_versions, 3);
    }

    #[test]
    fn test_key_rotation_result_serialization() {
        let result = KeyRotationResult {
            conversation_id: Uuid::new_v4(),
            previous_version: 1,
            new_version: 2,
            key_id: Uuid::new_v4(),
            rotated_at: Utc::now(),
            rotated_by: Uuid::new_v4(),
        };

        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["previous_version"], 1);
        assert_eq!(json["new_version"], 2);
    }

    #[test]
    fn test_key_rotation_summary_serialization() {
        let summary = KeyRotationSummary {
            total_conversations: 5,
            total_versions: 15,
            conversations_with_active_key: 5,
            default_max_versions: 3,
        };

        let json = serde_json::to_value(&summary).unwrap();
        assert_eq!(json["total_conversations"], 5);
        assert_eq!(json["total_versions"], 15);
    }

    #[test]
    fn test_key_rotation_error_display() {
        let err = KeyRotationError::ConversationNotFound(Uuid::new_v4());
        assert!(err.to_string().contains("Conversation not found"));

        let err = KeyRotationError::KeyGenerationFailed("test".to_string());
        assert!(err.to_string().contains("Key generation failed"));
    }

    #[test]
    fn test_key_version_serialization() {
        let version = KeyVersion::new(
            1,
            "key".to_string(),
            Uuid::new_v4(),
        );

        let json = serde_json::to_value(&version).unwrap();
        assert_eq!(json["version"], 1);
        assert_eq!(json["is_deprecated"], false);
        assert!(json["key_id"].is_string());
    }
}
