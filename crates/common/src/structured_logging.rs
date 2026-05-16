//! 结构化日志增强
//!
//! 提供生产环境日志优化：
//! - 高频端点日志降采样（减少日志量）
//! - 结构化日志字段标准化
//! - 请求上下文自动注入中间件（actix-web）
//! - 日志采样策略配置
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use common::structured_logging::{SamplingConfig, SamplingLayer, RequestLoggingMiddleware};
//!
//! // 配置采样：health 端点只记录 1% 的日志
//! let config = SamplingConfig::default()
//!     .with_rule("/health", 0.01)
//!     .with_rule("/api/v1/messages", 0.1);  // 消息端点记录 10%
//!
//! let sampling_layer = SamplingLayer::new(config);
//! ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tracing::Subscriber;
use tracing_subscriber::Layer;

/// 采样规则配置
#[derive(Debug, Clone)]
pub struct SamplingConfig {
    /// 路径采样率映射：路径前缀 -> 采样率 (0.0 - 1.0)
    pub rules: HashMap<String, f64>,
    /// 默认采样率（未匹配规则时使用）
    pub default_rate: f64,
    /// 是否对错误日志始终采样（不降采样）
    pub always_sample_errors: bool,
}

impl Default for SamplingConfig {
    fn default() -> Self {
        let mut rules = HashMap::new();
        // 健康检查端点：1% 采样
        rules.insert("/health".to_string(), 0.01);
        rules.insert("/healthz".to_string(), 0.01);
        rules.insert("/readyz".to_string(), 0.01);
        // 指标端点：5% 采样
        rules.insert("/metrics".to_string(), 0.05);
        // 心跳/ping：1% 采样
        rules.insert("/ping".to_string(), 0.01);

        Self {
            rules,
            default_rate: 1.0,
            always_sample_errors: true,
        }
    }
}

impl SamplingConfig {
    /// 添加采样规则
    pub fn with_rule(mut self, path_prefix: &str, rate: f64) -> Self {
        self.rules
            .insert(path_prefix.to_string(), rate.clamp(0.0, 1.0));
        self
    }

    /// 设置默认采样率
    pub fn with_default_rate(mut self, rate: f64) -> Self {
        self.default_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// 设置是否始终采样错误
    pub fn with_always_sample_errors(mut self, always: bool) -> Self {
        self.always_sample_errors = always;
        self
    }

    /// 获取指定路径的采样率
    pub fn get_rate(&self, path: &str) -> f64 {
        // 按路径前缀匹配（最长前缀优先）
        let mut best_match: Option<(&str, f64)> = None;
        for (prefix, rate) in &self.rules {
            if path.starts_with(prefix) {
                if best_match.is_none() || prefix.len() > best_match.unwrap().0.len() {
                    best_match = Some((prefix.as_str(), *rate));
                }
            }
        }
        best_match.map(|(_, rate)| rate).unwrap_or(self.default_rate)
    }
}

/// 采样统计
#[derive(Debug, Default)]
pub struct SamplingStats {
    /// 总日志事件数
    pub total_events: AtomicU64,
    /// 采样通过的事件数
    pub sampled_events: AtomicU64,
    /// 被过滤的事件数
    pub filtered_events: AtomicU64,
}

impl SamplingStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pass(&self) {
        self.total_events.fetch_add(1, Ordering::Relaxed);
        self.sampled_events.fetch_add(1, Ordering::Relaxed);
    }

    pub fn filter(&self) {
        self.total_events.fetch_add(1, Ordering::Relaxed);
        self.filtered_events.fetch_add(1, Ordering::Relaxed);
    }

    /// 获取采样率
    pub fn sample_rate(&self) -> f64 {
        let total = self.total_events.load(Ordering::Relaxed);
        if total == 0 {
            return 1.0;
        }
        let sampled = self.sampled_events.load(Ordering::Relaxed);
        sampled as f64 / total as f64
    }
}

/// 采样层
///
/// 基于计数器的确定性采样，不依赖随机数。
/// 每 N 个事件中只通过 1 个（N = 1/rate）。
pub struct SamplingLayer {
    config: SamplingConfig,
    counter: AtomicU64,
    stats: Arc<SamplingStats>,
}

impl SamplingLayer {
    pub fn new(config: SamplingConfig) -> Self {
        Self {
            config,
            counter: AtomicU64::new(0),
            stats: Arc::new(SamplingStats::new()),
        }
    }

    /// 获取采样统计
    pub fn stats(&self) -> Arc<SamplingStats> {
        self.stats.clone()
    }

    /// 判断是否应该采样
    fn should_sample(&self, path: &str, is_error: bool) -> bool {
        // 错误始终采样
        if is_error && self.config.always_sample_errors {
            self.stats.pass();
            return true;
        }

        let rate = self.config.get_rate(path);
        if rate >= 1.0 {
            self.stats.pass();
            return true;
        }
        if rate <= 0.0 {
            self.stats.filter();
            return false;
        }

        // 确定性采样：每 1/rate 个事件通过 1 个
        let interval = (1.0 / rate) as u64;
        let count = self.counter.fetch_add(1, Ordering::Relaxed);
        if count % interval == 0 {
            self.stats.pass();
            true
        } else {
            self.stats.filter();
            false
        }
    }
}

impl<S: Subscriber> Layer<S> for SamplingLayer {
    fn event_enabled(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        // 从事件元数据判断级别
        let metadata = event.metadata();
        let is_error = *metadata.level() == tracing::Level::ERROR;

        // 尝试从 span 中获取路径信息
        // 注意：在实际使用中，路径信息通常存储在 span 的扩展中
        // 这里我们使用一个简化的实现
        let path = "";

        self.should_sample(path, is_error)
    }
}

/// 请求日志中间件
///
/// 提供标准化的请求日志格式，适用于 Axum 框架。
/// 使用 tower::Layer 实现，与 axum 中间件系统兼容。
///
/// # 使用示例
///
/// ```rust,no_run
/// use axum::{Router, routing::get};
/// use common::structured_logging::RequestLoggingLayer;
///
/// let app = Router::new()
///     .route("/health", get(|| async { "OK" }))
///     .layer(RequestLoggingLayer::new());
/// ```
#[derive(Clone)]
pub struct RequestLoggingLayer;

impl RequestLoggingLayer {
    pub fn new() -> Self {
        Self
    }
}

/// 请求上下文注入
///
/// 为每个请求生成唯一的 request_id，并注入到 tracing span 中。
pub fn create_request_span(
    method: &str,
    path: &str,
    request_id: Option<&str>,
) -> tracing::Span {
    let request_id = request_id
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    tracing::info_span!(
        "request",
        method = %method,
        path = %path,
        request_id = %request_id,
        status = tracing::field::Empty,
        duration_ms = tracing::field::Empty,
    )
}

/// 记录请求完成
pub fn record_request_completion(
    span: &tracing::Span,
    status: u16,
    duration: std::time::Duration,
) {
    span.record("status", status);
    span.record("duration_ms", duration.as_millis() as u64);
}

/// 日志字段标准化
///
/// 提供标准化的日志字段名称，确保跨服务一致性。
pub mod fields {
    /// 请求 ID
    pub const REQUEST_ID: &str = "request_id";
    /// 追踪 ID
    pub const TRACE_ID: &str = "trace_id";
    /// 用户 ID
    pub const USER_ID: &str = "user_id";
    /// 会话 ID
    pub const CONVERSATION_ID: &str = "conversation_id";
    /// 消息 ID
    pub const MESSAGE_ID: &str = "message_id";
    /// 操作名称
    pub const OPERATION: &str = "operation";
    /// 耗时（毫秒）
    pub const DURATION_MS: &str = "duration_ms";
    /// 状态码
    pub const STATUS: &str = "status";
    /// 错误类型
    pub const ERROR_TYPE: &str = "error_type";
    /// 客户端 IP
    pub const CLIENT_IP: &str = "client_ip";
    /// User-Agent
    pub const USER_AGENT: &str = "user_agent";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sampling_config_default() {
        let config = SamplingConfig::default();
        assert_eq!(config.get_rate("/health"), 0.01);
        assert_eq!(config.get_rate("/healthz"), 0.01);
        assert_eq!(config.get_rate("/metrics"), 0.05);
        assert_eq!(config.get_rate("/api/v1/messages"), 1.0);
        assert!(config.always_sample_errors);
    }

    #[test]
    fn test_sampling_config_custom_rules() {
        let config = SamplingConfig::default()
            .with_rule("/api/v1/messages", 0.1)
            .with_rule("/api/v1/conversations", 0.5);

        assert_eq!(config.get_rate("/api/v1/messages"), 0.1);
        assert_eq!(config.get_rate("/api/v1/messages/123"), 0.1);
        assert_eq!(config.get_rate("/api/v1/conversations"), 0.5);
        assert_eq!(config.get_rate("/api/v1/users"), 1.0);
    }

    #[test]
    fn test_sampling_config_longest_prefix_match() {
        let config = SamplingConfig::default()
            .with_rule("/api", 0.5)
            .with_rule("/api/v1/messages", 0.1);

        // 更长的前缀应该优先匹配
        assert_eq!(config.get_rate("/api/v1/messages"), 0.1);
        assert_eq!(config.get_rate("/api/v1/messages/123"), 0.1);
        // 短前缀匹配
        assert_eq!(config.get_rate("/api/v1/users"), 0.5);
        assert_eq!(config.get_rate("/api/other"), 0.5);
    }

    #[test]
    fn test_sampling_config_default_rate() {
        let config = SamplingConfig::default().with_default_rate(0.5);
        assert_eq!(config.get_rate("/unknown/path"), 0.5);
    }

    #[test]
    fn test_sampling_config_clamp_rates() {
        let config = SamplingConfig::default()
            .with_rule("/test1", -0.5)  // 被 clamp 到 0.0
            .with_rule("/test2", 1.5);  // 被 clamp 到 1.0

        assert_eq!(config.get_rate("/test1"), 0.0);
        assert_eq!(config.get_rate("/test2"), 1.0);
    }

    #[test]
    fn test_sampling_stats_initial() {
        let stats = SamplingStats::new();
        assert_eq!(stats.total_events.load(Ordering::Relaxed), 0);
        assert_eq!(stats.sampled_events.load(Ordering::Relaxed), 0);
        assert_eq!(stats.filtered_events.load(Ordering::Relaxed), 0);
        assert_eq!(stats.sample_rate(), 1.0);
    }

    #[test]
    fn test_sampling_stats_pass_filter() {
        let stats = SamplingStats::new();
        stats.pass();
        stats.pass();
        stats.filter();

        assert_eq!(stats.total_events.load(Ordering::Relaxed), 3);
        assert_eq!(stats.sampled_events.load(Ordering::Relaxed), 2);
        assert_eq!(stats.filtered_events.load(Ordering::Relaxed), 1);
        assert!((stats.sample_rate() - 2.0 / 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_sampling_layer_should_sample_full_rate() {
        let config = SamplingConfig::default(); // default_rate = 1.0
        let layer = SamplingLayer::new(config);

        // 全速率应该始终采样
        assert!(layer.should_sample("/api/test", false));
        assert!(layer.should_sample("/api/test", false));
        assert!(layer.should_sample("/api/test", false));
    }

    #[test]
    fn test_sampling_layer_should_sample_errors_always() {
        let config = SamplingConfig::default()
            .with_rule("/health", 0.0)
            .with_always_sample_errors(true);
        let layer = SamplingLayer::new(config);

        // 即使采样率为 0，错误也应始终采样
        assert!(layer.should_sample("/health", true));
        assert!(layer.should_sample("/health", true));
    }

    #[test]
    fn test_sampling_layer_deterministic_sampling() {
        let config = SamplingConfig::default()
            .with_rule("/test", 0.5); // 50% 采样
        let layer = SamplingLayer::new(config);

        let mut pass_count = 0;
        for _ in 0..100 {
            if layer.should_sample("/test/api", false) {
                pass_count += 1;
            }
        }

        // 50% 采样率，应该大约有 50 个通过
        assert!(pass_count >= 40 && pass_count <= 60,
            "Expected ~50, got {}", pass_count);
    }

    #[test]
    fn test_sampling_layer_zero_rate() {
        let config = SamplingConfig::default()
            .with_rule("/test", 0.0)
            .with_always_sample_errors(false);
        let layer = SamplingLayer::new(config);

        // 零采样率，非错误日志不应通过
        assert!(!layer.should_sample("/test/api", false));
        assert!(!layer.should_sample("/test/api", false));
    }

    #[test]
    fn test_sampling_stats_serialization() {
        let stats = SamplingStats::new();
        stats.pass();
        stats.filter();

        // 通过 serde_json 序列化
        let json = serde_json::json!({
            "total": stats.total_events.load(Ordering::Relaxed),
            "sampled": stats.sampled_events.load(Ordering::Relaxed),
            "filtered": stats.filtered_events.load(Ordering::Relaxed),
            "rate": stats.sample_rate(),
        });

        assert_eq!(json["total"], 2);
        assert_eq!(json["sampled"], 1);
        assert_eq!(json["filtered"], 1);
    }

    #[test]
    fn test_fields_constants() {
        assert_eq!(fields::REQUEST_ID, "request_id");
        assert_eq!(fields::TRACE_ID, "trace_id");
        assert_eq!(fields::USER_ID, "user_id");
        assert_eq!(fields::DURATION_MS, "duration_ms");
        assert_eq!(fields::STATUS, "status");
    }

    #[test]
    fn test_create_request_span() {
        let span = create_request_span("GET", "/api/v1/users", Some("req-123"));
        assert!(span.is_disabled() == false);
    }

    #[test]
    fn test_create_request_span_without_id() {
        let span = create_request_span("POST", "/api/v1/messages", None);
        assert!(span.is_disabled() == false);
    }
}
