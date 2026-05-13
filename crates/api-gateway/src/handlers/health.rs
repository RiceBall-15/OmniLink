use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;
use common::pool_monitor::{PoolMonitor, HealthStatus};

/// 健康检查端点处理器
pub async fn health_check(
    State(monitor): State<Arc<PoolMonitor>>,
) -> Response {
    let result = monitor.health_check().await;

    let status_code = match result.status {
        HealthStatus::Healthy => StatusCode::OK,
        HealthStatus::Warning => StatusCode::OK,
        HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };

    (status_code, Json(json!(result))).into_response()
}

/// 连接池状态端点处理器
pub async fn pool_stats(
    State(monitor): State<Arc<PoolMonitor>>,
) -> Json<serde_json::Value> {
    let stats = monitor.get_pool_stats().await;
    Json(json!(stats))
}

/// 慢查询日志端点处理器
pub async fn slow_queries(
    State(monitor): State<Arc<PoolMonitor>>,
) -> Json<serde_json::Value> {
    let queries = monitor.get_slow_queries(50).await;
    Json(json!({
        "slow_queries": queries,
        "total": queries.len(),
    }))
}

/// Prometheus 指标端点处理器
pub async fn prometheus_metrics(
    State(monitor): State<Arc<PoolMonitor>>,
) -> (StatusCode, [(String, String); 1], String) {
    let metrics = monitor.export_prometheus_metrics().await;
    (
        StatusCode::OK,
        [("Content-Type".to_string(), "text/plain; version=0.0.4; charset=utf-8".to_string())],
        metrics,
    )
}
