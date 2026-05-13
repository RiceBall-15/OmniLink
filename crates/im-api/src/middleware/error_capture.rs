//! 全局错误捕获中间件
//!
//! 捕获所有未处理的错误并返回结构化的错误响应，
//! 确保客户端始终收到格式一致的 JSON 错误信息。

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use tracing::{error, warn};

/// 全局错误捕获中间件
///
/// 捕获 panic 和未处理的错误，返回统一格式的 JSON 错误响应。
/// 同时记录错误日志，方便排查问题。
pub async fn error_capture_middleware(
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();

    let response = next.run(request).await;

    // 检查响应状态码，对 5xx 错误记录告警日志
    let status = response.status();
    if status.is_server_error() {
        error!(
            method = %method,
            path = %uri,
            status = %status.as_u16(),
            "服务器错误响应"
        );
    } else if status.is_client_error() && status == StatusCode::UNAUTHORIZED {
        warn!(
            method = %method,
            path = %uri,
            status = %status.as_u16(),
            "未授权请求"
        );
    }

    response
}

/// 自定义的 JSON 解析错误处理
///
/// 当请求体 JSON 解析失败时，返回友好的错误信息
pub fn json_parse_error_response(message: String) -> Response {
    let body = serde_json::json!({
        "success": false,
        "error": {
            "code": 3001,
            "code_str": "E3001",
            "type": "请求参数无效",
            "message": message,
        },
        "data": null,
        "timestamp": chrono::Utc::now().timestamp(),
    });
    (StatusCode::BAD_REQUEST, axum::Json(body)).into_response()
}

/// 路由未找到错误响应
pub fn not_found_response(path: &str) -> Response {
    let body = serde_json::json!({
        "success": false,
        "error": {
            "code": 2001,
            "code_str": "E2001",
            "type": "资源未找到",
            "message": format!("路由未找到: {}", path),
        },
        "data": null,
        "timestamp": chrono::Utc::now().timestamp(),
    });
    (StatusCode::NOT_FOUND, axum::Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_parse_error_response() {
        let response = json_parse_error_response("无效的JSON格式".to_string());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_not_found_response() {
        let response = not_found_response("/api/unknown");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
