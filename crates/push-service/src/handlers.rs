use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::models::*;
use crate::services::PushService;

#[derive(Clone)]
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

    pub fn err(message: String) -> Self {
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

fn default_page() -> i64 {
    1
}
fn default_page_size() -> i64 {
    20
}

/// 统计查询参数
#[derive(Debug, Deserialize)]
pub struct StatsQueryParams {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

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
    Query(params): Query<PaginationParams>,
) -> Result<Json<ApiResponse<Vec<PushMessage>>>, StatusCode> {
    // Note: In production, user_id should come from JWT claims
    // For now, we accept it as a query parameter
    let user_id = Uuid::nil();
    match state
        .push_service
        .get_user_push_history(user_id, params.page, params.page_size)
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

// ==================== 设备管理 ====================

/// 注册设备
pub async fn register_device(
    State(state): State<AppState>,
    Json(req): Json<RegisterDeviceRequest>,
) -> Result<Json<ApiResponse<DeviceInfo>>, StatusCode> {
    let device_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let user_id = Uuid::nil(); // In production, from JWT
    
    let device = DeviceInfo {
        id: device_id,
        user_id,
        device_type: req.device_type,
        device_token: req.device_token,
        device_name: req.device_name,
        app_version: req.app_version,
        os_version: req.os_version,
        is_active: true,
        last_active_at: now,
        created_at: now,
    };
    
    match state.push_service.repository.create_device(device).await {
        Ok(device) => Ok(Json(ApiResponse::success(device))),
        Err(e) => {
            tracing::error!("Failed to register device: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取用户设备列表
pub async fn get_user_devices(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<DeviceInfo>>>, StatusCode> {
    let user_id = Uuid::nil(); // In production, from JWT
    match state.push_service.repository.get_user_devices(user_id).await {
        Ok(devices) => Ok(Json(ApiResponse::success(devices))),
        Err(e) => {
            tracing::error!("Failed to get user devices: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 注销设备
pub async fn unregister_device(
    State(state): State<AppState>,
    Path(device_id): Path<Uuid>,
) -> Result<Json<ApiResponse<bool>>, StatusCode> {
    match state.push_service.repository.delete_device(device_id).await {
        Ok(deleted) => {
            if deleted {
                Ok(Json(ApiResponse::success(true)))
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            tracing::error!("Failed to unregister device: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// ==================== 通知偏好 ====================

/// 获取通知偏好
pub async fn get_notification_preferences(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<NotificationPreferences>>, StatusCode> {
    let user_id = Uuid::nil(); // In production, from JWT
    match state.push_service.repository.get_notification_preferences(user_id).await {
        Ok(prefs) => Ok(Json(ApiResponse::success(prefs))),
        Err(e) => {
            tracing::error!("Failed to get notification preferences: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 更新通知偏好
pub async fn update_notification_preferences(
    State(state): State<AppState>,
    Json(req): Json<UpdateNotificationPreferencesRequest>,
) -> Result<Json<ApiResponse<NotificationPreferences>>, StatusCode> {
    let user_id = Uuid::nil(); // In production, from JWT
    
    // Get current preferences first
    let current = match state.push_service.repository.get_notification_preferences(user_id).await {
        Ok(prefs) => prefs,
        Err(e) => {
            tracing::error!("Failed to get current preferences: {:?}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    
    // Apply updates
    let mut prefs = current;
    if let Some(v) = req.enable_notifications {
        prefs.enable_notifications = v;
    }
    if let Some(v) = req.enable_message_notifications {
        prefs.enable_message_notifications = v;
    }
    if let Some(v) = req.enable_system_notifications {
        prefs.enable_system_notifications = v;
    }
    if let Some(v) = req.enable_promotional_notifications {
        prefs.enable_promotional_notifications = v;
    }
    if let Some(v) = req.enable_reminder_notifications {
        prefs.enable_reminder_notifications = v;
    }
    prefs.quiet_hours_start = req.quiet_hours_start;
    prefs.quiet_hours_end = req.quiet_hours_end;
    prefs.updated_at = chrono::Utc::now();
    
    match state.push_service.repository.upsert_notification_preferences(prefs).await {
        Ok(updated) => Ok(Json(ApiResponse::success(updated))),
        Err(e) => {
            tracing::error!("Failed to update notification preferences: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// ==================== 推送配置管理 ====================

/// 获取所有推送配置
pub async fn get_push_configs(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<PushConfigItem>>>, StatusCode> {
    match state.push_service.repository.get_push_configs().await {
        Ok(configs) => Ok(Json(ApiResponse::success(configs))),
        Err(e) => {
            tracing::error!("Failed to get push configs: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 创建/更新推送配置
pub async fn upsert_push_config(
    State(state): State<AppState>,
    Json(req): Json<UpsertPushConfigRequest>,
) -> Result<Json<ApiResponse<PushConfigItem>>, StatusCode> {
    let now = chrono::Utc::now();
    let config = PushConfigItem {
        id: Uuid::new_v4(),
        config_key: req.config_key,
        config_value: req.config_value,
        description: req.description,
        created_at: now,
        updated_at: now,
    };
    
    match state.push_service.repository.upsert_push_config(config).await {
        Ok(updated) => Ok(Json(ApiResponse::success(updated))),
        Err(e) => {
            tracing::error!("Failed to upsert push config: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 删除推送配置
pub async fn delete_push_config(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<bool>>, StatusCode> {
    match state.push_service.repository.delete_push_config(&key).await {
        Ok(deleted) => {
            if deleted {
                Ok(Json(ApiResponse::success(true)))
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            tracing::error!("Failed to delete push config: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// ==================== 增强监控 ====================

/// 获取推送健康状态
pub async fn get_push_health(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<PushHealthStatus>>, StatusCode> {
    match state.push_service.repository.get_push_health().await {
        Ok(health) => Ok(Json(ApiResponse::success(health))),
        Err(e) => {
            tracing::error!("Failed to get push health: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 发送测试推送
pub async fn send_test_push(
    State(state): State<AppState>,
    Json(req): Json<CreatePushRequest>,
) -> Result<Json<ApiResponse<PushMessage>>, StatusCode> {
    tracing::info!("Sending test push to device: {}", req.device_token);
    match state.push_service.send_push(req).await {
        Ok(message) => Ok(Json(ApiResponse::success(message))),
        Err(e) => {
            tracing::error!("Failed to send test push: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
