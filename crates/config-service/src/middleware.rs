use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use common::auth::{TokenManager, Claims};

pub async fn auth_middleware(
    State(token_manager): State<Arc<TokenManager>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(header) => {
            if header.starts_with("Bearer ") {
                Some(&header[7..])
            } else {
                None
            }
        }
        None => None,
    };

    let token = match token {
        Some(t) => t,
        None => {
            tracing::warn!("Missing authorization token");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    let claims = match TokenManager::verify_token(token_manager.secret(), token) {
        Ok(claims) => claims,
        Err(e) => {
            tracing::warn!("Invalid token: {:?}", e);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}

impl TokenManager {
    fn secret(&self) -> &[u8] {
        self.secret.as_bytes()
    }
}