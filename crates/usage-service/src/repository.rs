use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use super::models::*;

pub struct UsageRepository {
    pool: PgPool,
}

impl UsageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 创建Token使用记录
    pub async fn create_token_usage(&self, data: CreateTokenUsage) -> Result<TokenUsage> {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now();

        let usage = sqlx::query_as::<_, TokenUsage>(
            r#"
            INSERT INTO token_usage (id, user_id, conversation_id, model_name, provider,
                                   prompt_tokens, completion_tokens, total_tokens, cost, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#
        )
        .bind(id)
        .bind(&data.user_id)
        .bind(&data.conversation_id)
        .bind(&data.model_name)
        .bind(&data.provider)
        .bind(data.prompt_tokens)
        .bind(data.completion_tokens)
        .bind(data.total_tokens)
        .bind(data.cost)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(usage)
    }

    /// 创建API调用记录
    pub async fn create_api_call(&self, data: CreateApiCall) -> Result<ApiCall> {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now();

        let call = sqlx::query_as::<_, ApiCall>(
            r#"
            INSERT INTO api_call (id, user_id, api_endpoint, method, status_code, response_time_ms, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#
        )
        .bind(id)
        .bind(&data.user_id)
        .bind(&data.api_endpoint)
        .bind(&data.method)
        .bind(data.status_code)
        .bind(data.response_time_ms)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(call)
    }

    /// 查询用户的Token使用记录
    pub async fn get_token_usage(&self, user_id: Uuid, limit: i64, offset: i64) -> Result<Vec<TokenUsage>> {
        let usages = sqlx::query_as::<_, TokenUsage>(
            r#"
            SELECT * FROM token_usage
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(usages)
    }

    /// 获取用户统计
    pub async fn get_user_stats(&self, query: UsageQuery) -> Result<UsageStats> {
        // 基础查询
        let (total_tokens, total_cost, request_count): (i64, f64, i64) = sqlx::query_as(
            r#"
            SELECT
                COALESCE(SUM(total_tokens), 0) as total_tokens,
                COALESCE(SUM(cost), 0) as total_cost,
                COALESCE(COUNT(*), 0) as request_count
            FROM token_usage
            WHERE ($1::uuid IS NULL OR user_id = $1)
              AND ($2::timestamptz IS NULL OR created_at >= $2)
              AND ($3::timestamptz IS NULL OR created_at <= $3)
              AND ($4::text IS NULL OR model_name = $4)
              AND ($5::text IS NULL OR provider = $5)
            "#
        )
        .bind(query.user_id)
        .bind(query.start_date)
        .bind(query.end_date)
        .bind(&query.model_name)
        .bind(&query.provider)
        .fetch_one(&self.pool)
        .await?;

        // 按模型统计
        let by_model = sqlx::query_as::<_, ModelStats>(
            r#"
            SELECT
                model_name,
                SUM(total_tokens) as total_tokens,
                SUM(cost) as total_cost,
                COUNT(*) as request_count
            FROM token_usage
            WHERE ($1::uuid IS NULL OR user_id = $1)
              AND ($2::timestamptz IS NULL OR created_at >= $2)
              AND ($3::timestamptz IS NULL OR created_at <= $3)
              AND ($4::text IS NULL OR model_name = $4)
              AND ($5::text IS NULL OR provider = $5)
            GROUP BY model_name
            ORDER BY total_tokens DESC
            "#
        )
        .bind(query.user_id)
        .bind(query.start_date)
        .bind(query.end_date)
        .bind(&query.model_name)
        .bind(&query.provider)
        .fetch_all(&self.pool)
        .await?;

        // 按提供商统计
        let by_provider = sqlx::query_as::<_, ProviderStats>(
            r#"
            SELECT
                provider,
                SUM(total_tokens) as total_tokens,
                SUM(cost) as total_cost,
                COUNT(*) as request_count
            FROM token_usage
            WHERE ($1::uuid IS NULL OR user_id = $1)
              AND ($2::timestamptz IS NULL OR created_at >= $2)
              AND ($3::timestamptz IS NULL OR created_at <= $3)
              AND ($4::text IS NULL OR model_name = $4)
              AND ($5::text IS NULL OR provider = $5)
            GROUP BY provider
            ORDER BY total_tokens DESC
            "#
        )
        .bind(query.user_id)
        .bind(query.start_date)
        .bind(query.end_date)
        .bind(&query.model_name)
        .bind(&query.provider)
        .fetch_all(&self.pool)
        .await?;

        // 按日期统计
        let by_date = sqlx::query_as::<_, DateStats>(
            r#"
            SELECT
                DATE(created_at) as date,
                SUM(total_tokens) as total_tokens,
                SUM(cost) as total_cost,
                COUNT(*) as request_count
            FROM token_usage
            WHERE ($1::uuid IS NULL OR user_id = $1)
              AND ($2::timestamptz IS NULL OR created_at >= $2)
              AND ($3::timestamptz IS NULL OR created_at <= $3)
              AND ($4::text IS NULL OR model_name = $4)
              AND ($5::text IS NULL OR provider = $5)
            GROUP BY DATE(created_at)
            ORDER BY date DESC
            LIMIT 30
            "#
        )
        .bind(query.user_id)
        .bind(query.start_date)
        .bind(query.end_date)
        .bind(&query.model_name)
        .bind(&query.provider)
        .fetch_all(&self.pool)
        .await?;

        Ok(UsageStats {
            total_tokens,
            total_cost,
            request_count,
            by_model,
            by_provider,
            by_date,
        })
    }

    /// 获取API调用记录
    pub async fn get_api_calls(&self, user_id: Uuid, limit: i64, offset: i64) -> Result<Vec<ApiCall>> {
        let calls = sqlx::query_as::<_, ApiCall>(
            r#"
            SELECT * FROM api_call
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(calls)
    }

    /// 删除过期记录
    pub async fn cleanup_old_records(&self, days: i64) -> Result<u64> {
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(days);

        let result = sqlx::query(
            r#"
            DELETE FROM token_usage
            WHERE created_at < $1
            "#
        )
        .bind(cutoff_date)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}