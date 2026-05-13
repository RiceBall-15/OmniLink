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
use crate::handlers::metrics::{inc_error_count, inc_request_count};
use serde_json::json;

/// 全局错误捕获中间件
///
/// 捕获所有请求，记录请求计数，并将任何未处理的错误转换为标准 JSON 响应。
pub async fn error_capture_middleware(
    request: Request,
    next: Next,
) -> Response {
    // 递增请求计数
    inc_request_count();

    let response = next.run(request).await;

    // 检查响应状态码，如果是错误则递增错误计数
    let status = response.status();
    if status.is_server_error() || status.is_client_error() {
        inc_error_count();
    }

    // 如果响应已经是 JSON 格式或不是 500 错误，直接返回
    if status != StatusCode::INTERNAL_SERVER_ERROR {
        return response;
    }

    // 对于 500 错误，检查是否已经是结构化响应
    // 如果不是，包装为标准错误格式
    let content_type = response.headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.contains("application/json") {
        // 已经是 JSON 响应，直接返回
        return response;
    }

    // 将非 JSON 的 500 错误转换为标准格式
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&json!({
            "success": false,
            "error": {
                "code": 500,
                "code_str": "E5001",
                "type": "internal",
                "message": "内部服务器错误"
            }
        })).unwrap_or_default(),
    ).into_response()
}

/// JSON 解析错误处理
///
/// 当请求体 JSON 解析失败时返回标准错误响应
pub async fn json_parse_error_response(message: &str) -> Response {
    inc_error_count();
    (
        StatusCode::BAD_REQUEST,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&json!({
            "success": false,
            "error": {
                "code": 400,
                "code_str": "E3001",
                "type": "validation",
                "message": message
            }
        })).unwrap_or_default(),
    ).into_response()
}

/// 路由未找到处理
///
/// 当请求的路由不存在时返回标准错误响应
pub async fn not_found_response() -> Response {
    inc_error_count();
    (
        StatusCode::NOT_FOUND,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&json!({
            "success": false,
            "error": {
                "code": 404,
                "code_str": "E3002",
                "type": "validation",
                "message": "请求的资源不存在"
            }
        })).unwrap_or_default(),
    ).into_response()
}
