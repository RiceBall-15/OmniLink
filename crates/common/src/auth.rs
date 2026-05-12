use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use base64::{Engine as _, engine::general_purpose};
use crate::error::{AppError, Result};

/// JWT Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,           // 用户ID
    pub exp: i64,            // 过期时间
    pub iat: i64,            // 签发时间
    pub device_id: String,   // 设备ID
    pub jti: Uuid,           // Token唯一标识
}

/// Token管理器
pub struct TokenManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    access_token_ttl: i64,   // 访问Token有效期（秒）
    _refresh_token_ttl: i64,  // 刷新Token有效期（秒）
}

impl TokenManager {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            access_token_ttl: 3600 * 24 * 7,    // 7天
            _refresh_token_ttl: 3600 * 24 * 30,  // 30天
        }
    }

    /// 生成访问Token
    pub fn generate_access_token(&self, user_id: Uuid, device_id: String) -> String {
        let now = Utc::now().timestamp();
        let claims = Claims {
            sub: user_id,
            exp: now + self.access_token_ttl,
            iat: now,
            device_id,
            jti: Uuid::new_v4(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .expect("Failed to encode token")
    }

    /// 验证Token
    pub fn verify_token(&self, token: &str) -> Result<Claims> {
        let validation = Validation::default();

        decode::<Claims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    AppError::Auth("Token expired".to_string())
                }
                _ => AppError::Auth("Invalid token".to_string()),
            })
    }
}

/// 密码管理
pub struct PasswordManager;

impl PasswordManager {
    /// 哈希密码
    pub fn hash_password(password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|_| AppError::Auth("Password hash failed".to_string()))
    }

    /// 验证密码
    pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|_| AppError::Auth("Invalid password hash".to_string()))?;

        let argon2 = Argon2::default();

        Ok(argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}

/// 加密管理器
pub struct CryptoManager {
    master_key: [u8; 32],
}

impl CryptoManager {
    pub fn new(master_key: &[u8; 32]) -> Self {
        Self {
            master_key: *master_key,
        }
    }

    /// 加密数据
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new_from_slice(&self.master_key)
            .map_err(|_| AppError::Internal("Invalid encryption key".to_string()))?;

        let nonce_bytes: [u8; 12] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| AppError::Internal("Encryption failed".to_string()))?;

        // nonce + ciphertext
        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// 解密数据
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if ciphertext.len() < 12 {
            return Err(AppError::Internal("Invalid ciphertext".to_string()));
        }

        let cipher = Aes256Gcm::new_from_slice(&self.master_key)
            .map_err(|_| AppError::Internal("Invalid encryption key".to_string()))?;

        let nonce = Nonce::from_slice(&ciphertext[..12]);
        let encrypted = &ciphertext[12..];

        cipher
            .decrypt(nonce, encrypted)
            .map_err(|_| AppError::Internal("Decryption failed".to_string()))
    }

    /// 加密API Key
    pub fn encrypt_api_key(&self, api_key: &str) -> String {
        let encrypted = self.encrypt(api_key.as_bytes()).expect("Encryption failed");
        general_purpose::STANDARD.encode(encrypted)
    }

    /// 解密API Key
    pub fn decrypt_api_key(&self, encrypted: &str) -> Result<String> {
        let bytes = general_purpose::STANDARD.decode(encrypted)
            .map_err(|_| AppError::Internal("Invalid base64".to_string()))?;
        let decrypted = self.decrypt(&bytes)?;
        String::from_utf8(decrypted)
            .map_err(|_| AppError::Internal("Invalid UTF-8".to_string()))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_token_generate_and_verify() {
        let secret = b"test-secret-key-for-jwt-tokens";
        let manager = TokenManager::new(secret);
        let user_id = Uuid::new_v4();
        let device_id = "test-device-001".to_string();

        let token = manager.generate_access_token(user_id, device_id.clone());
        assert!(!token.is_empty());

        let claims = manager.verify_token(&token).unwrap();
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.device_id, device_id);
    }

    #[test]
    fn test_token_invalid() {
        let manager = TokenManager::new(b"secret");
        let result = manager.verify_token("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_token_wrong_secret() {
        let manager1 = TokenManager::new(b"secret-one");
        let manager2 = TokenManager::new(b"secret-two");
        let user_id = Uuid::new_v4();

        let token = manager1.generate_access_token(user_id, "device".to_string());
        let result = manager2.verify_token(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_password_hash_and_verify() {
        let password = "MySecure@Pass123";
        let hash = PasswordManager::hash_password(password).unwrap();

        assert!(PasswordManager::verify_password(password, &hash).unwrap());
        assert!(!PasswordManager::verify_password("wrong-password", &hash).unwrap());
    }

    #[test]
    fn test_password_different_hashes() {
        let password = "same-password";
        let hash1 = PasswordManager::hash_password(password).unwrap();
        let hash2 = PasswordManager::hash_password(password).unwrap();

        // Different salts produce different hashes
        assert_ne!(hash1, hash2);

        // But both verify correctly
        assert!(PasswordManager::verify_password(password, &hash1).unwrap());
        assert!(PasswordManager::verify_password(password, &hash2).unwrap());
    }

    #[test]
    fn test_crypto_encrypt_decrypt() {
        let key: [u8; 32] = rand::random();
        let crypto = CryptoManager::new(&key);
        let plaintext = b"Hello, OmniLink encrypted world!";

        let encrypted = crypto.encrypt(plaintext).unwrap();
        assert_ne!(encrypted, plaintext.to_vec());

        let decrypted = crypto.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_crypto_api_key_roundtrip() {
        let key: [u8; 32] = rand::random();
        let crypto = CryptoManager::new(&key);
        let api_key = "sk-proj-1234567890abcdef";

        let encrypted = crypto.encrypt_api_key(api_key);
        assert_ne!(encrypted, api_key);

        let decrypted = crypto.decrypt_api_key(&encrypted).unwrap();
        assert_eq!(decrypted, api_key);
    }

    #[test]
    fn test_crypto_decrypt_too_short() {
        let key: [u8; 32] = rand::random();
        let crypto = CryptoManager::new(&key);
        let result = crypto.decrypt(&[1, 2, 3]); // Less than 12 bytes
        assert!(result.is_err());
    }
}
