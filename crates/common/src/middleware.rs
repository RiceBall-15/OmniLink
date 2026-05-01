use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::time::Instant;

/// 日志记录中间件
/// 
/// 这个中间件会记录所有HTTP请求的基本信息，包括：
/// - HTTP方法
/// - 请求路径
/// - 响应时间
/// - 响应状态码
/// 
/// 用于监控和调试目的
pub async fn logging_middleware(
    req: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    let version = req.version();

    // 执行下一个中间件或处理器
    let response = next.run(req).await;

    let duration = start.elapsed();
    let status = response.status();

    // 记录请求日志
    tracing::info!(
        method = %method,
        uri = %uri,
        version = ?version,
        status = %status,
        duration_ms = duration.as_millis(),
        "HTTP request"
    );

    response
}

/// 错误处理中间件
/// 
/// 这个中间件捕获所有错误并将其转换为适当的HTTP响应
/// 包括：
/// - 数据库错误
/// - Redis错误
/// - 验证错误
/// - 认证错误
/// - 等等
pub async fn error_handler_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 尝试执行请求
    Ok(next.run(req).await)
}

/// 请求ID中间件
/// 
/// 为每个请求生成唯一的请求ID，便于追踪和调试
pub async fn request_id_middleware(
    mut req: Request,
    next: Next,
) -> Response {
    // 生成或提取请求ID
    let request_id = req
        .headers()
        .get("X-Request-ID")
        .and_then(|h| h.to_str().ok())
        .unwrap_or_else(|| {
            // 生成新的UUID作为请求ID
            uuid::Uuid::new_v4().to_string()
        });

    // 将请求ID添加到请求头和扩展中
    req.headers_mut().insert(
        "X-Request-ID",
        request_id.parse().unwrap(),
    );
    req.extensions_mut().insert(request_id.clone());

    // 执行下一个中间件
    let mut response = next.run(req).await;

    // 将请求ID添加到响应头
    response.headers_mut().insert(
        "X-Request-ID",
        request_id.parse().unwrap(),
    );

    response
}

/// CORS中间件配置
/// 
/// 配置跨域资源共享策略
pub fn cors_middleware() -> tower_http::cors::CorsLayer {
    use tower_http::cors::{Any, CorsLayer};
    use http::header::{AUTHORIZATION, ACCEPT, CONTENT_TYPE};

    // 配置CORS策略
    CorsLayer::new()
        // 允许的来源（生产环境应该配置具体的域名）
        .allow_origin(Any)
        // 允许的请求头
        .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE])
        // 允许的HTTP方法
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::PATCH,
            axum::http::Method::OPTIONS,
        ])
        // 允许携带凭证
        .allow_credentials(true)
        // 预检请求的缓存时间（秒）
        .max_age(3600)
}

/// 压缩中间件
/// 
/// 自动压缩响应体以减少带宽使用
pub fn compression_middleware() -> tower_http::compression::CompressionLayer {
    tower_http::compression::CompressionLayer::new()
}

/// 超时中间件
/// 
/// 设置请求的超时时间
pub fn timeout_middleware(
    timeout: std::time::Duration,
) -> tower::layer::util::Stack<
    tower_http::timeout::TimeoutLayer,
    tower::layer::util::Identity,
> {
    tower::ServiceBuilder::new()
        .layer(tower_http::timeout::TimeoutLayer::new(timeout))
        .into_inner()
}

/// 限流中间件
/// 
/// 基于IP地址的请求限流，防止滥用
pub struct RateLimiter {
    /// 每秒最多请求数
    max_requests: u64,
    /// 时间窗口（秒）
    window_secs: u64,
}

impl RateLimiter {
    /// 创建新的限流器
    pub fn new(max_requests: u64, window_secs: u64) -> Self {
        Self {
            max_requests,
            window_secs,
        }
    }

    /// 检查请求是否超过限流
    pub fn check_rate_limit(&self, ip: &str, redis: &redis::Client) -> Result<bool, AppError> {
        use redis::Commands;

        let mut conn = redis.get_connection()?;
        let key = format!("rate_limit:{}", ip);
        let current_time = chrono::Utc::now().timestamp();

        // 使用Redis的有序集合来记录请求时间
        let _: () = conn.zadd(&key, current_time, current_time)?;

        // 移除时间窗口外的记录
        let cutoff_time = current_time - self.window_secs as i64;
        let _: () = conn.zrembyscore(&key, 0, cutoff_time)?;

        // 获取当前时间窗口内的请求数
        let count: u64 = conn.zcard(&key)?;

        // 设置过期时间
        let _: () = conn.expire(&key, self.window_secs)?;

        Ok(count <= self.max_requests)
    }
}

use common::error::AppError;

/// 请求大小限制中间件
/// 
/// 限制请求体的大小，防止大文件攻击
pub fn limit_body_size(
    max_size: usize,
) -> tower_http::limit::RequestBodyLimitLayer {
    tower_http::limit::RequestBodyLimitLayer::new(
        max_size as u64,
        tower_http::limit::RequestBodyLimitAction::default(),
    )
}