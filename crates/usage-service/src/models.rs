use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Token使用记录
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TokenUsage {
    pub id: Uuid,
    pub user_id: Uuid,
    pub conversation_id: Option<Uuid>,
    pub model_name: String,
    pub provider: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub cost: f64,
    pub created_at: DateTime<Utc>,
}

/// 统计记录
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UsageStat {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub stat_type: String, // 'daily', 'weekly', 'monthly'
    pub model_name: Option<String>,
    pub provider: Option<String>,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub request_count: i64,
    pub stat_date: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// API调用记录
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ApiCall {
    pub id: Uuid,
    pub user_id: Uuid,
    pub api_endpoint: String,
    pub method: String,
    pub status_code: i32,
    pub response_time_ms: i32,
    pub created_at: DateTime<Utc>,
}

/// 创建Token使用记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTokenUsage {
    pub user_id: Uuid,
    pub conversation_id: Option<Uuid>,
    pub model_name: String,
    pub provider: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub cost: f64,
}

/// 创建API调用记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiCall {
    pub user_id: Uuid,
    pub api_endpoint: String,
    pub method: String,
    pub status_code: i32,
    pub response_time_ms: i32,
}

/// 统计查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageQuery {
    pub user_id: Option<Uuid>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub model_name: Option<String>,
    pub provider: Option<String>,
    pub stat_type: Option<String>,
}

/// 统计结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub total_tokens: i64,
    pub total_cost: f64,
    pub request_count: i64,
    pub by_model: Vec<ModelStats>,
    pub by_provider: Vec<ProviderStats>,
    pub by_date: Vec<DateStats>,
}

/// 模型统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStats {
    pub model_name: String,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub request_count: i64,
}

/// 提供商统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStats {
    pub provider: String,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub request_count: i64,
}

/// 日期统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateStats {
    pub date: String,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub request_count: i64,
}