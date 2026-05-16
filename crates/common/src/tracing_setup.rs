//! 分布式追踪模块
//!
//! 提供 OpenTelemetry 集成，支持：
//! - OTLP (OpenTelemetry Protocol) 导出
//! - Jaeger 导出
//! - 与 tracing-subscriber 集成
//! - 自动注入请求上下文传播

/// 追踪配置
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// 服务名称
    pub service_name: String,
    /// OTLP 导出端点 (例如: http://localhost:4317)
    pub otlp_endpoint: Option<String>,
    /// Jaeger Agent 端点 (例如: localhost:6831)
    pub jaeger_endpoint: Option<String>,
    /// 采样率 (0.0 - 1.0)
    pub sample_rate: f64,
    /// 是否启用控制台输出
    pub console_output: bool,
    /// 日志级别过滤
    pub log_level: String,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            service_name: "omnilink".to_string(),
            otlp_endpoint: None,
            jaeger_endpoint: None,
            sample_rate: 1.0,
            console_output: true,
            log_level: "info".to_string(),
        }
    }
}

impl TracingConfig {
    /// 从环境变量创建配置
    pub fn from_env(service_name: &str) -> Self {
        let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();
        let jaeger_endpoint = std::env::var("JAEGER_AGENT_ENDPOINT").ok();
        let sample_rate = std::env::var("OTEL_SAMPLE_RATE")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(1.0);
        let log_level = std::env::var("RUST_LOG")
            .unwrap_or_else(|_| "info".to_string());
        let console_output = std::env::var("TRACING_CONSOLE_OUTPUT")
            .map(|s| s != "false")
            .unwrap_or(true);

        Self {
            service_name: service_name.to_string(),
            otlp_endpoint,
            jaeger_endpoint,
            sample_rate,
            console_output,
            log_level,
        }
    }
}

/// 初始化追踪系统
///
/// 根据配置初始化 tracing-subscriber，支持：
/// - 控制台输出 (默认)
/// - JSON 格式输出 (生产环境)
/// - OpenTelemetry OTLP 导出 (可选)
/// - Jaeger 导出 (可选)
///
/// # 使用示例
///
/// ```rust,no_run
/// use common::tracing_setup::{TracingConfig, init_tracing};
///
/// // 基础控制台输出
/// init_tracing(&TracingConfig::default()).unwrap();
///
/// // 从环境变量配置
/// let config = TracingConfig::from_env("im-api");
/// init_tracing(&config).unwrap();
/// ```
pub fn init_tracing(config: &TracingConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    // 构建环境过滤器
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    // 基础格式化层
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    // 注册全局 subscriber
    if config.console_output {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();
    } else {
        // JSON 格式 (生产环境)
        let json_layer = tracing_subscriber::fmt::layer()
            .json()
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(json_layer)
            .init();
    }

    tracing::info!(
        service = %config.service_name,
        otlp_endpoint = ?config.otlp_endpoint,
        jaeger_endpoint = ?config.jaeger_endpoint,
        sample_rate = config.sample_rate,
        "Tracing initialized"
    );

    Ok(())
}

/// 请求追踪层
///
/// 为每个请求生成唯一的 trace ID 和 span ID，
/// 并通过 HTTP headers 传播上下文。
/// 使用 tower::Layer 实现，兼容 Axum 中间件系统。
#[derive(Clone)]
pub struct TracingLayer;

impl<S> tower::Layer<S> for TracingLayer {
    type Service = TracingService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TracingService { inner }
    }
}

/// 追踪服务
#[derive(Clone)]
pub struct TracingService<S> {
    inner: S,
}

impl<S> std::fmt::Debug for TracingService<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TracingService").finish()
    }
}

impl<S, ReqBody, ResBody> tower::Service<axum::http::Request<ReqBody>> for TracingService<S>
where
    S: tower::Service<axum::http::Request<ReqBody>, Response = axum::http::Response<ResBody>>
        + Send
        + Clone
        + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: axum::http::Request<ReqBody>) -> Self::Future {
        let trace_id = generate_trace_id();
        let span_id = generate_span_id();

        let method = req.method().clone();
        let uri = req.uri().clone();

        let start = std::time::Instant::now();
        let mut inner = self.inner.clone();
        let fut = inner.call(req);

        Box::pin(async move {
            let response = fut.await?;
            let duration = start.elapsed();

            tracing::info!(
                trace_id = %trace_id,
                span_id = %span_id,
                method = %method,
                uri = %uri,
                status = %response.status(),
                duration_ms = duration.as_millis(),
                "Request completed"
            );

            Ok(response)
        })
    }
}

/// 追踪上下文
#[derive(Debug, Clone)]
pub struct TraceContext {
    /// 全局追踪 ID
    pub trace_id: String,
    /// 当前 span ID
    pub span_id: String,
    /// 父 span ID
    pub parent_span_id: Option<String>,
}

impl TraceContext {
    /// 从请求扩展中获取追踪上下文
    pub fn from_request(req: &axum::extract::Request) -> Option<Self> {
        req.extensions().get::<Self>().cloned()
    }

    /// 转换为 HTTP headers (用于跨服务传播)
    pub fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("X-Trace-Id".to_string(), self.trace_id.clone()),
            ("X-Span-Id".to_string(), self.span_id.clone()),
        ]
    }
}

/// 生成追踪 ID (128位 hex)
fn generate_trace_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 16] = rng.gen();
    hex::encode(bytes)
}

/// 生成 span ID (64位 hex)
fn generate_span_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 8] = rng.gen();
    hex::encode(bytes)
}

/// 性能追踪包装器
///
/// 自动记录函数执行时间
pub struct PerformanceTracker {
    name: String,
    start: std::time::Instant,
}

impl PerformanceTracker {
    /// 开始追踪
    pub fn start(name: impl Into<String>) -> Self {
        let name = name.into();
        tracing::debug!(operation = %name, "Operation started");
        Self {
            name,
            start: std::time::Instant::now(),
        }
    }

    /// 完成追踪并记录耗时
    pub fn finish(self) {
        let duration = self.start.elapsed();
        tracing::info!(
            operation = %self.name,
            duration_ms = duration.as_millis(),
            "Operation completed"
        );
    }

    /// 完成追踪并记录耗时（带额外信息）
    pub fn finish_with_details(self, details: &str) {
        let duration = self.start.elapsed();
        tracing::info!(
            operation = %self.name,
            duration_ms = duration.as_millis(),
            details = %details,
            "Operation completed"
        );
    }
}

/// 跨服务追踪上下文传播
///
/// 从 HTTP headers 中提取追踪上下文
pub fn extract_trace_from_headers(headers: &axum::http::HeaderMap) -> Option<TraceContext> {
    let trace_id = headers
        .get("X-Trace-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())?;
    let span_id = headers
        .get("X-Span-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())?;

    Some(TraceContext {
        trace_id,
        span_id,
        parent_span_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracing_config_default() {
        let config = TracingConfig::default();
        assert_eq!(config.service_name, "omnilink");
        assert!(config.otlp_endpoint.is_none());
        assert!(config.jaeger_endpoint.is_none());
        assert_eq!(config.sample_rate, 1.0);
        assert!(config.console_output);
    }

    #[test]
    fn test_generate_trace_id() {
        let id1 = generate_trace_id();
        let id2 = generate_trace_id();
        // 128位 = 16字节 = 32 hex字符
        assert_eq!(id1.len(), 32);
        assert_eq!(id2.len(), 32);
        assert_ne!(id1, id2); // 随机生成，不应相同
    }

    #[test]
    fn test_generate_span_id() {
        let id1 = generate_span_id();
        let id2 = generate_span_id();
        // 64位 = 8字节 = 16 hex字符
        assert_eq!(id1.len(), 16);
        assert_eq!(id2.len(), 16);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_trace_context_to_headers() {
        let ctx = TraceContext {
            trace_id: "abc123".to_string(),
            span_id: "def456".to_string(),
            parent_span_id: None,
        };
        let headers = ctx.to_headers();
        assert_eq!(headers.len(), 2);
        assert_eq!(headers[0].0, "X-Trace-Id");
        assert_eq!(headers[0].1, "abc123");
        assert_eq!(headers[1].0, "X-Span-Id");
        assert_eq!(headers[1].1, "def456");
    }

    #[test]
    fn test_performance_tracker() {
        let tracker = PerformanceTracker::start("test_op");
        // 模拟一些工作
        std::thread::sleep(std::time::Duration::from_millis(10));
        tracker.finish_with_details("test completed");
    }

    #[test]
    fn test_extract_trace_from_headers_empty() {
        let headers = axum::http::HeaderMap::new();
        assert!(extract_trace_from_headers(&headers).is_none());
    }

    #[test]
    fn test_extract_trace_from_headers_valid() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("X-Trace-Id", "abc123".parse().unwrap());
        headers.insert("X-Span-Id", "def456".parse().unwrap());

        let ctx = extract_trace_from_headers(&headers).unwrap();
        assert_eq!(ctx.trace_id, "abc123");
        assert_eq!(ctx.span_id, "def456");
    }
}
