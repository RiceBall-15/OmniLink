use sqlx::{Pool, Postgres};
use redis::aio::ConnectionManager;
use serde::{Serialize};
use std::sync::Arc;
use std::time::{Instant, Duration};
use tokio::sync::RwLock;
use std::collections::VecDeque;

/// 连接池状态指标
#[derive(Debug, Clone, Serialize)]
pub struct PoolStats {
    /// 活跃连接数
    pub active_connections: u32,
    /// 空闲连接数
    pub idle_connections: u32,
    /// 等待获取连接的请求数
    pub waiting_requests: u32,
    /// 最大连接数
    pub max_connections: u32,
    /// 最小连接数
    pub min_connections: u32,
    /// 连接池使用率（百分比）
    pub usage_percent: f64,
    /// 上次检查时间戳
    pub last_check_timestamp: i64,
}

/// 慢查询记录
#[derive(Debug, Clone, Serialize)]
pub struct SlowQuery {
    /// SQL 语句（截断）
    pub query: String,
    /// 执行时间（毫秒）
    pub duration_ms: u64,
    /// 发生时间戳
    pub timestamp: i64,
    /// 来源（哪个服务）
    pub source: String,
}

/// 连接池健康状态
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum HealthStatus {
    /// 健康
    Healthy,
    /// 警告（连接池使用率高）
    Warning,
    /// 不健康（连接失败）
    Unhealthy,
}

/// 连接池健康检查结果
#[derive(Debug, Clone, Serialize)]
pub struct HealthCheckResult {
    /// 整体状态
    pub status: HealthStatus,
    /// PostgreSQL 状态
    pub postgres: ServiceHealth,
    /// Redis 状态
    pub redis: ServiceHealth,
    /// 检查时间戳
    pub timestamp: i64,
}

/// 单个服务的健康状态
#[derive(Debug, Clone, Serialize)]
pub struct ServiceHealth {
    /// 是否可用
    pub available: bool,
    /// 响应时间（毫秒）
    pub response_time_ms: u64,
    /// 错误信息（如果有）
    pub error: Option<String>,
}

/// Prometheus 指标格式
#[derive(Debug, Clone)]
pub struct PrometheusMetrics {
    pub gauges: Vec<(String, f64, String)>,  // (name, value, help)
    pub counters: Vec<(String, u64, String)>,
}

/// 连接池监控器
pub struct PoolMonitor {
    pg_pool: Pool<Postgres>,
    redis: ConnectionManager,
    slow_queries: Arc<RwLock<VecDeque<SlowQuery>>>,
    slow_query_threshold_ms: u64,
    max_slow_query_history: usize,
}

impl PoolMonitor {
    /// 创建新的监控器
    pub fn new(
        pg_pool: Pool<Postgres>,
        redis: ConnectionManager,
        slow_query_threshold_ms: u64,
    ) -> Self {
        Self {
            pg_pool,
            redis,
            slow_queries: Arc::new(RwLock::new(VecDeque::new())),
            slow_query_threshold_ms,
            max_slow_query_history: 100,
        }
    }

    /// 获取连接池状态指标
    pub async fn get_pool_stats(&self) -> PoolStats {
        let pool_options = self.pg_pool.options();
        let size = self.pg_pool.size();
        let idle = self.pg_pool.num_idle() as u32;
        let max_connections = pool_options.get_max_connections();
        let min_connections = pool_options.get_min_connections();

        // sqlx 不直接提供 waiting_requests，我们使用 size - idle 估算活跃连接
        let active = size - idle;

        let usage_percent = if max_connections > 0 {
            (active as f64 / max_connections as f64) * 100.0
        } else {
            0.0
        };

        PoolStats {
            active_connections: active,
            idle_connections: idle,
            waiting_requests: 0, // sqlx 不提供此指标
            max_connections,
            min_connections,
            usage_percent,
            last_check_timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// 记录慢查询
    pub async fn record_slow_query(&self, query: &str, duration: Duration, source: &str) {
        let duration_ms = duration.as_millis() as u64;

        if duration_ms >= self.slow_query_threshold_ms {
            let slow_query = SlowQuery {
                query: if query.len() > 200 {
                    format!("{}...", &query[..200])
                } else {
                    query.to_string()
                },
                duration_ms,
                timestamp: chrono::Utc::now().timestamp(),
                source: source.to_string(),
            };

            let mut queries = self.slow_queries.write().await;
            if queries.len() >= self.max_slow_query_history {
                queries.pop_front();
            }
            queries.push_back(slow_query);

            tracing::warn!(
                "Slow query detected: {}ms - {} - {}",
                duration_ms,
                source,
                if query.len() > 100 { &query[..100] } else { query }
            );
        }
    }

    /// 获取慢查询历史
    pub async fn get_slow_queries(&self, limit: usize) -> Vec<SlowQuery> {
        let queries = self.slow_queries.read().await;
        let start = if queries.len() > limit {
            queries.len() - limit
        } else {
            0
        };
        queries.range(start..).cloned().collect()
    }

    /// 执行健康检查
    pub async fn health_check(&self) -> HealthCheckResult {
        let timestamp = chrono::Utc::now().timestamp();

        // 检查 PostgreSQL
        let pg_start = Instant::now();
        let (pg_available, pg_error) = match sqlx::query("SELECT 1")
            .execute(&self.pg_pool)
            .await
        {
            Ok(_) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };
        let pg_response_time = pg_start.elapsed().as_millis() as u64;

        // 检查 Redis
        let redis_start = Instant::now();
        let (redis_available, redis_error) = {
            let mut conn = self.redis.clone();
            match redis::cmd("PING")
                .query_async::<_, String>(&mut conn)
                .await
            {
                Ok(_) => (true, None),
                Err(e) => (false, Some(e.to_string())),
            }
        };
        let redis_response_time = redis_start.elapsed().as_millis() as u64;

        // 确定整体状态
        let status = if !pg_available || !redis_available {
            HealthStatus::Unhealthy
        } else {
            let stats = self.get_pool_stats().await;
            if stats.usage_percent > 80.0 {
                HealthStatus::Warning
            } else {
                HealthStatus::Healthy
            }
        };

        HealthCheckResult {
            status,
            postgres: ServiceHealth {
                available: pg_available,
                response_time_ms: pg_response_time,
                error: pg_error,
            },
            redis: ServiceHealth {
                available: redis_available,
                response_time_ms: redis_response_time,
                error: redis_error,
            },
            timestamp,
        }
    }

    /// 导出 Prometheus 格式指标
    pub async fn export_prometheus_metrics(&self) -> String {
        let stats = self.get_pool_stats().await;
        let health = self.health_check().await;
        let slow_queries = self.get_slow_queries(100).await;

        let mut output = String::new();

        // 连接池指标
        output.push_str("# HELP omnilink_pg_pool_active Active PostgreSQL connections\n");
        output.push_str("# TYPE omnilink_pg_pool_active gauge\n");
        output.push_str(&format!("omnilink_pg_pool_active {}\n", stats.active_connections));

        output.push_str("# HELP omnilink_pg_pool_idle Idle PostgreSQL connections\n");
        output.push_str("# TYPE omnilink_pg_pool_idle gauge\n");
        output.push_str(&format!("omnilink_pg_pool_idle {}\n", stats.idle_connections));

        output.push_str("# HELP omnilink_pg_pool_max Maximum PostgreSQL connections\n");
        output.push_str("# TYPE omnilink_pg_pool_max gauge\n");
        output.push_str(&format!("omnilink_pg_pool_max {}\n", stats.max_connections));

        output.push_str("# HELP omnilink_pg_pool_usage_percent PostgreSQL connection pool usage percentage\n");
        output.push_str("# TYPE omnilink_pg_pool_usage_percent gauge\n");
        output.push_str(&format!("omnilink_pg_pool_usage_percent {:.2}\n", stats.usage_percent));

        // 健康状态指标
        let pg_health = if health.postgres.available { 1.0 } else { 0.0 };
        let redis_health = if health.redis.available { 1.0 } else { 0.0 };

        output.push_str("# HELP omnilink_postgres_healthy PostgreSQL availability\n");
        output.push_str("# TYPE omnilink_postgres_healthy gauge\n");
        output.push_str(&format!("omnilink_postgres_healthy {}\n", pg_health));

        output.push_str("# HELP omnilink_redis_healthy Redis availability\n");
        output.push_str("# TYPE omnilink_redis_healthy gauge\n");
        output.push_str(&format!("omnilink_redis_healthy {}\n", redis_health));

        output.push_str("# HELP omnilink_postgres_response_time_ms PostgreSQL response time in milliseconds\n");
        output.push_str("# TYPE omnilink_postgres_response_time_ms gauge\n");
        output.push_str(&format!("omnilink_postgres_response_time_ms {}\n", health.postgres.response_time_ms));

        output.push_str("# HELP omnilink_redis_response_time_ms Redis response time in milliseconds\n");
        output.push_str("# TYPE omnilink_redis_response_time_ms gauge\n");
        output.push_str(&format!("omnilink_redis_response_time_ms {}\n", health.redis.response_time_ms));

        // 慢查询计数
        output.push_str("# HELP omnilink_slow_queries_total Total number of slow queries recorded\n");
        output.push_str("# TYPE omnilink_slow_queries_total counter\n");
        output.push_str(&format!("omnilink_slow_queries_total {}\n", slow_queries.len()));

        output
    }
}
