use axum::{
    extract::{FromRequestParts, Request, State},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::Response,
};
use common::auth::{Claims, TokenManager};
use std::sync::Arc;

/// 认证中间件
pub async fn auth_middleware(
    State(token_manager): State<Arc<TokenManager>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 从请求头获取token
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "));

    if let Some(token) = auth_header {
        match token_manager.verify_token(token) {
            Ok(claims) => {
                // 将claims插入请求扩展中
                request.extensions_mut().insert(claims);
                Ok(next.run(request).await)
            }
            Err(_) => Err(StatusCode::UNAUTHORIZED),
        }
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// 认证用户包装器，可直接作为 handler 参数提取
/// 从请求扩展中获取由 auth_middleware 注入的 Claims
#[derive(Debug, Clone)]
pub struct AuthUser(pub Claims);

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Claims>()
            .cloned()
            .map(AuthUser)
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}
