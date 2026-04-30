use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use common::auth::Claims;
use crate::models::*;
use crate::services::{UsageService, CostCalculator};

pub struct AppState {
    pub usage_service: Arc<UsageService>,
}

/// 记录Token使用请求
#[derive(Debug, Deserialize)]
pub struct RecordTokenUsageRequest {
    pub conversation_id: Option<Uuid>,
    pub model_name: String,
    pub provider: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
}

/// 响应包装器
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

/// 分页查询参数
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

fn default_page() -> i64 { 1 }
fn default_page_size() -> i64 { 20 }

/// 记录Token使用
pub async fn record_token_usage(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<RecordTokenUsageRequest>,
) -> Result<Json<ApiResponse<TokenUsage>>, StatusCode> {
    let total_tokens = req.prompt_tokens + req.completion_tokens;

    // 自动计算费用
    let cost = CostCalculator::calculate_cost(
        &req.provider,
        &req.model_name,
        req.prompt_tokens,
        req.completion_tokens,
    );

    let data = CreateTokenUsage {
        user_id: claims.user_id,
        conversation_id: req.conversation_id,
        model_name: req.model_name,
        provider: req.provider,
        prompt_tokens: req.prompt_tokens,
        completion_tokens: req.completion_tokens,
        total_tokens,
        cost,
    };

    match state.usage_service.record_token_usage(data).await {
        Ok(usage) => Ok(Json(ApiResponse::success(usage))),
        Err(e) => {
            tracing::error!("Failed to record token usage: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取Token使用记录
pub async fn get_token_usage(
    State(state): State<AppState>,
    claims: Claims,
    Query(params): Query<PaginationParams>,
) -> Result<Json<ApiResponse<Vec<TokenUsage>>>, StatusCode> {
    match state
        .usage_service
        .get_token_usage(claims.user_id, params.page, params.page_size)
        .await
    {
        Ok(usages) => Ok(Json(ApiResponse::success(usages))),
        Err(e) => {
            tracing::error!("Failed to get token usage: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 统计查询参数
#[derive(Debug, Deserialize)]
pub struct StatsQueryParams {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub model_name: Option<String>,
    pub provider: Option<String>,
}

/// 获取用户统计数据
pub async fn get_user_stats(
    State(state): State<AppState>,
    claims: Claims,
    Query(params): Query<StatsQueryParams>,
) -> Result<Json<ApiResponse<UsageStats>>, StatusCode> {
    // 解析日期参数
    let start_date = params.start_date.and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    });

    let end_date = params.end_date.and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    });

    let query = UsageQuery {
        user_id: Some(claims.user_id),
        start_date,
        end_date,
        model_name: params.model_name,
        provider: params.provider,
        stat_type: None,
    };

    match state.usage_service.get_user_stats(query).await {
        Ok(stats) => Ok(Json(ApiResponse::success(stats))),
        Err(e) => {
            tracing::error!("Failed to get user stats: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取API调用记录
pub async fn get_api_calls(
    State(state): State<AppState>,
    claims: Claims,
    Query(params): Query<PaginationParams>,
) -> Result<Json<ApiResponse<Vec<ApiCall>>>, StatusCode> {
    match state
        .usage_service
        .get_api_calls(claims.user_id, params.page, params.page_size)
        .await
    {
        Ok(calls) => Ok(Json(ApiResponse::success(calls))),
        Err(e) => {
            tracing::error!("Failed to get api calls: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 计算费用（内部使用，不暴露给用户）
pub async fn calculate_cost(
    Json(req): Json<RecordTokenUsageRequest>,
) -> Json<ApiResponse<f64>> {
    let cost = CostCalculator::calculate_cost(
        &req.provider,
        &req.model_name,
        req.prompt_tokens,
        req.completion_tokens,
    );

    Json(ApiResponse::success(cost))
}

/// 清理过期记录（管理员功能）
pub async fn cleanup_old_records(
    State(state): State<AppState>,
    Path(days): Path<i64>,
) -> Result<Json<ApiResponse<u64>>, StatusCode> {
    match state.usage_service.cleanup_old_records(days).await {
        Ok(count) => Ok(Json(ApiResponse::success(count))),
        Err(e) => {
            tracing::error!("Failed to cleanup old records: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 健康检查
pub async fn health_check() -> &'static str {
    "Usage service is healthy"
}