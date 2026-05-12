use axum::{
    extract::ConnectInfo,
    http::StatusCode,
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use tracing::warn;

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

/// 速率限制状态（共享、线程安全）
#[derive(Clone)]
pub struct RateLimitState {
    records: Arc<Mutex<HashMap<String, RequestRecord>>>,
    config: RateLimitConfig,
}

impl RateLimitState {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            records: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// 检查请求是否允许
    async fn check_rate_limit(&self, ip: &str) -> Result<(), RateLimitError> {
        let mut records = self.records.lock().await;
        let now = Instant::now();

        if let Some(record) = records.get_mut(ip) {
            // 检查时间窗口是否已过期
            if now.duration_since(record.window_start) >= self.config.window_duration {
                // 重置窗口
                record.count = 1;
                record.window_start = now;
                Ok(())
            } else if record.count >= self.config.max_requests {
                // 超出限制
                Err(RateLimitError::TooManyRequests {
                    retry_after: self.config.window_duration
                        - now.duration_since(record.window_start),
                })
            } else {
                // 增加计数
                record.count += 1;
                Ok(())
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
            Ok(())
        }
    }

    /// 清理过期记录（定期调用以避免内存泄漏）
    pub async fn cleanup_expired(&self) {
        let mut records = self.records.lock().await;
        let now = Instant::now();
        records.retain(|_, record| {
            now.duration_since(record.window_start) < self.config.window_duration * 2
        });
    }
}

/// 速率限制错误
#[derive(Debug)]
enum RateLimitError {
    TooManyRequests { retry_after: Duration },
}

/// 速率限制中间件
pub async fn rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    axum::extract::State(state): axum::extract::State<RateLimitState>,
    request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Result<axum::http::Response<axum::body::Body>, StatusCode> {
    let ip = addr.ip().to_string();

    match state.check_rate_limit(&ip).await {
        Ok(()) => {
            // 允许请求
            let response = next.run(request).await;
            Ok(response)
        }
        Err(RateLimitError::TooManyRequests { retry_after }) => {
            // 超出限制
            warn!(
                ip = %ip,
                retry_after_secs = retry_after.as_secs(),
                "Rate limit exceeded"
            );
            Err(StatusCode::TOO_MANY_REQUESTS)
        }
    }
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
        assert!(state.check_rate_limit(ip).await.is_ok());
        assert!(state.check_rate_limit(ip).await.is_ok());
        assert!(state.check_rate_limit(ip).await.is_ok());

        // 第 4 个请求应该被限制
        assert!(state.check_rate_limit(ip).await.is_err());
    }

    #[tokio::test]
    async fn test_rate_limit_different_ips() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_duration: Duration::from_secs(60),
        };
        let state = RateLimitState::new(config);

        // 不同 IP 应该独立计数
        assert!(state.check_rate_limit("192.168.1.1").await.is_ok());
        assert!(state.check_rate_limit("192.168.1.2").await.is_ok());
        assert!(state.check_rate_limit("192.168.1.1").await.is_ok());
        assert!(state.check_rate_limit("192.168.1.2").await.is_ok());

        // 两个 IP 都应该被限制
        assert!(state.check_rate_limit("192.168.1.1").await.is_err());
        assert!(state.check_rate_limit("192.168.1.2").await.is_err());
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
        assert!(state.check_rate_limit(ip).await.is_ok());
        assert!(state.check_rate_limit(ip).await.is_ok());
        assert!(state.check_rate_limit(ip).await.is_err());

        // 等待窗口过期
        tokio::time::sleep(Duration::from_millis(150)).await;

        // 应该可以再次请求
        assert!(state.check_rate_limit(ip).await.is_ok());
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_duration: Duration::from_millis(100),
        };
        let state = RateLimitState::new(config);
        let ip = "192.168.1.1";

        state.check_rate_limit(ip).await.unwrap();

        // 等待过期
        tokio::time::sleep(Duration::from_millis(250)).await;

        // 清理
        state.cleanup_expired().await;

        // 记录应该被清理
        let records = state.records.lock().await;
        assert!(records.is_empty());
    }
}
