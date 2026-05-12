use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 推送消息
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PushMessage {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_type: String,
    pub device_token: String,
    pub title: String,
    pub body: String,
    pub data: Option<serde_json::Value>,
    pub badge: Option<i32>,
    pub sound: Option<String>,
    pub priority: Option<i32>,
    pub ttl: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    pub status: String,
    pub error: Option<String>,
}

/// 创建推送请求
#[derive(Debug, Deserialize)]
pub struct CreatePushRequest {
    pub user_id: Uuid,
    pub device_type: String,
    pub device_token: String,
    pub title: String,
    pub body: String,
    pub data: Option<serde_json::Value>,
    pub badge: Option<i32>,
    pub sound: Option<String>,
    pub priority: Option<i32>,
    pub ttl: Option<i32>,
}

/// 批量推送请求
#[derive(Debug, Deserialize)]
pub struct BatchPushRequest {
    pub messages: Vec<CreatePushRequest>,
}

/// 批量推送响应
#[derive(Debug, Serialize)]
pub struct BatchPushResponse {
    pub succeeded: Vec<Uuid>,
    pub failed: Vec<Uuid>,
}

/// 模板推送请求
#[derive(Debug, Deserialize)]
pub struct TemplatePushRequest {
    pub user_id: Uuid,
    pub device_type: String,
    pub device_token: String,
    pub template_name: String,
    pub variables: serde_json::Value,
}

/// 推送模板
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PushTemplate {
    pub id: Uuid,
    pub name: String,
    pub title_template: String,
    pub body_template: String,
    pub data_template: Option<serde_json::Value>,
    pub sound: Option<String>,
    pub badge: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建模板请求
#[derive(Debug, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub title_template: String,
    pub body_template: String,
    pub data_template: Option<serde_json::Value>,
    pub sound: Option<String>,
    pub badge: Option<bool>,
}

/// 推送统计
#[derive(Debug, Serialize)]
pub struct PushStats {
    pub total_sent: i64,
    pub total_failed: i64,
    pub by_device_type: Vec<DeviceTypeStats>,
    pub by_date: Vec<DateStats>,
}

/// 按设备类型统计
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DeviceTypeStats {
    pub device_type: String,
    pub sent: i64,
    pub failed: i64,
}

/// 按日期统计
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DateStats {
    pub date: chrono::NaiveDate,
    pub sent: i64,
    pub failed: i64,
}

/// 设备注册请求
#[derive(Debug, Deserialize)]
pub struct RegisterDeviceRequest {
    pub device_type: String,
    pub device_token: String,
    pub device_name: Option<String>,
    pub app_version: Option<String>,
    pub os_version: Option<String>,
}

/// 设备信息
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DeviceInfo {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_type: String,
    pub device_token: String,
    pub device_name: Option<String>,
    pub app_version: Option<String>,
    pub os_version: Option<String>,
    pub is_active: bool,
    pub last_active_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// 推送通知偏好
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NotificationPreferences {
    pub user_id: Uuid,
    pub enable_notifications: bool,
    pub enable_message_notifications: bool,
    pub enable_system_notifications: bool,
    pub enable_promotional_notifications: bool,
    pub enable_reminder_notifications: bool,
    pub quiet_hours_start: Option<String>,
    pub quiet_hours_end: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            enable_notifications: true,
            enable_message_notifications: true,
            enable_system_notifications: true,
            enable_promotional_notifications: false,
            enable_reminder_notifications: true,
            quiet_hours_start: None,
            quiet_hours_end: None,
            updated_at: Utc::now(),
        }
    }
}
