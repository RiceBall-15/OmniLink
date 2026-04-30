use anyhow::Result;
use sqlx::PgPool;
use std::sync::Arc;

use super::models::*;
use super::repository::UsageRepository;

pub struct UsageService {
    repository: UsageRepository,
}

impl UsageService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            repository: UsageRepository::new(pool),
        }
    }

    /// 记录Token使用
    pub async fn record_token_usage(&self, data: CreateTokenUsage) -> Result<TokenUsage> {
        self.repository.create_token_usage(data).await
    }

    /// 记录API调用
    pub async fn record_api_call(&self, data: CreateApiCall) -> Result<ApiCall> {
        self.repository.create_api_call(data).await
    }

    /// 获取用户Token使用记录
    pub async fn get_token_usage(
        &self,
        user_id: Uuid,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<TokenUsage>> {
        let offset = (page - 1) * page_size;
        self.repository.get_token_usage(user_id, page_size, offset).await
    }

    /// 获取用户统计数据
    pub async fn get_user_stats(&self, query: UsageQuery) -> Result<UsageStats> {
        self.repository.get_user_stats(query).await
    }

    /// 获取API调用记录
    pub async fn get_api_calls(
        &self,
        user_id: Uuid,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<ApiCall>> {
        let offset = (page - 1) * page_size;
        self.repository.get_api_calls(user_id, page_size, offset).await
    }

    /// 清理过期记录
    pub async fn cleanup_old_records(&self, days: i64) -> Result<u64> {
        self.repository.cleanup_old_records(days).await
    }
}

// 成本计算工具
pub struct CostCalculator;

impl CostCalculator {
    /// 计算OpenAI费用
    pub fn calculate_openai_cost(model: &str, prompt_tokens: i32, completion_tokens: i32) -> f64 {
        let (prompt_price, completion_price) = match model {
            "gpt-4" => (0.03, 0.06),
            "gpt-4-32k" => (0.06, 0.12),
            "gpt-3.5-turbo" => (0.0015, 0.002),
            "gpt-3.5-turbo-16k" => (0.003, 0.004),
            "gpt-4-turbo" => (0.01, 0.03),
            "gpt-4o" => (0.005, 0.015),
            _ => (0.0015, 0.002), // 默认价格
        };

        let prompt_cost = (prompt_tokens as f64 / 1000.0) * prompt_price;
        let completion_cost = (completion_tokens as f64 / 1000.0) * completion_price;

        prompt_cost + completion_cost
    }

    /// 计算Anthropic费用
    pub fn calculate_anthropic_cost(model: &str, prompt_tokens: i32, completion_tokens: i32) -> f64 {
        let (prompt_price, completion_price) = match model {
            "claude-3-opus" => (0.015, 0.075),
            "claude-3-sonnet" => (0.003, 0.015),
            "claude-3-haiku" => (0.00025, 0.00125),
            "claude-2.1" => (0.008, 0.024),
            "claude-2.0" => (0.008, 0.024),
            _ => (0.003, 0.015), // 默认价格
        };

        let prompt_cost = (prompt_tokens as f64 / 1000.0) * prompt_price;
        let completion_cost = (completion_tokens as f64 / 1000.0) * completion_price;

        prompt_cost + completion_cost
    }

    /// 计算Google Gemini费用
    pub fn calculate_google_cost(model: &str, prompt_tokens: i32, completion_tokens: i32) -> f64 {
        let (prompt_price, completion_price) = match model {
            "gemini-1.5-pro" => (0.0035, 0.0105),
            "gemini-1.5-flash" => (0.000075, 0.0003),
            "gemini-1.0-pro" => (0.0005, 0.0015),
            "gemini-pro-vision" => (0.00025, 0.0005),
            _ => (0.0005, 0.0015), // 默认价格
        };

        let prompt_cost = (prompt_tokens as f64 / 1000.0) * prompt_price;
        let completion_cost = (completion_tokens as f64 / 1000.0) * completion_price;

        prompt_cost + completion_cost
    }

    /// 统一计算费用
    pub fn calculate_cost(
        provider: &str,
        model: &str,
        prompt_tokens: i32,
        completion_tokens: i32,
    ) -> f64 {
        match provider.to_lowercase().as_str() {
            "openai" => Self::calculate_openai_cost(model, prompt_tokens, completion_tokens),
            "anthropic" => Self::calculate_anthropic_cost(model, prompt_tokens, completion_tokens),
            "google" => Self::calculate_google_cost(model, prompt_tokens, completion_tokens),
            _ => 0.0,
        }
    }
}