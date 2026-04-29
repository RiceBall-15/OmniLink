use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{Utc, Duration};
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
    refresh_token_ttl: i64,  // 刷新Token有效期（秒）
}

impl TokenManager {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            access_token_ttl: 3600 * 24 * 7,    // 7天
            refresh_token_ttl: 3600 * 24 * 30,  // 30天
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