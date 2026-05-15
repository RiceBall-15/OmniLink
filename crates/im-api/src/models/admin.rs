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
