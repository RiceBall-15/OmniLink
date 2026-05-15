use axum::{extract::State, Json};
use serde::Serialize;
use sqlx::PgPool;
use std::time::Instant;

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
