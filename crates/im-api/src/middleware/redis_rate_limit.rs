//! Redis 分布式限流中间件
//!
//! 使用 Redis 滑动窗口计数器实现分布式速率限制。
//! 支持多实例部署，所有实例共享同一 Redis 计数器。

use axum::{
    extract::ConnectInfo,
    http::{HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use redis::{aio::ConnectionManager, AsyncCommands};
use std::{net::SocketAddr, sync::Arc, time::{SystemTime, UNIX_EPOCH}};
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Redis 限流配置
#[derive(Clone, Debug)]
pub struct RedisRateLimitConfig {
    /// 时间窗口内的最大请求数
    pub max_requests: u32,
    /// 时间窗口持续时间（秒）
    pub window_secs: u64,
    /// IP 白名单（不受限流影响）
    pub whitelist_ips: Vec<String>,
    /// 认证用户的限流（可选，比 IP 限流更宽松）
    pub authenticated_max_requests: Option<u32>,
    /// Redis key 前缀
    pub key_prefix: String,
}

impl Default for RedisRateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window_secs: 60,
            whitelist_ips: vec!["127.0.0.1".to_string(), "::1".to_string()],
            authenticated_max_requests: None,
            key_prefix: "omnilink:ratelimit".to_string(),
        }
    }
}

/// 限流结果
#[derive(Debug)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub limit: u32,
    pub remaining: u32,
    pub reset_at: u64,
}

/// Redis 分布式限流状态
#[derive(Clone)]
pub struct RedisRateLimitState {
    redis: ConnectionManager,
    config: Arc<RwLock<RedisRateLimitConfig>>,
}

impl RedisRateLimitState {
    pub async fn new(redis: ConnectionManager, config: RedisRateLimitConfig) -> Self {
        Self {
            redis,
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// 热更新限流配置
    pub async fn update_config(&self, new_config: RedisRateLimitConfig) {
        let mut config = self.config.write().await;
        info!(
            old_max = config.max_requests,
            new_max = new_config.max_requests,
            "Redis rate limit config updated"
        );
        *config = new_config;
    }

    /// 获取当前配置快照
    pub async fn get_config_snapshot(&self) -> RedisRateLimitConfig {
        self.config.read().await.clone()
    }

    /// 使用 Redis 滑动窗口计数器检查限流
    ///
    /// 算法：使用 Redis 有序集合，score 为时间戳。
    /// 1. 添加当前请求时间戳
    /// 2. 移除窗口外的旧记录
    /// 3. 统计窗口内请求数
    pub async fn check_rate_limit(&self, ip: &str) -> RateLimitResult {
        let config = self.config.read().await.clone();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let key = format!("{}:{}", config.key_prefix, ip);
        let window_start = now.saturating_sub(config.window_secs);

        let mut redis = self.redis.clone();

        // 使用 Redis pipeline 减少网络往返
        let result: ((), (), u64, ()) = redis::pipe()
            .cmd("ZADD")
            .arg(&key)
            .arg(now)
            .arg(now)
            .ignore()
            .cmd("ZREMRANGEBYSCORE")
            .arg(&key)
            .arg(0)
            .arg(window_start)
            .ignore()
            .cmd("ZCARD")
            .arg(&key)
            .cmd("EXPIRE")
            .arg(&key)
            .arg(config.window_secs + 10) // 额外 10 秒缓冲
            .query_async(&mut redis)
            .await
            .unwrap_or(((), (), 0, ()));

        let count = result.2;
        let limit = config.max_requests;
        let remaining = limit.saturating_sub(count as u32);
        let reset_at = now + config.window_secs;

        if count <= limit as u64 {
            RateLimitResult {
                allowed: true,
                limit,
                remaining,
                reset_at,
            }
        } else {
            warn!(ip = %ip, count = count, limit = limit, "Rate limit exceeded (Redis)");
            RateLimitResult {
                allowed: false,
                limit,
                remaining: 0,
                reset_at,
            }
        }
    }

    /// 获取限流状态（不增加计数器）
    pub async fn get_rate_limit_status(&self, ip: &str) -> RateLimitResult {
        let config = self.config.read().await.clone();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let key = format!("{}:{}", config.key_prefix, ip);
        let window_start = now.saturating_sub(config.window_secs);

        let mut redis = self.redis.clone();

        // 只查询，不增加计数
        // 先清理过期条目
        let _: Result<(), _> = redis::cmd("ZREMRANGEBYSCORE")
            .arg(&key)
            .arg(0)
            .arg(window_start)
            .query_async(&mut redis)
            .await;

        // 查询当前窗口内的请求数
        let count_val: redis::Value = redis::pipe()
            .cmd("ZCARD")
            .arg(&key)
            .query_async(&mut redis)
            .await
            .unwrap_or(redis::Value::Int(0));

        let count = match count_val {
            redis::Value::Int(n) => n as u64,
            _ => 0,
        };

        let limit = config.max_requests;
        let remaining = limit.saturating_sub(count as u32);
        let reset_at = now + config.window_secs;

        RateLimitResult {
            allowed: count < limit as u64,
            limit,
            remaining,
            reset_at,
        }
    }

    /// 清除指定 IP 的限流记录
    pub async fn clear_rate_limit(&self, ip: &str) {
        let config = self.config.read().await.clone();
        let key = format!("{}:{}", config.key_prefix, ip);
        let mut redis = self.redis.clone();
        let _: Result<(), _> = redis.del(&key).await;
        info!(ip = %ip, "Rate limit cleared");
    }
}

/// Redis 限流中间件处理函数
pub async fn redis_rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    axum::Extension(state): axum::Extension<RedisRateLimitState>,
    request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Result<axum::http::Response<axum::body::Body>, StatusCode> {
    let ip = addr.ip().to_string();

    // 检查白名单
    let config = state.get_config_snapshot().await;
    if config.whitelist_ips.contains(&ip) {
        return Ok(next.run(request).await);
    }

    let result = state.check_rate_limit(&ip).await;

    if result.allowed {
        let mut response = next.run(request).await;
        let headers = response.headers_mut();

        if let Ok(val) = HeaderValue::from_str(&result.limit.to_string()) {
            headers.insert("X-RateLimit-Limit", val);
        }
        if let Ok(val) = HeaderValue::from_str(&result.remaining.to_string()) {
            headers.insert("X-RateLimit-Remaining", val);
        }
        if let Ok(val) = HeaderValue::from_str(&result.reset_at.to_string()) {
            headers.insert("X-RateLimit-Reset", val);
        }

        Ok(response)
    } else {
        let retry_after_secs = result.reset_at.saturating_sub(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );

        let mut response = StatusCode::TOO_MANY_REQUESTS.into_response();
        let headers = response.headers_mut();

        if let Ok(val) = HeaderValue::from_str(&result.limit.to_string()) {
            headers.insert("X-RateLimit-Limit", val);
        }
        headers.insert("X-RateLimit-Remaining", HeaderValue::from_static("0"));
        if let Ok(val) = HeaderValue::from_str(&result.reset_at.to_string()) {
            headers.insert("X-RateLimit-Reset", val);
        }
        if let Ok(val) = HeaderValue::from_str(&retry_after_secs.to_string()) {
            headers.insert("Retry-After", val);
        }

        Ok(response)
    }
}

/// 限流配置更新请求
#[derive(serde::Deserialize)]
pub struct UpdateRedisRateLimitRequest {
    pub max_requests: Option<u32>,
    pub window_secs: Option<u64>,
    pub whitelist_ips: Option<Vec<String>>,
    pub authenticated_max_requests: Option<u32>,
}

/// 获取当前 Redis 限流配置
pub async fn get_redis_rate_limit_config(
    axum::Extension(state): axum::Extension<RedisRateLimitState>,
) -> impl IntoResponse {
    let config = state.get_config_snapshot().await;
    Json(serde_json::json!({
        "backend": "redis",
        "max_requests": config.max_requests,
        "window_secs": config.window_secs,
        "whitelist_ips": config.whitelist_ips,
        "authenticated_max_requests": config.authenticated_max_requests,
        "key_prefix": config.key_prefix,
    }))
}

/// 更新 Redis 限流配置
pub async fn update_redis_rate_limit_config(
    axum::Extension(state): axum::Extension<RedisRateLimitState>,
    axum::extract::Json(req): axum::extract::Json<UpdateRedisRateLimitRequest>,
) -> impl IntoResponse {
    let current = state.get_config_snapshot().await;
    let new_config = RedisRateLimitConfig {
        max_requests: req.max_requests.unwrap_or(current.max_requests),
        window_secs: req.window_secs.unwrap_or(current.window_secs),
        whitelist_ips: req.whitelist_ips.unwrap_or(current.whitelist_ips),
        authenticated_max_requests: req
            .authenticated_max_requests
            .or(current.authenticated_max_requests),
        key_prefix: current.key_prefix,
    };

    if new_config.max_requests == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "max_requests must be greater than 0"})),
        );
    }
    if new_config.window_secs == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "window_secs must be greater than 0"})),
        );
    }

    state.update_config(new_config).await;

    (
        StatusCode::OK,
        Json(serde_json::json!({"message": "Redis rate limit config updated"})),
    )
}

/// 查询指定 IP 的限流状态
pub async fn get_ip_rate_limit_status(
    axum::Extension(state): axum::Extension<RedisRateLimitState>,
    axum::extract::Path(ip): axum::extract::Path<String>,
) -> impl IntoResponse {
    let result = state.get_rate_limit_status(&ip).await;
    Json(serde_json::json!({
        "ip": ip,
        "limit": result.limit,
        "remaining": result.remaining,
        "reset_at": result.reset_at,
        "allowed": result.allowed,
    }))
}

/// 清除指定 IP 的限流记录（管理员接口）
pub async fn clear_ip_rate_limit(
    axum::Extension(state): axum::Extension<RedisRateLimitState>,
    axum::extract::Path(ip): axum::extract::Path<String>,
) -> impl IntoResponse {
    state.clear_rate_limit(&ip).await;
    Json(serde_json::json!({
        "message": format!("Rate limit cleared for IP: {}", ip),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_rate_limit_config_default() {
        let config = RedisRateLimitConfig::default();
        assert_eq!(config.max_requests, 100);
        assert_eq!(config.window_secs, 60);
        assert!(config.whitelist_ips.contains(&"127.0.0.1".to_string()));
        assert_eq!(config.key_prefix, "omnilink:ratelimit");
    }

    #[test]
    fn test_rate_limit_result_properties() {
        let result = RateLimitResult {
            allowed: true,
            limit: 100,
            remaining: 99,
            reset_at: 1000,
        };
        assert!(result.allowed);
        assert_eq!(result.remaining, 99);
    }
}
