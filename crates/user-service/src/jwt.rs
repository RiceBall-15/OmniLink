use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;
use crate::error::{AppError, Result};

/// JWT Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: Uuid,  // 用户ID
    pub exp: i64,       // 过期时间
    pub iat: i64,       // 签发时间
}

/// JWT Token 管理器
pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    token_ttl: i64,  // Token有效期（秒），默认 7 天
}

impl JwtManager {
    /// 创建新的 JWT 管理器
    ///
    /// # Arguments
    /// * `secret` - JWT 密钥，从环境变量 JWT_SECRET 读取（默认值：your-secret-key）
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            token_ttl: 7 * 24 * 60 * 60,  // 7天
        }
    }

    /// 创建新的 JWT 管理器（支持自定义过期时间）
    ///
    /// # Arguments
    /// * `secret` - JWT 密钥
    /// * `token_ttl` - Token 有效期（秒）
    pub fn with_ttl(secret: &[u8], token_ttl: i64) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            token_ttl,
        }
    }

    /// 生成 JWT Token
    ///
    /// 使用 HS256 算法生成 JWT Token
    ///
    /// # Arguments
    /// * `user_id` - 用户 ID
    ///
    /// # Returns
    /// 返回 JWT Token 字符串
    pub fn generate_token(&self, user_id: Uuid) -> String {
        let now = Utc::now().timestamp();
        let claims = Claims {
            user_id,
            exp: now + self.token_ttl,
            iat: now,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .expect("Failed to encode token")
    }

    /// 验证 JWT Token
    ///
    /// 验证 Token 签名和过期时间，提取用户 ID
    ///
    /// # Arguments
    /// * `token` - JWT Token 字符串
    ///
    /// # Returns
    /// 返回 Claims（包含 user_id）
    ///
    /// # Errors
    /// 返回错误如果 Token 无效或已过期
    pub fn verify_token(&self, token: &str) -> Result<Claims> {
        let validation = Validation::default();

        decode::<Claims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    AppError::Auth("Token has expired".to_string())
                }
                jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                    AppError::Auth("Invalid token signature".to_string())
                }
                jsonwebtoken::errors::ErrorKind::InvalidToken => {
                    AppError::Auth("Invalid token format".to_string())
                }
                _ => AppError::Auth(format!("Token verification failed: {}", e)),
            })
    }

    /// 从 Token 中提取用户 ID
    ///
    /// # Arguments
    /// * `token` - JWT Token 字符串
    ///
    /// # Returns
    /// 返回用户 ID
    pub fn extract_user_id(&self, token: &str) -> Result<Uuid> {
        let claims = self.verify_token(token)?;
        Ok(claims.user_id)
    }
}
