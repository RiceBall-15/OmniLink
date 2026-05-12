use axum::{
    extract::Request,
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use tracing::Span;
use uuid::Uuid;

/// Request ID 中间件
///
/// 功能：
/// 1. 从请求 header 中提取 X-Request-ID，如果没有则生成新的 UUID
/// 2. 将 request_id 注入到 tracing span 中
/// 3. 在响应 header 中返回 X-Request-ID
pub async fn request_id_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 从请求 header 提取或生成 request_id
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // 将 request_id 注入到当前 tracing span
    Span::current().record("request_id", &request_id.as_str());

    // 将 request_id 添加到请求 extensions 中，方便 handler 使用
    request
        .extensions_mut()
        .insert(RequestId(request_id.clone()));

    // 执行后续处理
    let mut response = next.run(request).await;

    // 在响应 header 中添加 X-Request-ID
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response
            .headers_mut()
            .insert("x-request-id", header_value);
    }

    Ok(response)
}

/// Request ID 提取器
///
/// 可以在 handler 中使用此提取器获取当前请求的 ID
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for RequestId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_display() {
        let id = RequestId("test-123".to_string());
        assert_eq!(format!("{}", id), "test-123");
    }

    #[test]
    fn test_request_id_as_ref() {
        let id = RequestId("test-456".to_string());
        assert_eq!(id.as_ref(), "test-456");
    }

    #[test]
    fn test_request_id_clone() {
        let id = RequestId("test-789".to_string());
        let cloned = id.clone();
        assert_eq!(id.0, cloned.0);
    }
}
