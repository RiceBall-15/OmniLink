use sqlx::{Pool, Postgres, postgres::PgPoolOptions};
use redis::{Client as RedisClient, aio::ConnectionManager};
use anyhow::Result;

/// 数据库连接管理
pub struct DatabaseManager {
    pg_pool: Pool<Postgres>,
    redis: ConnectionManager,
}

impl DatabaseManager {
    pub async fn new(database_url: &str, redis_url: &str) -> Result<Self> {
        // PostgreSQL连接池
        let pg_pool = PgPoolOptions::new()
            .max_connections(100)
            .min_connections(10)
            .acquire_timeout(std::time::Duration::from_secs(30))
            .idle_timeout(std::time::Duration::from_secs(600))
            .connect(database_url)
            .await?;

        // Redis连接
        let redis_client = RedisClient::open(redis_url)?;
        let redis = ConnectionManager::new(redis_client).await?;

        Ok(Self { pg_pool, redis })
    }

    pub fn pg_pool(&self) -> &Pool<Postgres> {
        &self.pg_pool
    }

    pub fn redis(&self) -> &ConnectionManager {
        &self.redis
    }
}