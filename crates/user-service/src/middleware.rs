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
pub struct AuthContext {
    pub user_id: Uuid,
    pub device_id: String,
}

/// 认证中间件
pub async fn auth_middleware(
    State(token_manager): State<std::sync::Arc<common::auth::TokenManager>>,
    mut req: Request,
    next: Next,
) -> Result<Response, Response> {
    // 从Header获取Token
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body("Missing authorization header".to_string())
                .unwrap()
        })?;

    // 验证Token格式
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| {
            Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body("Invalid authorization header format".to_string())
                .unwrap()
        })?;

    // 验证Token
    let claims = token_manager
        .verify_token(token)
        .map_err(|e| {
            Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(e.to_string())
                .unwrap()
        })?;

    // 将用户信息添加到请求扩展
    let auth_context = AuthContext {
        user_id: claims.sub,
        device_id: claims.device_id,
    };
    req.extensions_mut().insert(auth_context);

    Ok(next.run(req).await)
}