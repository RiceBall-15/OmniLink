use axum::{extract::State, extract::Query, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::Instant;
use common::health_aggregator::{HealthAggregator, ServiceHealthStatus};

/// 标准化健康检查响应
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HealthCheckResponse {
    /// 服务状态: healthy, degraded, unhealthy
    pub status: String,
    /// 服务版本
    pub version: String,
    /// 检查时间戳
    pub timestamp: String,
    /// 依赖服务状态
    pub dependencies: Dependencies,
}

/// 依赖服务状态
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct Dependencies {
    pub database: DependencyStatus,
    pub redis: DependencyStatus,
}

/// 单个依赖的状态
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DependencyStatus {
    pub status: String,
    pub response_time_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 标准化健康检查处理函数
#[utoipa::path(
    get,
    path = "/api/health",
    tag = "health",
    responses(
        (status = 200, description = "服务健康", body = HealthCheckResponse),
    )
)]
pub async fn health_check_with_deps(State(pool): State<PgPool>) -> Json<HealthCheckResponse> {
    let version = env!("CARGO_PKG_VERSION").to_string();

    // 检查数据库连接
    let db_status = check_database(&pool).await;

    // 检查 Redis 连接（TCP ping）
    let redis_status = check_redis_tcp().await;

    // 确定整体状态
    let overall_status = if db_status.status == "healthy" && redis_status.status == "healthy" {
        "healthy"
    } else if db_status.status == "unhealthy" || redis_status.status == "unhealthy" {
        "unhealthy"
    } else {
        "degraded"
    };

    Json(HealthCheckResponse {
        status: overall_status.to_string(),
        version,
        timestamp: chrono::Utc::now().to_rfc3339(),
        dependencies: Dependencies {
            database: db_status,
            redis: redis_status,
        },
    })
}

/// 检查数据库连接
async fn check_database(pool: &PgPool) -> DependencyStatus {
    let start = Instant::now();

    match sqlx::query("SELECT 1").execute(pool).await {
        Ok(_) => DependencyStatus {
            status: "healthy".to_string(),
            response_time_ms: start.elapsed().as_millis() as u64,
            error: None,
        },
        Err(e) => DependencyStatus {
            status: "unhealthy".to_string(),
            response_time_ms: start.elapsed().as_millis() as u64,
            error: Some(e.to_string()),
        },
    }
}

/// 检查 Redis 连接（使用 TCP 连接检测）
async fn check_redis_tcp() -> DependencyStatus {
    let start = Instant::now();

    match tokio::net::TcpStream::connect("127.0.0.1:6379").await {
        Ok(_) => DependencyStatus {
            status: "healthy".to_string(),
            response_time_ms: start.elapsed().as_millis() as u64,
            error: None,
        },
        Err(e) => DependencyStatus {
            status: "unhealthy".to_string(),
            response_time_ms: start.elapsed().as_millis() as u64,
            error: Some(e.to_string()),
        },
    }
}

/// 聚合健康检查查询参数
#[derive(Debug, Deserialize)]
pub struct HealthQuery {
    /// 检查模式：deep（检查所有依赖）或 shallow（仅检查自身）
    #[serde(default = "default_mode")]
    pub mode: String,
}

fn default_mode() -> String {
    "deep".to_string()
}

/// 聚合健康检查响应
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AggregatedHealthResponse {
    /// 整体状态：healthy / degraded / unhealthy
    pub status: String,
    /// 各服务健康状态
    pub services: Vec<HealthServiceStatus>,
    /// 检查耗时（毫秒）
    pub total_latency_ms: u64,
    /// 时间戳
    pub timestamp: i64,
    /// 版本信息
    pub version: String,
}

/// 单个服务健康状态
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HealthServiceStatus {
    /// 服务名称
    pub name: String,
    /// 是否可用
    pub healthy: bool,
    /// 响应时间（毫秒）
    pub latency_ms: u64,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 聚合健康检查端点
///
/// 并发检查所有服务依赖（PostgreSQL、Redis），返回聚合健康报告。
/// 支持 deep/shallow 检查模式。
#[utoipa::path(
    get,
    path = "/api/health/status",
    tag = "health",
    params(
        ("mode" = Option<String>, Query, description = "检查模式: deep 或 shallow"),
    ),
    responses(
        (status = 200, description = "聚合健康报告", body = AggregatedHealthResponse),
    )
)]
pub async fn aggregated_health_check(
    State(pool): State<PgPool>,
    Query(query): Query<HealthQuery>,
) -> Json<AggregatedHealthResponse> {
    let aggregator = HealthAggregator::new(5000);

    let services = if query.mode == "shallow" {
        // shallow 模式：只检查自身，不检查依赖
        vec![ServiceHealthStatus {
            name: "im-api".to_string(),
            healthy: true,
            latency_ms: 0,
            error: None,
            details: None,
        }]
    } else {
        // deep 模式：并发检查所有依赖
        let db_check = common::health_aggregator::check_postgres(&pool);
        let redis_check = async {
            match tokio::net::TcpStream::connect("127.0.0.1:6379").await {
                Ok(_) => ServiceHealthStatus {
                    name: "Redis".to_string(),
                    healthy: true,
                    latency_ms: 0,
                    error: None,
                    details: None,
                },
                Err(e) => ServiceHealthStatus {
                    name: "Redis".to_string(),
                    healthy: false,
                    latency_ms: 0,
                    error: Some(e.to_string()),
                    details: None,
                },
            }
        };

        let (db_result, redis_result) = tokio::join!(db_check, redis_check);
        vec![db_result, redis_result]
    };

    let report = aggregator.check_all(services).await;

    Json(AggregatedHealthResponse {
        status: report.status,
        services: report
            .services
            .into_iter()
            .map(|s| HealthServiceStatus {
                name: s.name,
                healthy: s.healthy,
                latency_ms: s.latency_ms,
                error: s.error,
            })
            .collect(),
        total_latency_ms: report.total_latency_ms,
        timestamp: report.timestamp,
        version: report.version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serialization() {
        let response = HealthCheckResponse {
            status: "healthy".to_string(),
            version: "0.1.0".to_string(),
            timestamp: "2026-05-13T06:07:00Z".to_string(),
            dependencies: Dependencies {
                database: DependencyStatus {
                    status: "healthy".to_string(),
                    response_time_ms: 5,
                    error: None,
                },
                redis: DependencyStatus {
                    status: "healthy".to_string(),
                    response_time_ms: 2,
                    error: None,
                },
            },
        };

        let json = match serde_json::to_value(&response) {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Serialization failed: {}", e)}))),
        };
        assert_eq!(json["status"], "healthy");
        assert_eq!(json["version"], "0.1.0");
        assert!(json["dependencies"]["database"]["response_time_ms"].is_number());
    }
}
