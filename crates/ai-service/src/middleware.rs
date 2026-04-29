use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use common::{AppError, Claims};
use uuid::Uuid;

/// 认证上下文
#[derive(Debug, Clone)]
pub struct Auth(pub Claims);

/// 认证中间件
pub async fn auth_middleware(
    State(token_manager): State<common::auth::TokenManager>,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    // 获取Authorization头
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Missing authorization header".to_string()))?;

    // 解析Bearer token
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Invalid authorization format".to_string()))?;

    // 验证token
    let claims = token_manager
        .verify_token(token)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid token".to_string()))?;

    // 将claims添加到请求扩展中
    request.extensions_mut().insert(Auth(claims));

    Ok(next.run(request).await)
}