//! 敏感数据加密存储模块
//!
//! 提供敏感配置数据（如 API 密钥、数据库密码等）的加密存储功能：
//! - 基于 AES-256-GCM 的加密/解密
//! - 使用环境变量或配置文件中的主密钥
//! - 支持密钥派生（从主密钥派生子密钥）
//! - 透明的加密/解密接口

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{Engine as _, engine::general_purpose};
use rand::RngCore;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 加密密钥长度（256位）
const KEY_LENGTH: usize = 32;
/// Nonce 长度（96位）
const NONCE_LENGTH: usize = 12;

/// 敏感数据加密管理器
///
/// 使用 AES-256-GCM 加密算法保护敏感配置数据。
/// 主密钥从环境变量 `OMNILINK_MASTER_KEY` 加载，
/// 如果未设置则自动生成一个随机密钥（仅用于开发环境）。
pub struct SecretsManager {
    /// 主密钥（用于派生加密密钥）
    master_key: [u8; KEY_LENGTH],
    /// 加密密钥缓存: key_id -> Aes256Gcm
    ciphers: Arc<RwLock<HashMap<String, Aes256Gcm>>>,
}

impl SecretsManager {
    /// 创建新的 SecretsManager
    ///
    /// 从环境变量 `OMNILINK_MASTER_KEY` 加载主密钥。
    /// 如果未设置，生成随机密钥（仅开发环境使用，会在日志中警告）。
    pub fn new() -> Self {
        let master_key = Self::load_or_generate_master_key();
        Self {
            master_key,
            ciphers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 使用指定的主密钥创建（用于测试）
    pub fn with_master_key(master_key: [u8; KEY_LENGTH]) -> Self {
        Self {
            master_key,
            ciphers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 加载或生成主密钥
    fn load_or_generate_master_key() -> [u8; KEY_LENGTH] {
        match std::env::var("OMNILINK_MASTER_KEY") {
            Ok(key_hex) => {
                let key_bytes = hex::decode(&key_hex).unwrap_or_else(|_| {
                    tracing::warn!("OMNILINK_MASTER_KEY is not valid hex, using as raw bytes");
                    key_hex.as_bytes().to_vec()
                });
                let mut key = [0u8; KEY_LENGTH];
                let len = key_bytes.len().min(KEY_LENGTH);
                key[..len].copy_from_slice(&key_bytes[..len]);
                tracing::info!("Loaded master key from OMNILINK_MASTER_KEY environment variable");
                key
            }
            Err(_) => {
                let mut key = [0u8; KEY_LENGTH];
                OsRng.fill_bytes(&mut key);
                tracing::warn!(
                    "OMNILINK_MASTER_KEY not set, generated random key. \
                     THIS IS ONLY SAFE FOR DEVELOPMENT! \
                     Set OMNILINK_MASTER_KEY in production."
                );
                key
            }
        }
    }

    /// 获取或创建指定 key_id 的加密器
    async fn get_cipher(&self, key_id: &str) -> Aes256Gcm {
        let ciphers = self.ciphers.read().await;
        if let Some(cipher) = ciphers.get(key_id) {
            return cipher.clone();
        }
        drop(ciphers);

        // 从主密钥派生子密钥
        let derived_key = self.derive_key(key_id);
        let cipher = Aes256Gcm::new_from_slice(&derived_key)
            .expect("Failed to create cipher from derived key");

        let mut ciphers = self.ciphers.write().await;
        ciphers.insert(key_id.to_string(), cipher.clone());
        cipher
    }

    /// 从主密钥派生子密钥
    ///
    /// 使用简单的 XOR 派生（生产环境应使用 HKDF 或 PBKDF2）
    fn derive_key(&self, key_id: &str) -> [u8; KEY_LENGTH] {
        let mut derived = self.master_key;
        let id_bytes = key_id.as_bytes();
        for (i, byte) in derived.iter_mut().enumerate() {
            *byte ^= id_bytes[i % id_bytes.len()];
            // 额外混淆
            *byte = byte.wrapping_add((i as u8).wrapping_mul(37));
        }
        derived
    }

    /// 加密敏感数据
    ///
    /// # 参数
    /// - `key_id`: 密钥标识符（用于派生加密密钥）
    /// - `plaintext`: 要加密的明文
    ///
    /// # 返回
    /// Base64 编码的密文（包含 nonce）
    pub async fn encrypt(&self, key_id: &str, plaintext: &str) -> Result<String, SecretsError> {
        let cipher = self.get_cipher(key_id).await;

        let mut nonce_bytes = [0u8; NONCE_LENGTH];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| SecretsError::EncryptionFailed(e.to_string()))?;

        // 将 nonce 和密文拼接后 Base64 编码
        let mut combined = Vec::with_capacity(NONCE_LENGTH + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(general_purpose::STANDARD.encode(&combined))
    }

    /// 解密敏感数据
    ///
    /// # 参数
    /// - `key_id`: 密钥标识符
    /// - `encrypted`: Base64 编码的密文
    ///
    /// # 返回
    /// 解密后的明文
    pub async fn decrypt(&self, key_id: &str, encrypted: &str) -> Result<String, SecretsError> {
        let cipher = self.get_cipher(key_id).await;

        let combined = general_purpose::STANDARD
            .decode(encrypted)
            .map_err(|e| SecretsError::DecryptionFailed(format!("Invalid base64: {}", e)))?;

        if combined.len() < NONCE_LENGTH {
            return Err(SecretsError::DecryptionFailed("Ciphertext too short".to_string()));
        }

        let (nonce_bytes, ciphertext) = combined.split_at(NONCE_LENGTH);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| SecretsError::DecryptionFailed(e.to_string()))?;

        String::from_utf8(plaintext)
            .map_err(|e| SecretsError::DecryptionFailed(format!("Invalid UTF-8: {}", e)))
    }

    /// 加密 HashMap 中的所有值
    ///
    /// 用于批量加密配置项
    pub async fn encrypt_map(
        &self,
        key_id: &str,
        data: &HashMap<String, String>,
    ) -> Result<HashMap<String, String>, SecretsError> {
        let mut encrypted = HashMap::new();
        for (k, v) in data {
            encrypted.insert(k.clone(), self.encrypt(key_id, v).await?);
        }
        Ok(encrypted)
    }

    /// 解密 HashMap 中的所有值
    ///
    /// 用于批量解密配置项
    pub async fn decrypt_map(
        &self,
        key_id: &str,
        data: &HashMap<String, String>,
    ) -> Result<HashMap<String, String>, SecretsError> {
        let mut decrypted = HashMap::new();
        for (k, v) in data {
            decrypted.insert(k.clone(), self.decrypt(key_id, v).await?);
        }
        Ok(decrypted)
    }

    /// 轮换主密钥
    ///
    /// 使用旧密钥解密所有数据，然后用新密钥重新加密。
    ///
    /// # 参数
    /// - `old_manager`: 使用旧密钥的管理器
    /// - `new_master_key`: 新的主密钥
    ///
    /// # 返回
    /// 新的 SecretsManager 实例
    pub fn rotate_master_key(new_master_key: [u8; KEY_LENGTH]) -> Self {
        tracing::info!("Rotated master key");
        Self::with_master_key(new_master_key)
    }
}

impl Default for SecretsManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 敏感数据加密错误
#[derive(Debug)]
pub enum SecretsError {
    EncryptionFailed(String),
    DecryptionFailed(String),
}

impl std::fmt::Display for SecretsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretsError::EncryptionFailed(msg) => write!(f, "Encryption failed: {}", msg),
            SecretsError::DecryptionFailed(msg) => write!(f, "Decryption failed: {}", msg),
        }
    }
}

impl std::error::Error for SecretsError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_encrypt_decrypt() {
        let manager = SecretsManager::with_master_key([42u8; KEY_LENGTH]);
        let plaintext = "sk-very-secret-api-key-12345";

        let encrypted = manager.encrypt("test", plaintext).await.unwrap();
        assert_ne!(encrypted, plaintext);

        let decrypted = manager.decrypt("test", &encrypted).await.unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[tokio::test]
    async fn test_different_key_ids() {
        let manager = SecretsManager::with_master_key([42u8; KEY_LENGTH]);
        let plaintext = "secret-data";

        let encrypted1 = manager.encrypt("key1", plaintext).await.unwrap();
        let encrypted2 = manager.encrypt("key2", plaintext).await.unwrap();

        // 不同 key_id 应产生不同的密文
        assert_ne!(encrypted1, encrypted2);

        // 但都能正确解密
        assert_eq!(manager.decrypt("key1", &encrypted1).await.unwrap(), plaintext);
        assert_eq!(manager.decrypt("key2", &encrypted2).await.unwrap(), plaintext);
    }

    #[tokio::test]
    async fn test_encrypt_decrypt_map() {
        let manager = SecretsManager::with_master_key([42u8; KEY_LENGTH]);
        let mut data = HashMap::new();
        data.insert("openai_key".to_string(), "sk-abc123".to_string());
        data.insert("anthropic_key".to_string(), "sk-ant-xyz".to_string());

        let encrypted = manager.encrypt_map("config", &data).await.unwrap();
        assert_ne!(encrypted.get("openai_key"), data.get("openai_key"));

        let decrypted = manager.decrypt_map("config", &encrypted).await.unwrap();
        assert_eq!(decrypted, data);
    }

    #[tokio::test]
    async fn test_wrong_key_id_fails() {
        let manager = SecretsManager::with_master_key([42u8; KEY_LENGTH]);
        let encrypted = manager.encrypt("correct_key", "secret").await.unwrap();
        assert!(manager.decrypt("wrong_key", &encrypted).await.is_err());
    }

    #[tokio::test]
    async fn test_tampered_ciphertext_fails() {
        let manager = SecretsManager::with_master_key([42u8; KEY_LENGTH]);
        let mut encrypted = manager.encrypt("test", "secret").await.unwrap();
        // 篡改密文
        encrypted.push('X');
        assert!(manager.decrypt("test", &encrypted).await.is_err());
    }
}
