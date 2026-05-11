use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::jwt::JwtManager;

/// 认证上下文
///
/// 包含从 JWT Token 中提取的用户信息
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Uuid,
}

/// 认证中间件
///
/// 验证请求头中的 JWT Token，并将用户信息添加到请求扩展中
pub async fn auth_middleware(
    State(jwt_manager): State<Arc<JwtManager>>,
    mut req: Request,
    next: Next,
) -> Result<Response, Response<Body>> {
    // 从 Header 获取 Token
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(Body::from("Missing authorization header"))
                .unwrap()
        })?;

    // 验证 Token 格式
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| {
            Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(Body::from("Invalid authorization header format"))
                .unwrap()
        })?;

    // 验证 Token
    let claims = jwt_manager
        .verify_token(token)
        .map_err(|e| {
            Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(Body::from(e.to_string()))
                .unwrap()
        })?;

    // 将用户信息添加到请求扩展
    let auth_context = AuthContext {
        user_id: claims.user_id,
    };
    req.extensions_mut().insert(auth_context);

    Ok(next.run(req).await)
}
