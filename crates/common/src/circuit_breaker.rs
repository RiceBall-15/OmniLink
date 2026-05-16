//! 断路器模式实现
//!
//! 提供微服务间的弹性通信保护：
//! - Closed → 正常状态，允许请求通过
//! - Open → 熔断状态，拒绝所有请求
//! - HalfOpen → 半开状态，允许有限请求探测恢复
//!
//! # 使用示例
//!
//! ```rust
//! use common::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
//!
//! let config = CircuitBreakerConfig {
//!     failure_threshold: 5,
//!     recovery_timeout: std::time::Duration::from_secs(30),
//!     half_open_max_attempts: 3,
//!     ..Default::default()
//! };
//!
//! let cb = CircuitBreaker::new("user-service".to_string(), config);
//!
//! // 执行受保护的操作
//! let result = cb.execute(|| async {
//!     // 调用远程服务
//!     Ok::<_, String>("response".to_string())
//! }).await;
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// 断路器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    /// 正常状态 — 允许请求通过
    Closed,
    /// 熔断状态 — 拒绝所有请求
    Open,
    /// 半开状态 — 允许有限请求探测恢复
    HalfOpen,
}

impl fmt::Display for CircuitState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "Closed"),
            CircuitState::Open => write!(f, "Open"),
            CircuitState::HalfOpen => write!(f, "HalfOpen"),
        }
    }
}

/// 断路器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// 连续失败多少次后触发熔断
    pub failure_threshold: u32,
    /// 熔断后等待多久进入半开状态
    pub recovery_timeout: Duration,
    /// 半开状态下允许的最大探测请求数
    pub half_open_max_attempts: u32,
    /// 半开状态下探测成功多少次后恢复到 Closed
    pub half_open_success_threshold: u32,
    /// 请求超时时间（可选，超时算作失败）
    pub request_timeout: Option<Duration>,
    /// 滑动窗口大小（记录最近N次请求结果）
    pub window_size: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
            half_open_max_attempts: 3,
            half_open_success_threshold: 2,
            request_timeout: None,
            window_size: 10,
        }
    }
}

/// 断路器统计数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitStats {
    /// 服务名称
    pub service_name: String,
    /// 当前状态
    pub state: CircuitState,
    /// 总请求数
    pub total_requests: u64,
    /// 成功请求数
    pub total_successes: u64,
    /// 失败请求数
    pub total_failures: u64,
    /// 连续失败次数
    pub consecutive_failures: u32,
    /// 连续成功次数
    pub consecutive_successes: u32,
    /// 当前半开状态的探测尝试次数
    pub half_open_attempts: u32,
    /// 最后状态变更时间
    pub last_state_change: Option<chrono::DateTime<chrono::Utc>>,
    /// 最后失败时间
    pub last_failure: Option<chrono::DateTime<chrono::Utc>>,
    /// 失败率（百分比）
    pub failure_rate: f64,
}

/// 断路器内部状态
struct CircuitBreakerInner {
    state: CircuitState,
    /// 连续失败计数
    consecutive_failures: u32,
    /// 连续成功计数
    consecutive_successes: u32,
    /// 半开状态的探测尝试次数
    half_open_attempts: u32,
    /// 最后状态变更时间
    last_state_change: Instant,
    /// 最后失败时间
    last_failure_time: Option<Instant>,
    /// 最近请求结果滑动窗口（true=成功，false=失败）
    request_window: Vec<bool>,
    /// 统计计数器
    total_requests: u64,
    total_successes: u64,
    total_failures: u64,
}

/// 断路器
///
/// 线程安全的断路器实现，可在多个异步任务间共享。
#[derive(Clone)]
pub struct CircuitBreaker {
    inner: Arc<RwLock<CircuitBreakerInner>>,
    config: CircuitBreakerConfig,
    service_name: String,
    state_changes: Arc<AtomicU64>,
}

impl CircuitBreaker {
    /// 创建新的断路器实例
    pub fn new(service_name: String, config: CircuitBreakerConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(CircuitBreakerInner {
                state: CircuitState::Closed,
                consecutive_failures: 0,
                consecutive_successes: 0,
                half_open_attempts: 0,
                last_state_change: Instant::now(),
                last_failure_time: None,
                request_window: Vec::with_capacity(config.window_size as usize),
                total_requests: 0,
                total_successes: 0,
                total_failures: 0,
            })),
            config,
            service_name,
            state_changes: Arc::new(AtomicU64::new(0)),
        }
    }

    /// 获取当前断路器状态
    pub async fn state(&self) -> CircuitState {
        let inner = self.inner.read().await;
        // 检查是否应该从 Open 转换到 HalfOpen
        if inner.state == CircuitState::Open {
            if let Some(last_failure) = inner.last_failure_time {
                if last_failure.elapsed() >= self.config.recovery_timeout {
                    drop(inner);
                    self.transition_to(CircuitState::HalfOpen).await;
                    return CircuitState::HalfOpen;
                }
            }
        }
        inner.state
    }

    /// 获取服务名称
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// 执行受断路器保护的异步操作
    ///
    /// 如果断路器处于 Open 状态，立即返回错误。
    /// 如果处于 Closed 或 HalfOpen 状态，执行操作并根据结果更新状态。
    pub async fn execute<F, Fut, T, E>(&self, operation: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        // 检查是否允许请求通过
        match self.check_request_allowed().await {
            Ok(()) => {},
            Err(CircuitBreakerError::Rejected) => return Err(CircuitBreakerError::Rejected),
            Err(CircuitBreakerError::Timeout) => return Err(CircuitBreakerError::Timeout),
            Err(CircuitBreakerError::Operation(_)) => unreachable!(),
        }

        let start = Instant::now();
        let result = operation().await;

        match &result {
            Ok(_) => {
                self.record_success().await;
            }
            Err(_) => {
                self.record_failure().await;
            }
        }

        // 检查请求超时
        if let Some(timeout) = self.config.request_timeout {
            if start.elapsed() > timeout {
                self.record_failure().await;
                return Err(CircuitBreakerError::Timeout);
            }
        }

        result.map_err(CircuitBreakerError::Operation)
    }

    /// 执行受保护的操作（简化版，错误类型为 String）
    pub async fn execute_simple<F, Fut, T>(&self, operation: F) -> Result<T, String>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, String>>,
    {
        match self.execute(operation).await {
            Ok(v) => Ok(v),
            Err(CircuitBreakerError::Operation(e)) => Err(e),
            Err(CircuitBreakerError::Rejected) => {
                Err(format!("断路器已熔断，服务 {} 暂时不可用", self.service_name))
            }
            Err(CircuitBreakerError::Timeout) => {
                Err(format!("服务 {} 请求超时", self.service_name))
            }
        }
    }

    /// 检查请求是否被允许通过
    async fn check_request_allowed(&self) -> Result<(), CircuitBreakerError<()>> {
        let mut inner = self.inner.write().await;

        match inner.state {
            CircuitState::Closed => Ok(()),
            CircuitState::Open => {
                // 检查是否应该转换到 HalfOpen
                if let Some(last_failure) = inner.last_failure_time {
                    if last_failure.elapsed() >= self.config.recovery_timeout {
                        inner.state = CircuitState::HalfOpen;
                        inner.half_open_attempts = 0;
                        inner.consecutive_successes = 0;
                        inner.last_state_change = Instant::now();
                        self.state_changes.fetch_add(1, Ordering::Relaxed);
                        // 允许这个请求通过作为探测
                        return Ok(());
                    }
                }
                Err(CircuitBreakerError::Rejected)
            }
            CircuitState::HalfOpen => {
                if inner.half_open_attempts < self.config.half_open_max_attempts {
                    inner.half_open_attempts += 1;
                    Ok(())
                } else {
                    // 半开状态下达到最大探测数，拒绝
                    Err(CircuitBreakerError::Rejected)
                }
            }
        }
    }

    /// 记录请求成功
    async fn record_success(&self) {
        let mut inner = self.inner.write().await;

        inner.total_requests += 1;
        inner.total_successes += 1;
        inner.consecutive_failures = 0;
        inner.consecutive_successes += 1;

        // 更新滑动窗口
        Self::push_window(&mut inner.request_window, true, self.config.window_size as usize);

        match inner.state {
            CircuitState::Closed => {
                // 正常状态，不做额外操作
            }
            CircuitState::HalfOpen => {
                // 半开状态下成功，检查是否恢复到 Closed
                if inner.consecutive_successes >= self.config.half_open_success_threshold {
                    inner.state = CircuitState::Closed;
                    inner.consecutive_failures = 0;
                    inner.half_open_attempts = 0;
                    inner.last_state_change = Instant::now();
                    self.state_changes.fetch_add(1, Ordering::Relaxed);
                }
            }
            CircuitState::Open => {
                // 不应该在这里，但安全起见
            }
        }
    }

    /// 记录请求失败
    async fn record_failure(&self) {
        let mut inner = self.inner.write().await;

        inner.total_requests += 1;
        inner.total_failures += 1;
        inner.consecutive_successes = 0;
        inner.consecutive_failures += 1;
        inner.last_failure_time = Some(Instant::now());

        // 更新滑动窗口
        Self::push_window(&mut inner.request_window, false, self.config.window_size as usize);

        match inner.state {
            CircuitState::Closed => {
                // 检查是否触发熔断
                if inner.consecutive_failures >= self.config.failure_threshold {
                    inner.state = CircuitState::Open;
                    inner.last_state_change = Instant::now();
                    inner.half_open_attempts = 0;
                    self.state_changes.fetch_add(1, Ordering::Relaxed);
                }
            }
            CircuitState::HalfOpen => {
                // 半开状态下失败，立即回到 Open
                inner.state = CircuitState::Open;
                inner.last_state_change = Instant::now();
                inner.half_open_attempts = 0;
                self.state_changes.fetch_add(1, Ordering::Relaxed);
            }
            CircuitState::Open => {
                // 已经是 Open，更新最后失败时间
            }
        }
    }

    /// 向滑动窗口推入结果
    fn push_window(window: &mut Vec<bool>, result: bool, max_size: usize) {
        if window.len() >= max_size {
            window.remove(0);
        }
        window.push(result);
    }

    /// 获取断路器统计信息
    pub async fn stats(&self) -> CircuitStats {
        let inner = self.inner.read().await;

        let failure_rate = if inner.total_requests > 0 {
            (inner.total_failures as f64 / inner.total_requests as f64) * 100.0
        } else {
            0.0
        };

        // 计算滑动窗口失败率
        let window_failure_rate = if !inner.request_window.is_empty() {
            let failures = inner.request_window.iter().filter(|&&r| !r).count();
            (failures as f64 / inner.request_window.len() as f64) * 100.0
        } else {
            failure_rate
        };

        CircuitStats {
            service_name: self.service_name.clone(),
            state: inner.state,
            total_requests: inner.total_requests,
            total_successes: inner.total_successes,
            total_failures: inner.total_failures,
            consecutive_failures: inner.consecutive_failures,
            consecutive_successes: inner.consecutive_successes,
            half_open_attempts: inner.half_open_attempts,
            last_state_change: Some(chrono::Utc::now()
                - chrono::Duration::from_std(inner.last_state_change.elapsed())
                    .unwrap_or_default()),
            last_failure: inner.last_failure_time.map(|t| {
                chrono::Utc::now()
                    - chrono::Duration::from_std(t.elapsed()).unwrap_or_default()
            }),
            failure_rate: window_failure_rate,
        }
    }

    /// 强制重置断路器到 Closed 状态（管理员操作）
    pub async fn reset(&self) {
        let mut inner = self.inner.write().await;
        inner.state = CircuitState::Closed;
        inner.consecutive_failures = 0;
        inner.consecutive_successes = 0;
        inner.half_open_attempts = 0;
        inner.request_window.clear();
        inner.last_state_change = Instant::now();
        self.state_changes.fetch_add(1, Ordering::Relaxed);
    }

    /// 强制打开断路器（管理员操作）
    pub async fn force_open(&self) {
        let mut inner = self.inner.write().await;
        inner.state = CircuitState::Open;
        inner.last_failure_time = Some(Instant::now());
        inner.last_state_change = Instant::now();
        self.state_changes.fetch_add(1, Ordering::Relaxed);
    }

    /// 手动转换状态
    async fn transition_to(&self, new_state: CircuitState) {
        let mut inner = self.inner.write().await;
        inner.state = new_state;
        inner.last_state_change = Instant::now();
        self.state_changes.fetch_add(1, Ordering::Relaxed);
    }

    /// 获取状态变更次数
    pub fn state_change_count(&self) -> u64 {
        self.state_changes.load(Ordering::Relaxed)
    }
}

/// 断路器错误类型
#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    /// 断路器已熔断，请求被拒绝
    Rejected,
    /// 请求超时
    Timeout,
    /// 操作本身的错误
    Operation(E),
}

impl<E: fmt::Display> fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CircuitBreakerError::Rejected => write!(f, "断路器已熔断，请求被拒绝"),
            CircuitBreakerError::Timeout => write!(f, "请求超时"),
            CircuitBreakerError::Operation(e) => write!(f, "操作失败: {}", e),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> std::error::Error for CircuitBreakerError<E> {}

/// 断路器管理器
///
/// 管理多个服务的断路器实例。
#[derive(Clone)]
pub struct CircuitBreakerManager {
    breakers: Arc<RwLock<std::collections::HashMap<String, CircuitBreaker>>>,
    default_config: CircuitBreakerConfig,
}

impl CircuitBreakerManager {
    /// 创建新的断路器管理器
    pub fn new(default_config: CircuitBreakerConfig) -> Self {
        Self {
            breakers: Arc::new(RwLock::new(std::collections::HashMap::new())),
            default_config,
        }
    }

    /// 获取或创建指定服务的断路器
    pub async fn get_or_create(&self, service_name: &str) -> CircuitBreaker {
        let breakers = self.breakers.read().await;
        if let Some(cb) = breakers.get(service_name) {
            return cb.clone();
        }
        drop(breakers);

        let mut breakers = self.breakers.write().await;
        // 双重检查
        if let Some(cb) = breakers.get(service_name) {
            return cb.clone();
        }

        let cb = CircuitBreaker::new(
            service_name.to_string(),
            self.default_config.clone(),
        );
        breakers.insert(service_name.to_string(), cb.clone());
        cb
    }

    /// 获取所有断路器的统计信息
    pub async fn all_stats(&self) -> Vec<CircuitStats> {
        let breakers = self.breakers.read().await;
        let mut stats = Vec::with_capacity(breakers.len());
        for cb in breakers.values() {
            stats.push(cb.stats().await);
        }
        stats
    }

    /// 获取所有断路器的名称和状态
    pub async fn summary(&self) -> Vec<(String, CircuitState)> {
        let breakers = self.breakers.read().await;
        let mut summary = Vec::with_capacity(breakers.len());
        for (name, cb) in breakers.iter() {
            summary.push((name.clone(), cb.state().await));
        }
        summary
    }

    /// 重置所有断路器
    pub async fn reset_all(&self) {
        let breakers = self.breakers.read().await;
        for cb in breakers.values() {
            cb.reset().await;
        }
    }
}

impl Default for CircuitBreakerManager {
    fn default() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    fn test_config() -> CircuitBreakerConfig {
        CircuitBreakerConfig {
            failure_threshold: 3,
            recovery_timeout: Duration::from_millis(100),
            half_open_max_attempts: 2,
            half_open_success_threshold: 2,
            request_timeout: None,
            window_size: 10,
        }
    }

    #[tokio::test]
    async fn test_initial_state_is_closed() {
        let cb = CircuitBreaker::new("test".to_string(), test_config());
        assert_eq!(cb.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_stays_closed_on_success() {
        let cb = CircuitBreaker::new("test".to_string(), test_config());

        let result = cb.execute(|| async { Ok::<_, String>("ok") }).await;
        assert!(result.is_ok());
        assert_eq!(cb.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_transitions_to_open_after_threshold() {
        let cb = CircuitBreaker::new("test".to_string(), test_config());

        // Record failures up to threshold
        for _ in 0..3 {
            let _: Result<(), _> = cb
                .execute(|| async { Err::<(), _>("fail".to_string()) })
                .await
                .map_err(|e| match e {
                    CircuitBreakerError::Operation(s) => s,
                    _ => "rejected".to_string(),
                });
        }

        assert_eq!(cb.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_open_rejects_requests() {
        let cb = CircuitBreaker::new("test".to_string(), test_config());

        // Trip the circuit breaker
        for _ in 0..3 {
            let _: Result<(), _> = cb
                .execute(|| async { Err::<(), _>("fail".to_string()) })
                .await;
        }

        // Next request should be rejected
        let result = cb.execute(|| async { Ok::<_, String>("ok") }).await;
        assert!(matches!(result, Err(CircuitBreakerError::Rejected)));
    }

    #[tokio::test]
    async fn test_transitions_to_half_open_after_timeout() {
        let cb = CircuitBreaker::new("test".to_string(), test_config());

        // Trip the circuit breaker
        for _ in 0..3 {
            let _: Result<(), _> = cb
                .execute(|| async { Err::<(), _>("fail".to_string()) })
                .await;
        }
        assert_eq!(cb.state().await, CircuitState::Open);

        // Wait for recovery timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // State should transition to HalfOpen on next check
        assert_eq!(cb.state().await, CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_half_open_recovers_on_success() {
        let cb = CircuitBreaker::new("test".to_string(), test_config());

        // Trip the circuit breaker
        for _ in 0..3 {
            let _: Result<(), _> = cb
                .execute(|| async { Err::<(), _>("fail".to_string()) })
                .await;
        }

        // Wait for recovery
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Successful requests in half-open state
        for _ in 0..2 {
            let result = cb.execute(|| async { Ok::<_, String>("ok") }).await;
            assert!(result.is_ok());
        }

        // Should be back to Closed
        assert_eq!(cb.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_half_open_fails_back_to_open() {
        let cb = CircuitBreaker::new("test".to_string(), test_config());

        // Trip the circuit breaker
        for _ in 0..3 {
            let _: Result<(), _> = cb
                .execute(|| async { Err::<(), _>("fail".to_string()) })
                .await;
        }

        // Wait for recovery
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Fail in half-open state
        let _: Result<(), _> = cb
            .execute(|| async { Err::<(), _>("fail again".to_string()) })
            .await;

        // Should be back to Open
        assert_eq!(cb.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_stats_tracking() {
        let cb = CircuitBreaker::new("test".to_string(), test_config());

        // Some successes
        for _ in 0..5 {
            let _ = cb.execute(|| async { Ok::<_, String>("ok") }).await;
        }

        // Some failures
        for _ in 0..2 {
            let _: Result<(), _> = cb
                .execute(|| async { Err::<(), _>("fail".to_string()) })
                .await;
        }

        let stats = cb.stats().await;
        assert_eq!(stats.total_requests, 7);
        assert_eq!(stats.total_successes, 5);
        assert_eq!(stats.total_failures, 2);
        assert_eq!(stats.consecutive_failures, 2);
        assert!(stats.failure_rate > 28.0 && stats.failure_rate < 29.0);
    }

    #[tokio::test]
    async fn test_reset() {
        let cb = CircuitBreaker::new("test".to_string(), test_config());

        // Trip the circuit breaker
        for _ in 0..3 {
            let _: Result<(), _> = cb
                .execute(|| async { Err::<(), _>("fail".to_string()) })
                .await;
        }
        assert_eq!(cb.state().await, CircuitState::Open);

        // Reset
        cb.reset().await;
        assert_eq!(cb.state().await, CircuitState::Closed);

        let stats = cb.stats().await;
        assert_eq!(stats.consecutive_failures, 0);
    }

    #[tokio::test]
    async fn test_force_open() {
        let cb = CircuitBreaker::new("test".to_string(), test_config());
        assert_eq!(cb.state().await, CircuitState::Closed);

        cb.force_open().await;
        assert_eq!(cb.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_manager_get_or_create() {
        let manager = CircuitBreakerManager::new(test_config());

        let cb1 = manager.get_or_create("service-a").await;
        let cb2 = manager.get_or_create("service-a").await;

        // Should return the same instance
        assert_eq!(cb1.service_name(), cb2.service_name());

        let cb3 = manager.get_or_create("service-b").await;
        assert_ne!(cb1.service_name(), cb3.service_name());
    }

    #[tokio::test]
    async fn test_manager_all_stats() {
        let manager = CircuitBreakerManager::new(test_config());

        manager.get_or_create("svc-1").await;
        manager.get_or_create("svc-2").await;

        let stats = manager.all_stats().await;
        assert_eq!(stats.len(), 2);
    }

    #[tokio::test]
    async fn test_execute_simple() {
        let cb = CircuitBreaker::new("test".to_string(), test_config());

        let result = cb
            .execute_simple(|| async { Ok::<_, String>("hello".to_string()) })
            .await;
        assert_eq!(result.unwrap(), "hello");

        let result = cb
            .execute_simple(|| async { Err::<String, _>("oops".to_string()) })
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let cb = CircuitBreaker::new("test".to_string(), test_config());
        let counter = Arc::new(AtomicU32::new(0));

        let mut handles = Vec::new();
        for _ in 0..10 {
            let cb_clone = cb.clone();
            let counter_clone = counter.clone();
            handles.push(tokio::spawn(async move {
                let _ = cb_clone
                    .execute(|| async {
                        counter_clone.fetch_add(1, Ordering::Relaxed);
                        Ok::<_, String>("ok")
                    })
                    .await;
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(counter.load(Ordering::Relaxed), 10);
        let stats = cb.stats().await;
        assert_eq!(stats.total_requests, 10);
        assert_eq!(stats.total_successes, 10);
    }

    #[tokio::test]
    async fn test_state_display() {
        assert_eq!(format!("{}", CircuitState::Closed), "Closed");
        assert_eq!(format!("{}", CircuitState::Open), "Open");
        assert_eq!(format!("{}", CircuitState::HalfOpen), "HalfOpen");
    }

    #[tokio::test]
    async fn test_error_display() {
        let err: CircuitBreakerError<String> = CircuitBreakerError::Rejected;
        assert_eq!(format!("{}", err), "断路器已熔断，请求被拒绝");

        let err: CircuitBreakerError<String> = CircuitBreakerError::Timeout;
        assert_eq!(format!("{}", err), "请求超时");

        let err: CircuitBreakerError<String> = CircuitBreakerError::Operation("boom".to_string());
        assert_eq!(format!("{}", err), "操作失败: boom");
    }
}
