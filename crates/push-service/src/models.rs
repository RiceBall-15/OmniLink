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

/// 推送配置项
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PushConfigItem {
    pub id: Uuid,
    pub config_key: String,
    pub config_value: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建/更新推送配置请求
#[derive(Debug, Deserialize)]
pub struct UpsertPushConfigRequest {
    pub config_key: String,
    pub config_value: String,
    pub description: Option<String>,
}

/// 更新通知偏好请求
#[derive(Debug, Deserialize)]
pub struct UpdateNotificationPreferencesRequest {
    pub enable_notifications: Option<bool>,
    pub enable_message_notifications: Option<bool>,
    pub enable_system_notifications: Option<bool>,
    pub enable_promotional_notifications: Option<bool>,
    pub enable_reminder_notifications: Option<bool>,
    pub quiet_hours_start: Option<String>,
    pub quiet_hours_end: Option<String>,
}

/// 推送健康状态
#[derive(Debug, Serialize)]
pub struct PushHealthStatus {
    pub total_devices: i64,
    pub active_devices: i64,
    pub devices_by_type: Vec<DeviceTypeCount>,
    pub recent_failures: i64,
    pub success_rate: f64,
}

/// 按设备类型计数
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DeviceTypeCount {
    pub device_type: String,
    pub count: i64,
}

/// 设备类型常量
pub mod device_types {
    pub const IOS: &str = "ios";
    pub const ANDROID: &str = "android";
    pub const WEB: &str = "web";
    pub const DESKTOP: &str = "desktop";

    pub fn is_valid(device_type: &str) -> bool {
        matches!(device_type, IOS | ANDROID | WEB | DESKTOP)
    }
}

/// 推送状态常量
pub mod push_status {
    pub const PENDING: &str = "pending";
    pub const SENT: &str = "sent";
    pub const FAILED: &str = "failed";
    pub const DELIVERED: &str = "delivered";
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    // === 设备类型验证测试 ===

    #[test]
    fn test_valid_device_types() {
        assert!(device_types::is_valid("ios"));
        assert!(device_types::is_valid("android"));
        assert!(device_types::is_valid("web"));
        assert!(device_types::is_valid("desktop"));
    }

    #[test]
    fn test_invalid_device_types() {
        assert!(!device_types::is_valid("windows_phone"));
        assert!(!device_types::is_valid("unknown"));
        assert!(!device_types::is_valid(""));
    }

    // === CreatePushRequest 序列化测试 ===

    #[test]
    fn test_create_push_request_deserialization() {
        let json = r#"{
            "user_id": "550e8400-e29b-41d4-a716-446655440000",
            "device_type": "ios",
            "device_token": "abc123",
            "title": "Test",
            "body": "Hello"
        }"#;

        let request: CreatePushRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.device_type, "ios");
        assert_eq!(request.title, "Test");
        assert!(request.data.is_none());
        assert!(request.badge.is_none());
    }

    #[test]
    fn test_create_push_request_with_optional_fields() {
        let json = r#"{
            "user_id": "550e8400-e29b-41d4-a716-446655440000",
            "device_type": "android",
            "device_token": "xyz789",
            "title": "Alert",
            "body": "World",
            "data": {"key": "value"},
            "badge": 5,
            "sound": "default",
            "priority": 10,
            "ttl": 3600
        }"#;

        let request: CreatePushRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.badge, Some(5));
        assert_eq!(request.sound, Some("default".to_string()));
        assert_eq!(request.priority, Some(10));
        assert_eq!(request.ttl, Some(3600));
        assert!(request.data.is_some());
    }

    // === BatchPushRequest 测试 ===

    #[test]
    fn test_batch_push_request_deserialization() {
        let json = r#"{
            "messages": [
                {
                    "user_id": "550e8400-e29b-41d4-a716-446655440000",
                    "device_type": "ios",
                    "device_token": "tok1",
                    "title": "T1",
                    "body": "B1"
                },
                {
                    "user_id": "550e8400-e29b-41d4-a716-446655440001",
                    "device_type": "android",
                    "device_token": "tok2",
                    "title": "T2",
                    "body": "B2"
                }
            ]
        }"#;

        let request: BatchPushRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.messages[0].device_type, "ios");
        assert_eq!(request.messages[1].device_type, "android");
    }

    #[test]
    fn test_batch_push_response_serialization() {
        let response = BatchPushResponse {
            succeeded: vec![Uuid::new_v4()],
            failed: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("succeeded"));
        assert!(json.contains("failed"));
    }

    // === TemplatePushRequest 测试 ===

    #[test]
    fn test_template_push_request_deserialization() {
        let json = r#"{
            "user_id": "550e8400-e29b-41d4-a716-446655440000",
            "device_type": "web",
            "device_token": "webtok",
            "template_name": "welcome",
            "variables": {"name": "Alice"}
        }"#;

        let request: TemplatePushRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.template_name, "welcome");
        assert_eq!(request.variables["name"], "Alice");
    }

    // === RegisterDeviceRequest 测试 ===

    #[test]
    fn test_register_device_request_deserialization() {
        let json = r#"{
            "device_type": "ios",
            "device_token": "apns_token_123",
            "device_name": "iPhone 15",
            "app_version": "1.0.0",
            "os_version": "17.0"
        }"#;

        let request: RegisterDeviceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.device_type, "ios");
        assert_eq!(request.device_name, Some("iPhone 15".to_string()));
        assert_eq!(request.app_version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_register_device_request_minimal() {
        let json = r#"{
            "device_type": "android",
            "device_token": "fcm_token_456"
        }"#;

        let request: RegisterDeviceRequest = serde_json::from_str(json).unwrap();
        assert!(request.device_name.is_none());
        assert!(request.app_version.is_none());
        assert!(request.os_version.is_none());
    }

    // === NotificationPreferences 测试 ===

    #[test]
    fn test_notification_preferences_default() {
        let prefs = NotificationPreferences::default();
        assert!(prefs.enable_notifications);
        assert!(prefs.enable_message_notifications);
        assert!(prefs.enable_system_notifications);
        assert!(!prefs.enable_promotional_notifications);
        assert!(prefs.enable_reminder_notifications);
        assert!(prefs.quiet_hours_start.is_none());
        assert!(prefs.quiet_hours_end.is_none());
    }

    // === UpdateNotificationPreferencesRequest 测试 ===

    #[test]
    fn test_update_notification_preferences_partial() {
        let json = r#"{
            "enable_promotional_notifications": true,
            "quiet_hours_start": "22:00"
        }"#;

        let request: UpdateNotificationPreferencesRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.enable_promotional_notifications, Some(true));
        assert_eq!(request.quiet_hours_start, Some("22:00".to_string()));
        assert!(request.enable_notifications.is_none());
        assert!(request.quiet_hours_end.is_none());
    }

    // === CreateTemplateRequest 测试 ===

    #[test]
    fn test_create_template_request() {
        let json = r#"{
            "name": "welcome",
            "title_template": "Welcome {{name}}!",
            "body_template": "Hello {{name}}, welcome to our platform."
        }"#;

        let request: CreateTemplateRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "welcome");
        assert!(request.title_template.contains("{{name}}"));
        assert!(request.data_template.is_none());
    }

    // === PushStats 序列化测试 ===

    #[test]
    fn test_push_stats_serialization() {
        let stats = PushStats {
            total_sent: 100,
            total_failed: 5,
            by_device_type: vec![],
            by_date: vec![],
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total_sent\":100"));
        assert!(json.contains("\"total_failed\":5"));
    }

    // === PushHealthStatus 序列化测试 ===

    #[test]
    fn test_push_health_status_serialization() {
        let health = PushHealthStatus {
            total_devices: 50,
            active_devices: 45,
            devices_by_type: vec![],
            recent_failures: 2,
            success_rate: 0.95,
        };

        let json = serde_json::to_string(&health).unwrap();
        assert!(json.contains("\"total_devices\":50"));
        assert!(json.contains("\"active_devices\":45"));
        assert!(json.contains("0.95"));
    }
}
