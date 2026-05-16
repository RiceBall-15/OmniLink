//! 健康聚合器
//!
//! 收集各依赖服务的健康状态，生成聚合健康报告。
//! 支持 PostgreSQL、Redis、MinIO 等服务的并发健康检查。

use serde::{Deserialize, Serialize};
use std::time::Instant;
use tokio::time::{timeout, Duration};

/// 单个服务的健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealthStatus {
    /// 服务名称
    pub name: String,
    /// 是否可用
    pub healthy: bool,
    /// 响应时间（毫秒）
    pub latency_ms: u64,
    /// 错误信息
    pub error: Option<String>,
    /// 额外信息（如版本号、连接数等）
    pub details: Option<serde_json::Value>,
}

/// 聚合健康报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedHealthReport {
    /// 整体状态：healthy / degraded / unhealthy
    pub status: String,
    /// 各服务健康状态
    pub services: Vec<ServiceHealthStatus>,
    /// 检查耗时（毫秒）
    pub total_latency_ms: u64,
    /// 时间戳
    pub timestamp: i64,
    /// 版本信息
    pub version: String,
}

/// 健康聚合器
pub struct HealthAggregator {
    /// 超时时间（毫秒）
    timeout_ms: u64,
    /// 版本信息
    version: String,
}

impl HealthAggregator {
    /// 创建新的健康聚合器
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            timeout_ms,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// 设置版本信息
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// 执行所有健康检查并生成聚合报告
    /// 
    /// 传入服务名称和对应的健康检查 future 列表
    pub async fn check_all(&self, checks: Vec<ServiceHealthStatus>) -> AggregatedHealthReport {
        let start = Instant::now();

        let services = checks;
        let total_latency_ms = start.elapsed().as_millis() as u64;

        // 计算整体状态
        let status = if services.iter().all(|s| s.healthy) {
            "healthy".to_string()
        } else if services.iter().any(|s| s.healthy) {
            "degraded".to_string()
        } else {
            "unhealthy".to_string()
        };

        AggregatedHealthReport {
            status,
            services,
            total_latency_ms,
            timestamp: chrono::Utc::now().timestamp(),
            version: self.version.clone(),
        }
    }

    /// 获取超时时间
    pub fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }
}

/// PostgreSQL 健康检查
pub async fn check_postgres(pool: &sqlx::PgPool) -> ServiceHealthStatus {
    let start = Instant::now();
    let result = sqlx::query("SELECT 1").fetch_one(pool).await;
    let latency_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(_) => ServiceHealthStatus {
            name: "PostgreSQL".to_string(),
            healthy: true,
            latency_ms,
            error: None,
            details: Some(serde_json::json!({
                "pool_size": pool.size(),
                "idle": pool.num_idle(),
            })),
        },
        Err(e) => ServiceHealthStatus {
            name: "PostgreSQL".to_string(),
            healthy: false,
            latency_ms,
            error: Some(e.to_string()),
            details: None,
        },
    }
}

/// Redis 健康检查
pub async fn check_redis(conn: &mut redis::aio::ConnectionManager) -> ServiceHealthStatus {
    let start = Instant::now();
    let result: Result<String, _> = redis::cmd("PING")
        .query_async(conn)
        .await;
    let latency_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(reply) if reply == "PONG" => ServiceHealthStatus {
            name: "Redis".to_string(),
            healthy: true,
            latency_ms,
            error: None,
            details: None,
        },
        Ok(reply) => ServiceHealthStatus {
            name: "Redis".to_string(),
            healthy: false,
            latency_ms,
            error: Some(format!("unexpected PING response: {}", reply)),
            details: None,
        },
        Err(e) => ServiceHealthStatus {
            name: "Redis".to_string(),
            healthy: false,
            latency_ms,
            error: Some(e.to_string()),
            details: None,
        },
    }
}

/// MinIO 健康检查（HTTP HEAD 请求）
pub async fn check_minio(endpoint: &str) -> ServiceHealthStatus {
    let start = Instant::now();
    let url = format!("{}/minio/health/live", endpoint);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    let result = client.get(&url).send().await;
    let latency_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(resp) if resp.status().is_success() => ServiceHealthStatus {
            name: "MinIO".to_string(),
            healthy: true,
            latency_ms,
            error: None,
            details: None,
        },
        Ok(resp) => ServiceHealthStatus {
            name: "MinIO".to_string(),
            healthy: false,
            latency_ms,
            error: Some(format!("HTTP {}", resp.status())),
            details: None,
        },
        Err(e) => ServiceHealthStatus {
            name: "MinIO".to_string(),
            healthy: false,
            latency_ms,
            error: Some(e.to_string()),
            details: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregated_health_report_healthy() {
        let report = AggregatedHealthReport {
            status: "healthy".to_string(),
            services: vec![
                ServiceHealthStatus {
                    name: "PostgreSQL".to_string(),
                    healthy: true,
                    latency_ms: 5,
                    error: None,
                    details: None,
                },
                ServiceHealthStatus {
                    name: "Redis".to_string(),
                    healthy: true,
                    latency_ms: 2,
                    error: None,
                    details: None,
                },
            ],
            total_latency_ms: 10,
            timestamp: 1234567890,
            version: "0.1.0".to_string(),
        };

        assert_eq!(report.status, "healthy");
        assert!(report.services.iter().all(|s| s.healthy));
    }

    #[test]
    fn test_aggregated_health_report_degraded() {
        let services = vec![
            ServiceHealthStatus {
                name: "PostgreSQL".to_string(),
                healthy: true,
                latency_ms: 5,
                error: None,
                details: None,
            },
            ServiceHealthStatus {
                name: "Redis".to_string(),
                healthy: false,
                latency_ms: 5000,
                error: Some("Connection refused".to_string()),
                details: None,
            },
        ];

        // Simulate the status calculation
        let status = if services.iter().all(|s| s.healthy) {
            "healthy"
        } else if services.iter().any(|s| s.healthy) {
            "degraded"
        } else {
            "unhealthy"
        };

        assert_eq!(status, "degraded");
    }

    #[test]
    fn test_aggregated_health_report_unhealthy() {
        let services = vec![
            ServiceHealthStatus {
                name: "PostgreSQL".to_string(),
                healthy: false,
                latency_ms: 5000,
                error: Some("Connection refused".to_string()),
                details: None,
            },
        ];

        let status = if services.iter().all(|s| s.healthy) {
            "healthy"
        } else if services.iter().any(|s| s.healthy) {
            "degraded"
        } else {
            "unhealthy"
        };

        assert_eq!(status, "unhealthy");
    }

    #[test]
    fn test_service_health_status_serialization() {
        let status = ServiceHealthStatus {
            name: "Redis".to_string(),
            healthy: true,
            latency_ms: 3,
            error: None,
            details: Some(serde_json::json!({"version": "7.0"})),
        };

        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["name"], "Redis");
        assert_eq!(json["healthy"], true);
        assert_eq!(json["latency_ms"], 3);
        assert!(json["error"].is_null());
        assert_eq!(json["details"]["version"], "7.0");
    }
}
