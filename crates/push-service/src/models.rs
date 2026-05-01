use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 设备类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    iOS,
    Android,
    Web,
}

/// 推送通知类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NotificationType {
    Message,
    System,
    Promotional,
    Reminder,
}

/// 推送通知优先级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NotificationPriority {
    High,
    Normal,
    Low,
}

/// 注册设备请求
#[derive(Debug, Deserialize)]
pub struct RegisterDeviceRequest {
    pub device_type: DeviceType,
    pub device_token: String,
    pub device_name: Option<String>,
    pub app_version: Option<String>,
    pub os_version: Option<String>,
}

/// 注册设备响应
#[derive(Debug, Serialize)]
pub struct RegisterDeviceResponse {
    pub device_id: Uuid,
    pub device_token: String,
}

/// 设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_type: DeviceType,
    pub device_token: String,
    pub device_name: Option<String>,
    pub app_version: Option<String>,
    pub os_version: Option<String>,
    pub is_active: bool,
    pub last_active_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// 更新设备请求
#[derive(Debug, Deserialize)]
pub struct UpdateDeviceRequest {
    pub device_name: Option<String>,
    pub is_active: Option<bool>,
}

/// 发送推送通知请求
#[derive(Debug, Deserialize)]
pub struct SendNotificationRequest {
    pub user_id: Option<Uuid>,
    pub device_id: Option<Uuid>,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub notification_type: NotificationType,
    #[serde(default)]
    pub priority: NotificationPriority,
    pub data: Option<serde_json::Value>,
    pub badge: Option<i32>,
    pub sound: Option<String>,
    pub url: Option<String>,
}

/// 发送推送通知响应
#[derive(Debug, Serialize)]
pub struct SendNotificationResponse {
    pub notification_id: Uuid,
    pub sent_count: usize,
    pub failed_count: usize,
}

/// 批量发送推送通知请求
#[derive(Debug, Deserialize)]
pub struct BatchSendNotificationRequest {
    pub notifications: Vec<SendNotificationRequest>,
}

/// 批量发送推送通知响应
#[derive(Debug, Serialize)]
pub struct BatchSendNotificationResponse {
    pub notification_ids: Vec<Uuid>,
    pub total_sent: usize,
    pub total_failed: usize,
}

/// 推送通知偏好
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub user_id: Uuid,
    pub enable_notifications: bool,
    pub enable_message_notifications: bool,
    pub enable_system_notifications: bool,
    pub enable_promotional_notifications: bool,
    pub enable_reminder_notifications: bool,
    pub quiet_hours_start: Option<String>, // HH:MM format
    pub quiet_hours_end: Option<String>,   // HH:MM format
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

/// 推送通知历史查询参数
#[derive(Debug, Deserialize)]
pub struct NotificationHistoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub notification_type: Option<NotificationType>,
}

/// 推送通知历史响应
#[derive(Debug, Serialize)]
pub struct NotificationHistoryResponse {
    pub notifications: Vec<NotificationRecord>,
    pub total: i64,
}

/// 推送通知记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub body: String,
    pub notification_type: NotificationType,
    pub priority: NotificationPriority,
    pub data: Option<serde_json::Value>,
    pub is_read: bool,
    pub sent_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}

/// 批量标记已读请求
#[derive(Debug, Deserialize)]
pub struct BatchMarkReadRequest {
    pub notification_ids: Vec<Uuid>,
}

/// 批量标记已读响应
#[derive(Debug, Serialize)]
pub struct BatchMarkReadResponse {
    pub marked_count: usize,
}

/// 日期范围查询
#[derive(Debug, Deserialize)]
pub struct DateRangeQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// 推送统计信息
#[derive(Debug, Serialize)]
pub struct PushStats {
    pub total_sent: i64,
    pub total_delivered: i64,
    pub total_failed: i64,
    pub total_opened: i64,
    pub delivery_rate: f64,
    pub open_rate: f64,
    pub stats_by_type: Vec<NotificationTypeStats>,
}

/// 按类型统计的推送信息
#[derive(Debug, Serialize)]
pub struct NotificationTypeStats {
    pub notification_type: NotificationType,
    pub count: i64,
    pub percentage: f64,
}

/// 测试推送请求
#[derive(Debug, Deserialize)]
pub struct TestPushRequest {
    pub device_id: Option<Uuid>,
    pub title: Option<String>,
    pub body: Option<String>,
}