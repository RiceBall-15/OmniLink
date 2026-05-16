//! 系统指标收集与监控
//!
//! 提供运行时指标收集，包括：
//! - 请求计数（总数、按端点）
//! - 响应时间（平均、P50/P95/P99）
//! - 错误率
//! - WebSocket 连接数
//! - 消息吞吐量
//! - 系统资源使用

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::Serialize;

/// 请求指标
#[derive(Debug, Clone, Serialize)]
pub struct RequestMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_response_time_ms: f64,
    pub p50_response_time_ms: f64,
    pub p95_response_time_ms: f64,
    pub p99_response_time_ms: f64,
    pub requests_per_second: f64,
}

/// 端点指标
#[derive(Debug, Clone, Serialize)]
pub struct EndpointMetrics {
    pub path: String,
    pub method: String,
    pub request_count: u64,
    pub error_count: u64,
    pub avg_response_time_ms: f64,
}

/// WebSocket 指标
#[derive(Debug, Clone, Serialize)]
pub struct WebSocketMetrics {
    pub active_connections: u64,
    pub total_connections: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub auth_failures: u64,
}

/// 消息指标
#[derive(Debug, Clone, Serialize)]
pub struct MessageMetrics {
    pub total_messages: u64,
    pub messages_per_minute: f64,
    pub offline_messages_queued: u64,
    pub offline_messages_delivered: u64,
}

/// 系统资源指标
#[derive(Debug, Clone, Serialize)]
pub struct SystemMetrics {
    pub uptime_secs: u64,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub cpu_usage_percent: f64,
    pub goroutines: u64, // tokio tasks
}

/// 完整指标快照
#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    pub timestamp: i64,
    pub requests: RequestMetrics,
    pub websockets: WebSocketMetrics,
    pub messages: MessageMetrics,
    pub system: SystemMetrics,
    pub top_endpoints: Vec<EndpointMetrics>,
}

/// 响应时间记录
struct ResponseTimeRecord {
    times: Vec<f64>,
    max_size: usize,
}

impl ResponseTimeRecord {
    fn new(max_size: usize) -> Self {
        Self {
            times: Vec::with_capacity(max_size),
            max_size,
        }
    }

    fn record(&mut self, time_ms: f64) {
        if self.times.len() >= self.max_size {
            self.times.remove(0);
        }
        self.times.push(time_ms);
    }

    fn avg(&self) -> f64 {
        if self.times.is_empty() {
            return 0.0;
        }
        self.times.iter().sum::<f64>() / self.times.len() as f64
    }

    fn percentile(&self, p: f64) -> f64 {
        if self.times.is_empty() {
            return 0.0;
        }
        let mut sorted = self.times.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = ((p / 100.0) * sorted.len() as f64) as usize;
        sorted.get(idx).copied().unwrap_or(0.0)
    }
}

/// 指标收集器
pub struct MetricsCollector {
    start_time: Instant,
    // Request metrics
    total_requests: Arc<RwLock<u64>>,
    successful_requests: Arc<RwLock<u64>>,
    failed_requests: Arc<RwLock<u64>>,
    response_times: Arc<RwLock<ResponseTimeRecord>>,
    // Endpoint metrics
    endpoint_stats: Arc<RwLock<HashMap<String, (u64, u64, f64)>>>, // (count, errors, total_time)
    // WebSocket metrics
    ws_active: Arc<RwLock<u64>>,
    ws_total: Arc<RwLock<u64>>,
    ws_sent: Arc<RwLock<u64>>,
    ws_received: Arc<RwLock<u64>>,
    ws_auth_failures: Arc<RwLock<u64>>,
    // Message metrics
    total_messages: Arc<RwLock<u64>>,
    offline_queued: Arc<RwLock<u64>>,
    offline_delivered: Arc<RwLock<u64>>,
    message_timestamps: Arc<RwLock<Vec<Instant>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            total_requests: Arc::new(RwLock::new(0)),
            successful_requests: Arc::new(RwLock::new(0)),
            failed_requests: Arc::new(RwLock::new(0)),
            response_times: Arc::new(RwLock::new(ResponseTimeRecord::new(10000))),
            endpoint_stats: Arc::new(RwLock::new(HashMap::new())),
            ws_active: Arc::new(RwLock::new(0)),
            ws_total: Arc::new(RwLock::new(0)),
            ws_sent: Arc::new(RwLock::new(0)),
            ws_received: Arc::new(RwLock::new(0)),
            ws_auth_failures: Arc::new(RwLock::new(0)),
            total_messages: Arc::new(RwLock::new(0)),
            offline_queued: Arc::new(RwLock::new(0)),
            offline_delivered: Arc::new(RwLock::new(0)),
            message_timestamps: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 记录 HTTP 请求
    pub async fn record_request(&self, method: &str, path: &str, status: u16, duration: Duration) {
        let duration_ms = duration.as_secs_f64() * 1000.0;

        // 总数
        {
            let mut total = self.total_requests.write().await;
            *total += 1;
        }

        if status < 400 {
            let mut success = self.successful_requests.write().await;
            *success += 1;
        } else {
            let mut failed = self.failed_requests.write().await;
            *failed += 1;
        }

        // 响应时间
        {
            let mut times = self.response_times.write().await;
            times.record(duration_ms);
        }

        // 端点统计
        let key = format!("{} {}", method, path);
        {
            let mut endpoints = self.endpoint_stats.write().await;
            let entry = endpoints.entry(key).or_insert((0, 0, 0.0));
            entry.0 += 1;
            if status >= 400 {
                entry.1 += 1;
            }
            entry.2 += duration_ms;
        }
    }

    /// 记录 WebSocket 连接
    pub async fn record_ws_connect(&self) {
        let mut active = self.ws_active.write().await;
        *active += 1;
        let mut total = self.ws_total.write().await;
        *total += 1;
    }

    /// 记录 WebSocket 断开
    pub async fn record_ws_disconnect(&self) {
        let mut active = self.ws_active.write().await;
        if *active > 0 {
            *active -= 1;
        }
    }

    /// 记录 WebSocket 消息
    pub async fn record_ws_message(&self, direction: &str) {
        match direction {
            "sent" => {
                let mut sent = self.ws_sent.write().await;
                *sent += 1;
            }
            "received" => {
                let mut recv = self.ws_received.write().await;
                *recv += 1;
            }
            _ => {}
        }
    }

    /// 记录 WebSocket 认证失败
    pub async fn record_ws_auth_failure(&self) {
        let mut failures = self.ws_auth_failures.write().await;
        *failures += 1;
    }

    /// 记录消息
    pub async fn record_message(&self) {
        let mut total = self.total_messages.write().await;
        *total += 1;
        let mut timestamps = self.message_timestamps.write().await;
        timestamps.push(Instant::now());
        // 保留最近 5 分钟的时间戳
        let cutoff = Instant::now() - Duration::from_secs(300);
        timestamps.retain(|t| *t > cutoff);
    }

    /// 记录离线消息
    pub async fn record_offline_queued(&self) {
        let mut queued = self.offline_queued.write().await;
        *queued += 1;
    }

    pub async fn record_offline_delivered(&self) {
        let mut delivered = self.offline_delivered.write().await;
        *delivered += 1;
    }

    /// 获取完整指标快照
    pub async fn snapshot(&self) -> MetricsSnapshot {
        let total = *self.total_requests.read().await;
        let success = *self.successful_requests.read().await;
        let failed = *self.failed_requests.read().await;

        let times = self.response_times.read().await;
        let avg = times.avg();
        let p50 = times.percentile(50.0);
        let p95 = times.percentile(95.0);
        let p99 = times.percentile(99.0);

        let uptime = self.start_time.elapsed().as_secs();
        let rps = if uptime > 0 {
            total as f64 / uptime as f64
        } else {
            0.0
        };

        // Top endpoints
        let endpoints = self.endpoint_stats.read().await;
        let mut top_endpoints: Vec<EndpointMetrics> = endpoints
            .iter()
            .map(|(key, (count, errors, total_time))| {
                let parts: Vec<&str> = key.splitn(2, ' ').collect();
                EndpointMetrics {
                    method: parts.first().unwrap_or(&"").to_string(),
                    path: parts.get(1).unwrap_or(&"").to_string(),
                    request_count: *count,
                    error_count: *errors,
                    avg_response_time_ms: if *count > 0 { total_time / *count as f64 } else { 0.0 },
                }
            })
            .collect();
        top_endpoints.sort_by(|a, b| b.request_count.cmp(&a.request_count));
        top_endpoints.truncate(10);

        // Message throughput
        let msg_timestamps = self.message_timestamps.read().await;
        let messages_per_minute = msg_timestamps.len() as f64 / 5.0; // 5-minute window

        // System metrics (simplified)
        let (mem_used, mem_total) = get_memory_usage();

        MetricsSnapshot {
            timestamp: chrono::Utc::now().timestamp(),
            requests: RequestMetrics {
                total_requests: total,
                successful_requests: success,
                failed_requests: failed,
                avg_response_time_ms: avg,
                p50_response_time_ms: p50,
                p95_response_time_ms: p95,
                p99_response_time_ms: p99,
                requests_per_second: rps,
            },
            websockets: WebSocketMetrics {
                active_connections: *self.ws_active.read().await,
                total_connections: *self.ws_total.read().await,
                messages_sent: *self.ws_sent.read().await,
                messages_received: *self.ws_received.read().await,
                auth_failures: *self.ws_auth_failures.read().await,
            },
            messages: MessageMetrics {
                total_messages: *self.total_messages.read().await,
                messages_per_minute,
                offline_messages_queued: *self.offline_queued.read().await,
                offline_messages_delivered: *self.offline_delivered.read().await,
            },
            system: SystemMetrics {
                uptime_secs: uptime,
                memory_used_mb: mem_used,
                memory_total_mb: mem_total,
                cpu_usage_percent: 0.0, // Would need /proc/stat parsing
                goroutines: 0, // Would need tokio runtime stats
            },
            top_endpoints,
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// 获取内存使用情况（MB）
fn get_memory_usage() -> (u64, u64) {
    #[cfg(target_os = "linux")]
    {
        if let Ok(info) = std::fs::read_to_string("/proc/meminfo") {
            let mut total = 0u64;
            let mut available = 0u64;
            for line in info.lines() {
                if line.starts_with("MemTotal:") {
                    total = parse_mem_value(line);
                } else if line.starts_with("MemAvailable:") {
                    available = parse_mem_value(line);
                }
            }
            let used = total.saturating_sub(available);
            return (used / 1024, total / 1024); // Convert KB to MB
        }
    }
    (0, 0)
}

#[cfg(target_os = "linux")]
fn parse_mem_value(line: &str) -> u64 {
    line.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0)
}

/// 指标中间件 - 用于 Axum
pub mod middleware {
    use super::*;
    use axum::{
        extract::State,
        http::Request,
        middleware::Next,
        response::Response,
    };

    /// 指标中间件处理函数
    pub async fn metrics_middleware(
        State(collector): State<Arc<MetricsCollector>>,
        request: Request<axum::body::Body>,
        next: Next,
    ) -> Response {
        let method = request.method().to_string();
        let path = request.uri().path().to_string();
        let start = Instant::now();

        let response = next.run(request).await;

        let duration = start.elapsed();
        let status = response.status().as_u16();

        collector.record_request(&method, &path, status, duration).await;

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collector_request() {
        let collector = MetricsCollector::new();

        collector.record_request("GET", "/api/test", 200, Duration::from_millis(50)).await;
        collector.record_request("POST", "/api/test", 201, Duration::from_millis(100)).await;
        collector.record_request("GET", "/api/test", 500, Duration::from_millis(200)).await;

        let snapshot = collector.snapshot().await;
        assert_eq!(snapshot.requests.total_requests, 3);
        assert_eq!(snapshot.requests.successful_requests, 2);
        assert_eq!(snapshot.requests.failed_requests, 1);
    }

    #[tokio::test]
    async fn test_metrics_collector_websocket() {
        let collector = MetricsCollector::new();

        collector.record_ws_connect().await;
        collector.record_ws_connect().await;
        collector.record_ws_disconnect().await;

        let snapshot = collector.snapshot().await;
        assert_eq!(snapshot.websockets.active_connections, 1);
        assert_eq!(snapshot.websockets.total_connections, 2);
    }

    #[tokio::test]
    async fn test_metrics_collector_messages() {
        let collector = MetricsCollector::new();

        collector.record_message().await;
        collector.record_message().await;
        collector.record_offline_queued().await;
        collector.record_offline_delivered().await;

        let snapshot = collector.snapshot().await;
        assert_eq!(snapshot.messages.total_messages, 2);
        assert_eq!(snapshot.messages.offline_messages_queued, 1);
        assert_eq!(snapshot.messages.offline_messages_delivered, 1);
    }

    #[tokio::test]
    async fn test_response_time_percentiles() {
        let collector = MetricsCollector::new();

        // Record 100 requests with varying response times
        for i in 0..100 {
            collector.record_request(
                "GET",
                "/api/test",
                200,
                Duration::from_millis(i * 10),
            ).await;
        }

        let snapshot = collector.snapshot().await;
        assert!(snapshot.requests.p50_response_time_ms > 0.0);
        assert!(snapshot.requests.p95_response_time_ms > snapshot.requests.p50_response_time_ms);
        assert!(snapshot.requests.p99_response_time_ms >= snapshot.requests.p95_response_time_ms);
    }

    #[tokio::test]
    async fn test_endpoint_stats() {
        let collector = MetricsCollector::new();

        collector.record_request("GET", "/api/messages", 200, Duration::from_millis(10)).await;
        collector.record_request("GET", "/api/messages", 200, Duration::from_millis(20)).await;
        collector.record_request("POST", "/api/messages", 201, Duration::from_millis(30)).await;

        let snapshot = collector.snapshot().await;
        assert_eq!(snapshot.top_endpoints.len(), 2);
    }

    #[tokio::test]
    async fn test_ws_messages() {
        let collector = MetricsCollector::new();

        collector.record_ws_message("sent").await;
        collector.record_ws_message("sent").await;
        collector.record_ws_message("received").await;
        collector.record_ws_auth_failure().await;

        let snapshot = collector.snapshot().await;
        assert_eq!(snapshot.websockets.messages_sent, 2);
        assert_eq!(snapshot.websockets.messages_received, 1);
        assert_eq!(snapshot.websockets.auth_failures, 1);
    }
}
