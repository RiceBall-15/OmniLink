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
use crate::services::ConfigService;

pub struct AppState {
    pub config_service: Arc<ConfigService>,
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

/// 获取配置项
pub async fn get_config(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    match state.config_service.get_config(&key).await {
        Ok(Some(value)) => Ok(Json(ApiResponse::success(value))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to get config: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 设置配置项（需要管理员权限）
pub async fn set_config(
    State(state): State<AppState>,
    claims: Claims,
    Path(key): Path<String>,
    Json(req): Json<CreateConfigRequest>,
) -> Result<Json<ApiResponse<ConfigItem>>, StatusCode> {
    match state
        .config_service
        .set_config(&key, &req.value, Some(claims.user_id))
        .await
    {
        Ok(config) => Ok(Json(ApiResponse::success(config))),
        Err(e) => {
            tracing::error!("Failed to set config: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 删除配置项（需要管理员权限）
pub async fn delete_config(
    State(state): State<AppState>,
    claims: Claims,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<bool>>, StatusCode> {
    match state.config_service.delete_config(&key).await {
        Ok(deleted) => {
            if deleted {
                Ok(Json(ApiResponse::success(true)))
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            tracing::error!("Failed to delete config: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取所有配置
pub async fn list_configs(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<ConfigItem>>>, StatusCode> {
    match state.config_service.list_configs().await {
        Ok(configs) => Ok(Json(ApiResponse::success(configs))),
        Err(e) => {
            tracing::error!("Failed to list configs: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 批量获取配置
pub async fn batch_get_configs(
    State(state): State<AppState>,
    Json(req): Json<BatchConfigQuery>,
) -> Result<Json<ApiResponse<BatchConfigResponse>>, StatusCode> {
    match state
        .config_service
        .batch_get_configs(&req.keys)
        .await
    {
        Ok(response) => Ok(Json(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("Failed to batch get configs: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取配置历史
pub async fn get_config_history(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(params): Query<HistoryParams>,
) -> Result<Json<ApiResponse<Vec<ConfigHistory>>>, StatusCode> {
    let limit = params.limit.unwrap_or(50);

    match state
        .config_service
        .get_config_history(&key, limit)
        .await
    {
        Ok(history) => Ok(Json(ApiResponse::success(history))),
        Err(e) => {
            tracing::error!("Failed to get config history: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct HistoryParams {
    pub limit: Option<i64>,
}

/// 恢复配置版本（需要管理员权限）
pub async fn restore_config_version(
    State(state): State<AppState>,
    claims: Claims,
    Path((key, version)): Path<(String, i32)>,
) -> Result<Json<ApiResponse<ConfigItem>>, StatusCode> {
    match state
        .config_service
        .restore_config_version(&key, version, Some(claims.user_id))
        .await
    {
        Ok(Some(config)) => Ok(Json(ApiResponse::success(config))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to restore config version: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 添加配置订阅
pub async fn add_subscription(
    State(state): State<AppState>,
    Json(req): Json<CreateSubscriptionRequest>,
) -> Result<Json<ApiResponse<ConfigSubscription>>, StatusCode> {
    match state.config_service.add_subscription(req).await {
        Ok(subscription) => Ok(Json(ApiResponse::success(subscription))),
        Err(e) => {
            tracing::error!("Failed to add subscription: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取配置订阅
pub async fn get_subscriptions(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<Vec<ConfigSubscription>>>, StatusCode> {
    match state.config_service.get_subscriptions(&key).await {
        Ok(subs) => Ok(Json(ApiResponse::success(subs))),
        Err(e) => {
            tracing::error!("Failed to get subscriptions: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 删除订阅
pub async fn remove_subscription(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<bool>>, StatusCode> {
    match state.config_service.remove_subscription(id).await {
        Ok(removed) => {
            if removed {
                Ok(Json(ApiResponse::success(true)))
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            tracing::error!("Failed to remove subscription: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 预热缓存
pub async fn warmup_cache(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    match state.config_service.warmup_cache().await {
        Ok(_) => Ok(Json(ApiResponse::success("Cache warmed up".to_string()))),
        Err(e) => {
            tracing::error!("Failed to warmup cache: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 健康检查
pub async fn health_check() -> &'static str {
    "Config service is healthy"
}