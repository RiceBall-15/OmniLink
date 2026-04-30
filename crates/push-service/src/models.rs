use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 推送消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushMessage {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_type: String, // 'ios', 'android', 'web'
    pub device_token: String,
    pub title: String,
    pub body: String,
    pub data: Option<serde_json::Value>,
    pub badge: Option<i32>,
    pub sound: Option<String>,
    pub priority: Option<i32>, // 1-10
    pub ttl: Option<i32>, // 生存时间(秒)
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    pub status: String, // 'pending', 'sent', 'failed'
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

/// 使用模板推送请求
#[derive(Debug, Deserialize)]
pub struct TemplatePushRequest {
    pub template_name: String,
    pub user_id: Uuid,
    pub device_type: String,
    pub device_token: String,
    pub variables: serde_json::Value, // 模板变量
}

/// 推送统计
#[derive(Debug, Serialize)]
pub struct PushStats {
    pub total_sent: i64,
    pub total_failed: i64,
    pub by_device_type: Vec<DeviceTypeStats>,
    pub by_date: Vec<DateStats>,
}

/// 设备类型统计
#[derive(Debug, Serialize)]
pub struct DeviceTypeStats {
    pub device_type: String,
    pub sent: i64,
    pub failed: i64,
}

/// 日期统计
#[derive(Debug, Serialize)]
pub struct DateStats {
    pub date: String,
    pub sent: i64,
    pub failed: i64,
}

/// APNs配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApnsConfig {
    pub team_id: String,
    pub key_id: String,
    pub private_key: String,
    pub bundle_id: String,
    pub use_sandbox: bool,
}

/// FCM配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcmConfig {
    pub server_key: String,
    pub sender_id: String,
}

/// 极光推送配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JpushConfig {
    pub app_key: String,
    pub master_secret: String,
}