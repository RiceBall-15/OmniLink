use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode, header::AUTHORIZATION},
};

use crate::utils::jwt::verify_token;

/// 用户 ID 扩展（从 JWT token 中提取）
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub email: String,
}

/// 从请求头中提取并验证 JWT token
///
/// 手动解析 Authorization: Bearer <token> 头，避免依赖 TypedHeader
/// 使用 axum re-export 的 async_trait 以兼容 axum-core 0.4
#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 从 Authorization header 中提取 token
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "缺少认证 token"))?;

        // 解析 Bearer token
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or((StatusCode::UNAUTHORIZED, "无效的认证格式"))?;

        // 验证 token
        let claims = verify_token(token)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "无效的 token"))?;

        // 检查 token 是否过期
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        if claims.exp < now {
            return Err((StatusCode::UNAUTHORIZED, "Token 已过期"));
        }

        Ok(AuthUser {
            user_id: claims.sub,
            email: claims.email,
        })
    }
}

/// 路由层面获取用户 ID 的扩展（用于 handler）
pub async fn extract_user_id(
    auth: AuthUser,
) -> Result<String, (StatusCode, String)> {
    Ok(auth.user_id)
}
