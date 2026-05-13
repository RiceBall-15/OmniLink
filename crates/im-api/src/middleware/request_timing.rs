use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::info;

/// 请求耗时中间件
/// 记录每个HTTP请求的方法、路径、状态码和耗时
pub async fn request_timing_middleware(
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();

    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status();

    info!(
        method = %method,
        path = %uri,
        status = %status.as_u16(),
        duration_ms = %duration.as_millis(),
        "HTTP请求完成"
    );

    response
}
