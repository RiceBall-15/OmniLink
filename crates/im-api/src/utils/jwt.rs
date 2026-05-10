use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use chrono::{Duration, Utc};
use crate::models::auth::Claims;

/// JWT 密钥（生产环境应该从环境变量读取）
const JWT_SECRET: &str = "your-secret-key-change-in-production";

/// Token 过期时间（7天）
const TOKEN_EXPIRATION_DAYS: i64 = 7;

/// 生成 JWT Token
pub fn generate_token(user_id: &str, email: &str) -> Result<String, String> {
    let now = Utc::now();
    let exp = now + Duration::days(TOKEN_EXPIRATION_DAYS);

    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        exp: exp.timestamp() as usize,
        iat: now.timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_ref()),
    )
    .map_err(|e| format!("生成 token 失败: {}", e))
}

/// 验证 JWT Token
pub fn verify_token(token: &str) -> Result<Claims, String> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET.as_ref()),
        &Validation::new(Algorithm::HS256),
    )
    .map(|data| data.claims)
    .map_err(|e| format!("Token 验证失败: {}", e))
}
