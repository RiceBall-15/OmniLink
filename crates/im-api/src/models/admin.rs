//! 管理员 API 数据模型

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 管理员查看的用户信息
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminUserInfo {
    pub id: String,
    pub username: String,
    pub email: String,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub status: String, // active, banned, suspended
    pub online_status: String,
    pub last_active_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 用户列表查询参数
#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminUserQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub search: Option<String>,
    pub status: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

/// 更新用户状态请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserStatusRequest {
    pub status: String, // active, banned, suspended
    pub reason: Option<String>,
}

/// 强制登出请求
#[derive(Debug, Serialize, ToSchema)]
pub struct ForceLogoutResult {
    pub user_id: String,
    pub username: String,
    pub sessions_revoked: i32,
    pub success: bool,
    pub message: String,
}

/// 用户活动统计
#[derive(Debug, Serialize, ToSchema)]
pub struct UserActivityStats {
    pub user_id: String,
    pub username: String,
    pub total_messages: i64,
    pub messages_today: i64,
    pub messages_this_week: i64,
    pub messages_this_month: i64,
    pub active_conversations: i64,
    pub avg_messages_per_day: f64,
    pub last_active_at: Option<String>,
    pub peak_hours: Vec<PeakHour>,
}

/// 高峰时段
#[derive(Debug, Serialize, ToSchema)]
pub struct PeakHour {
    pub hour: i32,
    pub message_count: i64,
}

/// 用户活动记录
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserActivityRecord {
    pub user_id: String,
    pub activity_type: String, // login, message, file_upload, etc.
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub created_at: String,
}

/// 管理员操作日志
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminActionLog {
    pub admin_id: String,
    pub action: String,
    pub target_user_id: Option<String>,
    pub details: Option<String>,
    pub created_at: String,
}

/// 管理员仪表盘统计
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminDashboardStats {
    pub total_users: i64,
    pub active_users_today: i64,
    pub online_users: i64,
    pub banned_users: i64,
    pub total_messages_today: i64,
    pub total_conversations: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_user_info_serialize() {
        let user = AdminUserInfo {
            id: "test-id".to_string(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            nickname: Some("Test User".to_string()),
            avatar: None,
            status: "active".to_string(),
            online_status: "online".to_string(),
            last_active_at: Some("2026-05-16T01:00:00Z".to_string()),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-05-16T00:00:00Z".to_string(),
        };
        let json = serde_json::to_value(&user).unwrap();
        assert_eq!(json["username"], "testuser");
        assert_eq!(json["status"], "active");
        assert!(json["nickname"].is_string());
        assert!(json["avatar"].is_null());
    }

    #[test]
    fn test_admin_user_query_defaults() {
        let json = r#"{}"#;
        let query: AdminUserQuery = serde_json::from_str(json).unwrap();
        assert!(query.page.is_none());
        assert!(query.limit.is_none());
        assert!(query.search.is_none());
    }

    #[test]
    fn test_update_user_status_request() {
        let json = r#"{"status": "banned", "reason": "spam"}"#;
        let req: UpdateUserStatusRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.status, "banned");
        assert_eq!(req.reason, Some("spam".to_string()));
    }

    #[test]
    fn test_force_logout_result() {
        let result = ForceLogoutResult {
            user_id: "uid".to_string(),
            username: "test".to_string(),
            sessions_revoked: 3,
            success: true,
            message: "Logged out".to_string(),
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["sessions_revoked"], 3);
        assert!(json["success"].as_bool().unwrap());
    }

    #[test]
    fn test_user_activity_stats() {
        let stats = UserActivityStats {
            user_id: "uid".to_string(),
            username: "test".to_string(),
            total_messages: 100,
            messages_today: 5,
            messages_this_week: 30,
            messages_this_month: 80,
            active_conversations: 10,
            avg_messages_per_day: 3.3,
            last_active_at: None,
            peak_hours: vec![PeakHour { hour: 14, message_count: 20 }],
        };
        let json = serde_json::to_value(&stats).unwrap();
        assert_eq!(json["total_messages"], 100);
        assert!(json["peak_hours"].is_array());
    }
}
