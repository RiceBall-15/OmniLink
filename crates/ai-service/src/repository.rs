use common::models::Assistant;
use common::{AppError, Result};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use chrono::Utc;

/// AI助手仓库
pub struct AssistantRepository {
    pool: Pool<Postgres>,
}

impl AssistantRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// 根据ID查找助手
    pub async fn find_by_id(&self, assistant_id: Uuid) -> Result<Option<Assistant>> {
        let assistant = sqlx::query_as::<_, Assistant>(
            "SELECT * FROM assistants WHERE id = $1"
        )
        .bind(assistant_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(assistant)
    }

    /// 获取用户的所有助手
    pub async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Assistant>> {
        let assistants = sqlx::query_as::<_, Assistant>(
            "SELECT * FROM assistants WHERE created_by = $1 ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(assistants)
    }

    /// 创建助手
    pub async fn create(
        &self,
        assistant_id: Uuid,
        name: String,
        description: Option<String>,
        model_id: String,
        system_prompt: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<i32>,
        created_by: Uuid,
    ) -> Result<Assistant> {
        let now = Utc::now();

        sqlx::query_as::<_, Assistant>(
            r#"
            INSERT INTO assistants (id, name, description, model_id, system_prompt, temperature, max_tokens, created_by, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#
        )
        .bind(assistant_id)
        .bind(name)
        .bind(description)
        .bind(model_id)
        .bind(system_prompt)
        .bind(temperature)
        .bind(max_tokens)
        .bind(created_by)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e))
    }

    /// 更新助手
    pub async fn update(
        &self,
        assistant_id: Uuid,
        name: Option<String>,
        description: Option<String>,
        model_id: Option<String>,
        system_prompt: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<i32>,
    ) -> Result<Assistant> {
        let now = Utc::now();

        // 动态构建更新查询
        let mut query = String::from("UPDATE assistants SET updated_at = $1");
        let mut param_count = 1;

        if name.is_some() {
            param_count += 1;
            query.push_str(&format!(", name = ${}", param_count));
        }
        if description.is_some() {
            param_count += 1;
            query.push_str(&format!(", description = ${}", param_count));
        }
        if model_id.is_some() {
            param_count += 1;
            query.push_str(&format!(", model_id = ${}", param_count));
        }
        if system_prompt.is_some() {
            param_count += 1;
            query.push_str(&format!(", system_prompt = ${}", param_count));
        }
        if temperature.is_some() {
            param_count += 1;
            query.push_str(&format!(", temperature = ${}", param_count));
        }
        if max_tokens.is_some() {
            param_count += 1;
            query.push_str(&format!(", max_tokens = ${}", param_count));
        }

        param_count += 1;
        query.push_str(&format!(" WHERE id = ${} RETURNING *", param_count));

        let mut query_builder = sqlx::query_as::<_, Assistant>(&query);
        query_builder = query_builder.bind(now);

        if let Some(name) = name {
            query_builder = query_builder.bind(name);
        }
        if let Some(description) = description {
            query_builder = query_builder.bind(description);
        }
        if let Some(model_id) = model_id {
            query_builder = query_builder.bind(model_id);
        }
        if let Some(system_prompt) = system_prompt {
            query_builder = query_builder.bind(system_prompt);
        }
        if let Some(temperature) = temperature {
            query_builder = query_builder.bind(temperature);
        }
        if let Some(max_tokens) = max_tokens {
            query_builder = query_builder.bind(max_tokens);
        }
        query_builder = query_builder.bind(assistant_id);

        query_builder
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(e))
    }

    /// 删除助手
    pub async fn delete(&self, assistant_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM assistants WHERE id = $1")
            .bind(assistant_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e))?;

        Ok(())
    }
}

/// Token使用仓库
pub struct TokenUsageRepository {
    pool: Pool<Postgres>,
}

impl TokenUsageRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// 创建或更新Token使用记录
    pub async fn upsert(
        &self,
        user_id: Uuid,
        conversation_id: Option<Uuid>,
        model_id: String,
        request_count_delta: i32,
        prompt_tokens_delta: i32,
        completion_tokens_delta: i32,
        estimated_cost_delta: f64,
    ) -> Result<()> {
        let now = Utc::now();
        let today = now.format("%Y-%m-%d").to_string();

        sqlx::query(
            r#"
            INSERT INTO token_usage (user_id, conversation_id, model_id, request_count, prompt_tokens, completion_tokens, total_tokens, estimated_cost, date, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (user_id, model_id, date)
            DO UPDATE SET
                request_count = token_usage.request_count + EXCLUDED.request_count,
                prompt_tokens = token_usage.prompt_tokens + EXCLUDED.prompt_tokens,
                completion_tokens = token_usage.completion_tokens + EXCLUDED.completion_tokens,
                total_tokens = token_usage.total_tokens + EXCLUDED.total_tokens,
                estimated_cost = token_usage.estimated_cost + EXCLUDED.estimated_cost
            "#
        )
        .bind(user_id)
        .bind(conversation_id)
        .bind(&model_id)
        .bind(request_count_delta)
        .bind(prompt_tokens_delta)
        .bind(completion_tokens_delta)
        .bind(prompt_tokens_delta + completion_tokens_delta)
        .bind(estimated_cost_delta)
        .bind(&today)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e))?;

        Ok(())
    }

    /// 获取用户的使用统计
    pub async fn get_user_usage(
        &self,
        user_id: Uuid,
        start_date: Option<String>,
        end_date: Option<String>,
    ) -> Result<Vec<TokenUsage>> {
        let mut query = String::from(
            "SELECT model_id, SUM(request_count) as request_count, SUM(prompt_tokens) as prompt_tokens,
                    SUM(completion_tokens) as completion_tokens, SUM(total_tokens) as total_tokens,
                    SUM(estimated_cost) as estimated_cost
             FROM token_usage WHERE user_id = $1"
        );
        let mut param_count = 1;

        if start_date.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND date >= ${}", param_count));
        }
        if end_date.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND date <= ${}", param_count));
        }

        query.push_str(" GROUP BY model_id ORDER BY total_tokens DESC");

        let mut query_builder = sqlx::query_as::<_, TokenUsage>(&query);
        query_builder = query_builder.bind(user_id);

        if let Some(start_date) = start_date {
            query_builder = query_builder.bind(start_date);
        }
        if let Some(end_date) = end_date {
            query_builder = query_builder.bind(end_date);
        }

        query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(e))
    }

    /// 获取总使用统计
    pub async fn get_total_usage(&self, user_id: Uuid) -> Result<TokenUsageSummary> {
        let row = sqlx::query_as::<_, (i64, i64, i64, i64, f64)>(
            "SELECT COALESCE(SUM(request_count), 0) as total_requests,
                    COALESCE(SUM(prompt_tokens), 0) as total_prompt_tokens,
                    COALESCE(SUM(completion_tokens), 0) as total_completion_tokens,
                    COALESCE(SUM(total_tokens), 0) as total_tokens,
                    COALESCE(SUM(estimated_cost), 0) as total_cost
             FROM token_usage WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(TokenUsageSummary {
            request_count: row.0,
            prompt_tokens: row.1,
            completion_tokens: row.2,
            total_tokens: row.3,
            estimated_cost: row.4,
        })
    }
}

/// Token使用记录
#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
struct TokenUsage {
    model_id: String,
    request_count: i64,
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
    estimated_cost: f64,
}

/// Token使用汇总
#[derive(Debug, serde::Serialize)]
pub struct TokenUsageSummary {
    pub request_count: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub estimated_cost: f64,
}