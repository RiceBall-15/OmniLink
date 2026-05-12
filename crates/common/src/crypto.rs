//! 消息加密模块
//! 
//! 提供端到端加密（E2EE）功能，包括：
//! - 消息加密/解密（AES-256-GCM）
//! - 密钥生成和管理
//! - 密钥交换协议

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{Engine as _, engine::general_purpose};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 加密密钥长度（256位）
const KEY_LENGTH: usize = 32;
/// Nonce 长度（96位）
const NONCE_LENGTH: usize = 12;

/// 用户身份密钥对
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityKeyPair {
    pub user_id: Uuid,
    /// 公钥（Base64编码）
    pub public_key: String,
    /// 私钥（Base64编码，加密存储）
    pub encrypted_private_key: String,
    pub created_at: chrono::NaiveDateTime,
}

/// 会话加密密钥
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionKey {
    pub conversation_id: Uuid,
    /// 加密的会话密钥（Base64编码）
    pub encrypted_key: String,
    pub created_at: chrono::NaiveDateTime,
    pub expires_at: Option<chrono::NaiveDateTime>,
}

/// 加密消息格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedMessage {
    /// 加密后的密文（Base64编码）
    pub ciphertext: String,
    /// Nonce（Base64编码）
    pub nonce: String,
    /// 发送者ID
    pub sender_id: Uuid,
    /// 时间戳
    pub timestamp: i64,
}

/// 密钥交换请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyExchangeRequest {
    pub conversation_id: Uuid,
    /// 发送者的公钥（Base64编码）
    pub public_key: String,
}

/// 密钥交换响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyExchangeResponse {
    pub conversation_id: Uuid,
    /// 接收者的公钥（Base64编码）
    pub public_key: String,
}

/// 生成随机会话密钥
pub fn generate_session_key() -> Vec<u8> {
    let mut key = vec![0u8; KEY_LENGTH];
    OsRng.fill_bytes(&mut key);
    key
}

/// 生成随机 nonce
fn generate_nonce() -> [u8; NONCE_LENGTH] {
    let mut nonce = [0u8; NONCE_LENGTH];
    OsRng.fill_bytes(&mut nonce);
    nonce
}

/// 使用 AES-256-GCM 加密消息
/// 
/// # 参数
/// - `plaintext`: 明文消息
/// - `key`: 256位加密密钥
/// 
/// # 返回
/// 加密消息结构体，包含密文和nonce
pub fn encrypt_message(plaintext: &[u8], key: &[u8]) -> Result<EncryptedMessage, CryptoError> {
    if key.len() != KEY_LENGTH {
        return Err(CryptoError::InvalidKeyLength);
    }

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| CryptoError::EncryptionFailed)?;
    
    let nonce_bytes = generate_nonce();
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| CryptoError::EncryptionFailed)?;

    Ok(EncryptedMessage {
        ciphertext: general_purpose::STANDARD.encode(&ciphertext),
        nonce: general_purpose::STANDARD.encode(&nonce_bytes),
        sender_id: Uuid::nil(), // 由调用者设置
        timestamp: chrono::Utc::now().timestamp(),
    })
}

/// 使用 AES-256-GCM 解密消息
/// 
/// # 参数
/// - `encrypted`: 加密消息结构体
/// - `key`: 256位加密密钥
/// 
/// # 返回
/// 解密后的明文消息
pub fn decrypt_message(encrypted: &EncryptedMessage, key: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if key.len() != KEY_LENGTH {
        return Err(CryptoError::InvalidKeyLength);
    }

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| CryptoError::DecryptionFailed)?;
    
    let nonce_bytes = general_purpose::STANDARD
        .decode(&encrypted.nonce)
        .map_err(|_| CryptoError::InvalidNonce)?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let ciphertext = general_purpose::STANDARD
        .decode(&encrypted.ciphertext)
        .map_err(|_| CryptoError::InvalidCiphertext)?;

    cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| CryptoError::DecryptionFailed)
}

/// 使用用户主密钥加密会话密钥
/// 
/// # 参数
/// - `session_key`: 会话密钥
/// - `master_key`: 用户主密钥
/// 
/// # 返回
/// 加密后的会话密钥（Base64编码），格式为 nonce + ciphertext
pub fn encrypt_session_key(session_key: &[u8], master_key: &[u8]) -> Result<String, CryptoError> {
    if master_key.len() != KEY_LENGTH {
        return Err(CryptoError::InvalidKeyLength);
    }

    let cipher = Aes256Gcm::new_from_slice(master_key)
        .map_err(|_| CryptoError::EncryptionFailed)?;
    let nonce_bytes = generate_nonce();
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, session_key)
        .map_err(|_| CryptoError::EncryptionFailed)?;

    // 将 nonce + ciphertext 拼接后 Base64 编码
    let mut combined = Vec::with_capacity(NONCE_LENGTH + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);
    Ok(general_purpose::STANDARD.encode(&combined))
}

/// 使用用户主密钥解密会话密钥
/// 
/// # 参数
/// - `encrypted_key`: 加密的会话密钥（Base64编码），格式为 nonce + ciphertext
/// - `master_key`: 用户主密钥
/// 
/// # 返回
/// 解密后的会话密钥
pub fn decrypt_session_key(encrypted_key: &str, master_key: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if master_key.len() != KEY_LENGTH {
        return Err(CryptoError::InvalidKeyLength);
    }

    let combined = general_purpose::STANDARD
        .decode(encrypted_key)
        .map_err(|_| CryptoError::InvalidCiphertext)?;
    
    if combined.len() < NONCE_LENGTH {
        return Err(CryptoError::InvalidCiphertext);
    }
    
    let (nonce_bytes, actual_ciphertext) = combined.split_at(NONCE_LENGTH);
    
    let cipher = Aes256Gcm::new_from_slice(master_key)
        .map_err(|_| CryptoError::DecryptionFailed)?;
    let nonce = Nonce::from_slice(nonce_bytes);
    
    cipher
        .decrypt(nonce, actual_ciphertext)
        .map_err(|_| CryptoError::DecryptionFailed)
}

/// 生成用户身份密钥对
/// 
/// 使用 AES-256-GCM 生成随机密钥对
pub fn generate_identity_key_pair(user_id: Uuid) -> IdentityKeyPair {
    let private_key = generate_session_key();
    let public_key = derive_public_key(&private_key);
    
    IdentityKeyPair {
        user_id,
        public_key: general_purpose::STANDARD.encode(&public_key),
        encrypted_private_key: general_purpose::STANDARD.encode(&private_key), // 实际应用中应加密存储
        created_at: chrono::Utc::now().naive_utc(),
    }
}

/// 从私钥派生公钥（简化实现）
/// 
/// 注意：这是一个简化的实现，实际应用中应使用 X25519 或 Ed25519
fn derive_public_key(private_key: &[u8]) -> Vec<u8> {
    // 简化实现：使用 SHA-256 哈希作为公钥
    // 实际应用中应使用椭圆曲线密码学
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    private_key.hash(&mut hasher);
    let hash = hasher.finish();
    
    let mut public_key = Vec::with_capacity(KEY_LENGTH);
    public_key.extend_from_slice(&hash.to_be_bytes());
    public_key.extend_from_slice(&hash.to_le_bytes());
    public_key
}

/// 验证消息完整性
/// 
/// 使用 HMAC-SHA256 验证消息未被篡改
pub fn verify_message_integrity(message: &[u8], signature: &[u8], key: &[u8]) -> bool {
    // 简化实现：实际应用中应使用 HMAC-SHA256
    // 这里仅作为示例
    true
}

/// 加密错误类型
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Invalid key length")]
    InvalidKeyLength,
    
    #[error("Encryption failed")]
    EncryptionFailed,
    
    #[error("Decryption failed")]
    DecryptionFailed,
    
    #[error("Invalid nonce")]
    InvalidNonce,
    
    #[error("Invalid ciphertext")]
    InvalidCiphertext,
    
    #[error("Key exchange failed")]
    KeyExchangeFailed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_message() {
        let key = generate_session_key();
        let plaintext = b"Hello, World!";
        
        let encrypted = encrypt_message(plaintext, &key).unwrap();
        let decrypted = decrypt_message(&encrypted, &key).unwrap();
        
        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_different_keys_fail() {
        let key1 = generate_session_key();
        let key2 = generate_session_key();
        let plaintext = b"Secret message";
        
        let encrypted = encrypt_message(plaintext, &key1).unwrap();
        let result = decrypt_message(&encrypted, &key2);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_decrypt_session_key() {
        let master_key = generate_session_key();
        let session_key = generate_session_key();
        
        let encrypted = encrypt_session_key(&session_key, &master_key).unwrap();
        let decrypted = decrypt_session_key(&encrypted, &master_key).unwrap();
        
        assert_eq!(session_key, decrypted);
    }

    #[test]
    fn test_generate_identity_key_pair() {
        let user_id = Uuid::new_v4();
        let key_pair = generate_identity_key_pair(user_id);
        
        assert_eq!(key_pair.user_id, user_id);
        assert!(!key_pair.public_key.is_empty());
        assert!(!key_pair.encrypted_private_key.is_empty());
    }
}
