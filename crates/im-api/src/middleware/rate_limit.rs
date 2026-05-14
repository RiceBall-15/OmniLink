use axum::{
    extract::ConnectInfo,
    http::{HeaderValue, StatusCode},
    response::IntoResponse,
    Extension,
    Json,
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};

/// 速率限制配置
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    /// 时间窗口内的最大请求数
    pub max_requests: u32,
    /// 时间窗口持续时间
    pub window_duration: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window_duration: Duration::from_secs(60),
        }
    }
}

/// IP 请求记录
#[derive(Clone, Debug)]
struct RequestRecord {
    count: u32,
    window_start: Instant,
}

/// 速率限制检查结果
struct RateLimitResult {
    allowed: bool,
    limit: u32,
    remaining: u32,
    reset_at: u64, // Unix timestamp (seconds)
}

/// 速率限制状态（共享、线程安全，支持热更新配置）
#[derive(Clone)]
pub struct RateLimitState {
    records: Arc<Mutex<HashMap<String, RequestRecord>>>,
    config: Arc<RwLock<RateLimitConfig>>,
}

impl RateLimitState {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            records: Arc::new(Mutex::new(HashMap::new())),
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// 获取当前配置的快照
    async fn get_config(&self) -> RateLimitConfig {
        self.config.read().await.clone()
    }

    /// 热更新限流配置
    pub async fn update_config(&self, new_config: RateLimitConfig) {
        let mut config = self.config.write().await;
        info!(
            old_max = config.max_requests,
            old_window_secs = config.window_duration.as_secs(),
            new_max = new_config.max_requests,
            new_window_secs = new_config.window_duration.as_secs(),
            "Rate limit config updated (hot-reload)"
        );
        *config = new_config;
    }

    /// 获取当前配置（用于 API 查询）
    pub async fn get_config_snapshot(&self) -> RateLimitConfig {
        self.config.read().await.clone()
    }

    /// 检查请求是否允许（返回详细结果，含 headers 信息）
    async fn check_rate_limit_detailed(&self, ip: &str) -> RateLimitResult {
        let config = self.get_config().await;
        let mut records = self.records.lock().await;
        let now = Instant::now();

        if let Some(record) = records.get_mut(ip) {
            let elapsed = now.duration_since(record.window_start);
            if elapsed >= config.window_duration {
                // 窗口已过期，重置
                record.count = 1;
                record.window_start = now;
                let reset_at = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
                    + config.window_duration.as_secs();
                RateLimitResult {
                    allowed: true,
                    limit: config.max_requests,
                    remaining: config.max_requests.saturating_sub(1),
                    reset_at,
                }
            } else if record.count >= config.max_requests {
                // 超出限制
                let retry_after = config.window_duration - elapsed;
                let reset_at = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
                    + retry_after.as_secs();
                RateLimitResult {
                    allowed: false,
                    limit: config.max_requests,
                    remaining: 0,
                    reset_at,
                }
            } else {
                // 未超出限制
                record.count += 1;
                let remaining = config.max_requests.saturating_sub(record.count);
                let reset_at = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
                    + (config.window_duration - elapsed).as_secs();
                RateLimitResult {
                    allowed: true,
                    limit: config.max_requests,
                    remaining,
                    reset_at,
                }
            }
        } else {
            // 新 IP
            records.insert(
                ip.to_string(),
                RequestRecord {
                    count: 1,
                    window_start: now,
                },
            );
            let reset_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                + config.window_duration.as_secs();
            RateLimitResult {
                allowed: true,
                limit: config.max_requests,
                remaining: config.max_requests.saturating_sub(1),
                reset_at,
            }
        }
    }

    /// 清理过期记录（定期调用以避免内存泄漏）
    pub async fn cleanup_expired(&self) {
        let config = self.get_config().await;
        let mut records = self.records.lock().await;
        let now = Instant::now();
        records.retain(|_, record| {
            now.duration_since(record.window_start) < config.window_duration * 2
        });
    }
}

/// 速率限制错误
#[derive(Debug)]
enum RateLimitError {
    TooManyRequests { retry_after: Duration },
}

/// 速率限制中间件（带标准 X-RateLimit-* 响应头）
pub async fn rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    axum::extract::State(state): axum::extract::State<RateLimitState>,
    request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Result<axum::http::Response<axum::body::Body>, StatusCode> {
    let ip = addr.ip().to_string();

    let result = state.check_rate_limit_detailed(&ip).await;

    if result.allowed {
        // 允许请求，在响应中添加速率限制头
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
        // 超出限制，返回 429 并带标准头
        warn!(
            ip = %ip,
            limit = result.limit,
            reset_at = result.reset_at,
            "Rate limit exceeded"
        );

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
pub struct UpdateRateLimitRequest {
    pub max_requests: Option<u32>,
    pub window_duration_secs: Option<u64>,
}

/// 获取当前限流配置
pub async fn get_rate_limit_config(
    Extension(state): Extension<RateLimitState>,
) -> impl IntoResponse {
    let config = state.get_config_snapshot().await;
    Json(serde_json::json!({
        "max_requests": config.max_requests,
        "window_duration_secs": config.window_duration.as_secs(),
    }))
}

/// 更新限流配置（热更新，无需重启）
pub async fn update_rate_limit_config(
    Extension(state): Extension<RateLimitState>,
    Json(req): Json<UpdateRateLimitRequest>,
) -> impl IntoResponse {
    let current = state.get_config_snapshot().await;
    let new_config = RateLimitConfig {
        max_requests: req.max_requests.unwrap_or(current.max_requests),
        window_duration: req
            .window_duration_secs
            .map(Duration::from_secs)
            .unwrap_or(current.window_duration),
    };

    // 基本验证
    if new_config.max_requests == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "max_requests must be greater than 0"})),
        );
    }
    if new_config.window_duration.is_zero() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "window_duration must be greater than 0"})),
        );
    }

    state.update_config(new_config).await;

    (
        StatusCode::OK,
        Json(serde_json::json!({"message": "Rate limit config updated successfully"})),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limit_within_window() {
        let config = RateLimitConfig {
            max_requests: 3,
            window_duration: Duration::from_secs(60),
        };
        let state = RateLimitState::new(config);
        let ip = "192.168.1.1";

        // 前 3 个请求应该成功
        let r1 = state.check_rate_limit_detailed(ip).await;
        assert!(r1.allowed);
        assert_eq!(r1.remaining, 2);

        let r2 = state.check_rate_limit_detailed(ip).await;
        assert!(r2.allowed);
        assert_eq!(r2.remaining, 1);

        let r3 = state.check_rate_limit_detailed(ip).await;
        assert!(r3.allowed);
        assert_eq!(r3.remaining, 0);

        // 第 4 个请求应该被限制
        let r4 = state.check_rate_limit_detailed(ip).await;
        assert!(!r4.allowed);
        assert_eq!(r4.remaining, 0);
    }

    #[tokio::test]
    async fn test_rate_limit_different_ips() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_duration: Duration::from_secs(60),
        };
        let state = RateLimitState::new(config);

        // 不同 IP 应该独立计数
        assert!(state.check_rate_limit_detailed("192.168.1.1").await.allowed);
        assert!(state.check_rate_limit_detailed("192.168.1.2").await.allowed);
        assert!(state.check_rate_limit_detailed("192.168.1.1").await.allowed);
        assert!(state.check_rate_limit_detailed("192.168.1.2").await.allowed);

        // 两个 IP 都应该被限制
        assert!(!state.check_rate_limit_detailed("192.168.1.1").await.allowed);
        assert!(!state.check_rate_limit_detailed("192.168.1.2").await.allowed);
    }

    #[tokio::test]
    async fn test_rate_limit_window_reset() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_duration: Duration::from_millis(100),
        };
        let state = RateLimitState::new(config);
        let ip = "192.168.1.1";

        // 用完配额
        state.check_rate_limit_detailed(ip).await;
        state.check_rate_limit_detailed(ip).await;
        assert!(!state.check_rate_limit_detailed(ip).await.allowed);

        // 等待窗口过期
        tokio::time::sleep(Duration::from_millis(150)).await;

        // 应该可以再次请求
        assert!(state.check_rate_limit_detailed(ip).await.allowed);
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_duration: Duration::from_millis(100),
        };
        let state = RateLimitState::new(config);
        let ip = "192.168.1.1";

        state.check_rate_limit_detailed(ip).await;

        // 等待过期
        tokio::time::sleep(Duration::from_millis(250)).await;

        // 清理
        state.cleanup_expired().await;

        // 记录应该被清理
        let records = state.records.lock().await;
        assert!(records.is_empty());
    }

    #[tokio::test]
    async fn test_hot_reload_config() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_duration: Duration::from_secs(60),
        };
        let state = RateLimitState::new(config);
        let ip = "192.168.1.1";

        // 用完初始配额
        state.check_rate_limit_detailed(ip).await;
        state.check_rate_limit_detailed(ip).await;
        assert!(!state.check_rate_limit_detailed(ip).await.allowed);

        // 热更新配置，增加限额
        let new_config = RateLimitConfig {
            max_requests: 5,
            window_duration: Duration::from_secs(60),
        };
        state.update_config(new_config).await;

        // 旧窗口中 count=2，新限制是 5，应该可以继续请求
        assert!(state.check_rate_limit_detailed(ip).await.allowed);
        assert!(state.check_rate_limit_detailed(ip).await.allowed);
        assert!(state.check_rate_limit_detailed(ip).await.allowed);
        // 现在 count=5，应该被限制
        assert!(!state.check_rate_limit_detailed(ip).await.allowed);
    }

    #[tokio::test]
    async fn test_rate_limit_headers_info() {
        let config = RateLimitConfig {
            max_requests: 5,
            window_duration: Duration::from_secs(60),
        };
        let state = RateLimitState::new(config);
        let ip = "10.0.0.1";

        let result = state.check_rate_limit_detailed(ip).await;
        assert!(result.allowed);
        assert_eq!(result.limit, 5);
        assert_eq!(result.remaining, 4);
        assert!(result.reset_at > 0);
    }
}
