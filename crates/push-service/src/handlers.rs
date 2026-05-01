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
use crate::services::PushService;

pub struct AppState {
    pub push_service: Arc<PushService>,
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

/// 发送单条推送
pub async fn send_push(
    State(state): State<AppState>,
    Json(req): Json<CreatePushRequest>,
) -> Result<Json<ApiResponse<PushMessage>>, StatusCode> {
    match state.push_service.send_push(req).await {
        Ok(message) => Ok(Json(ApiResponse::success(message))),
        Err(e) => {
            tracing::error!("Failed to send push: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 批量发送推送
pub async fn batch_send_push(
    State(state): State<AppState>,
    Json(req): Json<BatchPushRequest>,
) -> Result<Json<ApiResponse<BatchPushResponse>>, StatusCode> {
    match state.push_service.batch_send_push(req).await {
        Ok(response) => Ok(Json(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("Failed to batch send push: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 使用模板发送推送
pub async fn send_template_push(
    State(state): State<AppState>,
    Json(req): Json<TemplatePushRequest>,
) -> Result<Json<ApiResponse<PushMessage>>, StatusCode> {
    match state.push_service.send_template_push(req).await {
        Ok(message) => Ok(Json(ApiResponse::success(message))),
        Err(e) => {
            tracing::error!("Failed to send template push: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取用户推送历史
pub async fn get_user_push_history(
    State(state): State<AppState>,
    claims: Claims,
    Query(params): Query<PaginationParams>,
) -> Result<Json<ApiResponse<Vec<PushMessage>>>, StatusCode> {
    match state
        .push_service
        .get_user_push_history(claims.user_id, params.page, params.page_size)
        .await
    {
        Ok(messages) => Ok(Json(ApiResponse::success(messages))),
        Err(e) => {
            tracing::error!("Failed to get push history: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 创建推送模板
pub async fn create_template(
    State(state): State<AppState>,
    Json(req): Json<CreateTemplateRequest>,
) -> Result<Json<ApiResponse<PushTemplate>>, StatusCode> {
    match state.push_service.create_template(req).await {
        Ok(template) => Ok(Json(ApiResponse::success(template))),
        Err(e) => {
            tracing::error!("Failed to create template: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取所有推送模板
pub async fn list_templates(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<PushTemplate>>>, StatusCode> {
    match state.push_service.list_templates().await {
        Ok(templates) => Ok(Json(ApiResponse::success(templates))),
        Err(e) => {
            tracing::error!("Failed to list templates: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 删除推送模板
pub async fn delete_template(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<bool>>, StatusCode> {
    match state.push_service.delete_template(&name).await {
        Ok(deleted) => {
            if deleted {
                Ok(Json(ApiResponse::success(true)))
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            tracing::error!("Failed to delete template: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取推送统计
pub async fn get_push_stats(
    State(state): State<AppState>,
    Query(params): Query<StatsQueryParams>,
) -> Result<Json<ApiResponse<PushStats>>, StatusCode> {
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

    match state
        .push_service
        .get_push_stats(start_date, end_date)
        .await
    {
        Ok(stats) => Ok(Json(ApiResponse::success(stats))),
        Err(e) => {
            tracing::error!("Failed to get push stats: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StatsQueryParams {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// 清理过期推送记录
pub async fn cleanup_old_messages(
    State(state): State<AppState>,
    Path(days): Path<i64>,
) -> Result<Json<ApiResponse<u64>>, StatusCode> {
    match state.push_service.cleanup_old_messages(days).await {
        Ok(count) => Ok(Json(ApiResponse::success(count))),
        Err(e) => {
            tracing::error!("Failed to cleanup old messages: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 健康检查
pub async fn health_check() -> &'static str {
    "Push service is healthy"
}