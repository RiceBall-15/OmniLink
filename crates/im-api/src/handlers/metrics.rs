use axum::{extract::State, Json};
use serde::Serialize;
use sqlx::PgPool;
use std::time::SystemTime;

/// 应用指标响应
#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    /// 服务运行时间（秒）
    pub uptime_seconds: u64,
    /// 当前时间
    pub timestamp: String,
    /// 数据库连接池状态
    pub database: DatabaseMetrics,
}

/// 数据库指标
#[derive(Debug, Serialize)]
pub struct DatabaseMetrics {
    /// 连接池大小
    pub pool_size: u32,
    /// 空闲连接数
    pub idle_connections: u32,
}

/// 全局启动时间
static START_TIME: std::sync::OnceLock<SystemTime> = std::sync::OnceLock::new();

/// 初始化启动时间
pub fn init_start_time() {
    START_TIME.get_or_init(SystemTime::now);
}

/// 获取应用指标
pub async fn get_metrics(State(pool): State<PgPool>) -> Json<MetricsResponse> {
    let uptime = START_TIME
        .get()
        .and_then(|start| start.elapsed().ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let pool_size = pool.size();
    let idle = pool.num_idle() as u32;

    Json(MetricsResponse {
        uptime_seconds: uptime,
        timestamp: chrono::Utc::now().to_rfc3339(),
        database: DatabaseMetrics {
            pool_size,
            idle_connections: idle,
        },
    })
}
