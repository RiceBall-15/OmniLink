//! 应用指标和监控模块
//!
//! 提供 Prometheus 格式指标和业务指标统计。
//! 包括：请求计数、响应时间、错误率、业务指标等。

use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;
use sqlx::PgPool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

/// 全局启动时间
static START_TIME: std::sync::OnceLock<SystemTime> = std::sync::OnceLock::new();

/// 全局业务指标计数器
static METRICS: BusinessMetrics = BusinessMetrics {
    total_requests: AtomicU64::new(0),
    total_errors: AtomicU64::new(0),
    total_messages_sent: AtomicU64::new(0),
    total_messages_received: AtomicU64::new(0),
    total_conversations_created: AtomicU64::new(0),
    total_users_registered: AtomicU64::new(0),
    total_ws_connections: AtomicU64::new(0),
    total_auth_failures: AtomicU64::new(0),
};

/// 业务指标计数器
pub struct BusinessMetrics {
    /// 总请求数
    pub total_requests: AtomicU64,
    /// 总错误数
    pub total_errors: AtomicU64,
    /// 发送消息总数
    pub total_messages_sent: AtomicU64,
    /// 接收消息总数
    pub total_messages_received: AtomicU64,
    /// 创建会话总数
    pub total_conversations_created: AtomicU64,
    /// 注册用户总数
    pub total_users_registered: AtomicU64,
    /// WebSocket 连接总数
    pub total_ws_connections: AtomicU64,
    /// 认证失败总数
    pub total_auth_failures: AtomicU64,
}

/// 递增请求计数
pub fn inc_request_count() {
    METRICS.total_requests.fetch_add(1, Ordering::Relaxed);
}

/// 递增错误计数
pub fn inc_error_count() {
    METRICS.total_errors.fetch_add(1, Ordering::Relaxed);
}

/// 递增发送消息计数
pub fn inc_message_sent() {
    METRICS.total_messages_sent.fetch_add(1, Ordering::Relaxed);
}

/// 递增接收消息计数
pub fn inc_message_received() {
    METRICS.total_messages_received.fetch_add(1, Ordering::Relaxed);
}

/// 递增创建会话计数
pub fn inc_conversation_created() {
    METRICS.total_conversations_created.fetch_add(1, Ordering::Relaxed);
}

/// 递增注册用户计数
pub fn inc_user_registered() {
    METRICS.total_users_registered.fetch_add(1, Ordering::Relaxed);
}

/// 递增 WebSocket 连接计数
pub fn inc_ws_connection() {
    METRICS.total_ws_connections.fetch_add(1, Ordering::Relaxed);
}

/// 递减 WebSocket 连接计数
pub fn dec_ws_connection() {
    METRICS.total_ws_connections.fetch_sub(1, Ordering::Relaxed);
}

/// 递增认证失败计数
pub fn inc_auth_failure() {
    METRICS.total_auth_failures.fetch_add(1, Ordering::Relaxed);
}

/// 应用指标响应（JSON格式）
#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    /// 服务运行时间（秒）
    pub uptime_seconds: u64,
    /// 当前时间
    pub timestamp: String,
    /// 数据库连接池状态
    pub database: DatabaseMetrics,
    /// 业务指标
    pub business: BusinessMetricsSnapshot,
}

/// 数据库指标
#[derive(Debug, Serialize)]
pub struct DatabaseMetrics {
    /// 连接池大小
    pub pool_size: u32,
    /// 空闲连接数
    pub idle_connections: u32,
}

/// 业务指标快照
#[derive(Debug, Serialize)]
pub struct BusinessMetricsSnapshot {
    /// 总请求数
    pub total_requests: u64,
    /// 总错误数
    pub total_errors: u64,
    /// 错误率（百分比）
    pub error_rate_percent: f64,
    /// 发送消息总数
    pub messages_sent: u64,
    /// 接收消息总数
    pub messages_received: u64,
    /// 创建会话总数
    pub conversations_created: u64,
    /// 注册用户总数
    pub users_registered: u64,
    /// 当前 WebSocket 连接数
    pub ws_connections: u64,
    /// 认证失败总数
    pub auth_failures: u64,
}

/// 初始化启动时间
pub fn init_start_time() {
    START_TIME.get_or_init(SystemTime::now);
}

/// 获取应用指标（JSON格式）
pub async fn get_metrics(State(pool): State<PgPool>) -> Json<MetricsResponse> {
    let uptime = START_TIME
        .get()
        .and_then(|start| start.elapsed().ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let pool_size = pool.size();
    let idle = pool.num_idle() as u32;

    let total_requests = METRICS.total_requests.load(Ordering::Relaxed);
    let total_errors = METRICS.total_errors.load(Ordering::Relaxed);
    let error_rate = if total_requests > 0 {
        (total_errors as f64 / total_requests as f64) * 100.0
    } else {
        0.0
    };

    Json(MetricsResponse {
        uptime_seconds: uptime,
        timestamp: chrono::Utc::now().to_rfc3339(),
        database: DatabaseMetrics {
            pool_size,
            idle_connections: idle,
        },
        business: BusinessMetricsSnapshot {
            total_requests,
            total_errors,
            error_rate_percent: (error_rate * 100.0).round() / 100.0,
            messages_sent: METRICS.total_messages_sent.load(Ordering::Relaxed),
            messages_received: METRICS.total_messages_received.load(Ordering::Relaxed),
            conversations_created: METRICS.total_conversations_created.load(Ordering::Relaxed),
            users_registered: METRICS.total_users_registered.load(Ordering::Relaxed),
            ws_connections: METRICS.total_ws_connections.load(Ordering::Relaxed),
            auth_failures: METRICS.total_auth_failures.load(Ordering::Relaxed),
        },
    })
}

/// Prometheus 格式指标端点
///
/// 返回 Prometheus 兼容的文本格式指标，可直接被 Prometheus 抓取。
pub async fn get_prometheus_metrics(State(pool): State<PgPool>) -> impl IntoResponse {
    let uptime = START_TIME
        .get()
        .and_then(|start| start.elapsed().ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let pool_size = pool.size();
    let idle = pool.num_idle() as u32;

    let total_requests = METRICS.total_requests.load(Ordering::Relaxed);
    let total_errors = METRICS.total_errors.load(Ordering::Relaxed);

    let metrics_text = format!(
        r#"# HELP omnilink_uptime_seconds Service uptime in seconds
# TYPE omnilink_uptime_seconds gauge
omnilink_uptime_seconds {uptime}

# HELP omnilink_db_pool_size Database connection pool size
# TYPE omnilink_db_pool_size gauge
omnilink_db_pool_size {pool_size}

# HELP omnilink_db_idle_connections Database idle connections
# TYPE omnilink_db_idle_connections gauge
omnilink_db_idle_connections {idle}

# HELP omnilink_requests_total Total number of HTTP requests
# TYPE omnilink_requests_total counter
omnilink_requests_total {total_requests}

# HELP omnilink_errors_total Total number of errors
# TYPE omnilink_errors_total counter
omnilink_errors_total {total_errors}

# HELP omnilink_messages_sent_total Total messages sent
# TYPE omnilink_messages_sent_total counter
omnilink_messages_sent_total {messages_sent}

# HELP omnilink_messages_received_total Total messages received
# TYPE omnilink_messages_received_total counter
omnilink_messages_received_total {messages_received}

# HELP omnilink_conversations_created_total Total conversations created
# TYPE omnilink_conversations_created_total counter
omnilink_conversations_created_total {conversations_created}

# HELP omnilink_users_registered_total Total users registered
# TYPE omnilink_users_registered_total counter
omnilink_users_registered_total {users_registered}

# HELP omnilink_ws_connections Current WebSocket connections
# TYPE omnilink_ws_connections gauge
omnilink_ws_connections {ws_connections}

# HELP omnilink_auth_failures_total Total authentication failures
# TYPE omnilink_auth_failures_total counter
omnilink_auth_failures_total {auth_failures}
"#,
        uptime = uptime,
        pool_size = pool_size,
        idle = idle,
        total_requests = total_requests,
        total_errors = total_errors,
        messages_sent = METRICS.total_messages_sent.load(Ordering::Relaxed),
        messages_received = METRICS.total_messages_received.load(Ordering::Relaxed),
        conversations_created = METRICS.total_conversations_created.load(Ordering::Relaxed),
        users_registered = METRICS.total_users_registered.load(Ordering::Relaxed),
        ws_connections = METRICS.total_ws_connections.load(Ordering::Relaxed),
        auth_failures = METRICS.total_auth_failures.load(Ordering::Relaxed),
    );

    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
        metrics_text,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inc_request_count() {
        let before = METRICS.total_requests.load(Ordering::Relaxed);
        inc_request_count();
        let after = METRICS.total_requests.load(Ordering::Relaxed);
        assert_eq!(after, before + 1);
    }

    #[test]
    fn test_inc_error_count() {
        let before = METRICS.total_errors.load(Ordering::Relaxed);
        inc_error_count();
        let after = METRICS.total_errors.load(Ordering::Relaxed);
        assert_eq!(after, before + 1);
    }

    #[test]
    fn test_inc_message_sent() {
        let before = METRICS.total_messages_sent.load(Ordering::Relaxed);
        inc_message_sent();
        let after = METRICS.total_messages_sent.load(Ordering::Relaxed);
        assert_eq!(after, before + 1);
    }

    #[test]
    fn test_ws_connection_count() {
        let before = METRICS.total_ws_connections.load(Ordering::Relaxed);
        inc_ws_connection();
        assert_eq!(METRICS.total_ws_connections.load(Ordering::Relaxed), before + 1);
        dec_ws_connection();
        assert_eq!(METRICS.total_ws_connections.load(Ordering::Relaxed), before);
    }

    #[test]
    fn test_inc_auth_failure() {
        let before = METRICS.total_auth_failures.load(Ordering::Relaxed);
        inc_auth_failure();
        let after = METRICS.total_auth_failures.load(Ordering::Relaxed);
        assert_eq!(after, before + 1);
    }

    #[test]
    fn test_init_start_time() {
        init_start_time();
        assert!(START_TIME.get().is_some());
    }
}
