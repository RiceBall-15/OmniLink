//! 用户活动追踪数据模型

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 用户活动统计响应
#[derive(Debug, Serialize, ToSchema)]
pub struct UserActivityResponse {
    pub user_id: String,
    pub total_messages: i64,
    pub messages_today: i64,
    pub messages_this_week: i64,
    pub messages_this_month: i64,
    pub total_files_uploaded: i64,
    pub active_conversations: i64,
    pub avg_messages_per_day: f64,
    pub last_active_at: Option<String>,
    pub activity_pattern: Vec<ActivityByHour>,
    pub recent_activity: Vec<RecentActivity>,
}

/// 按小时统计的活动
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ActivityByHour {
    pub hour: i32,
    pub count: i64,
}

/// 最近活动
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecentActivity {
    pub activity_type: String,
    pub description: String,
    pub created_at: String,
}

/// 用户活动查询参数
#[derive(Debug, Deserialize, ToSchema)]
pub struct ActivityQuery {
    pub days: Option<i32>,
}

/// 记录用户活动的内部结构
#[derive(Debug, Serialize, Deserialize)]
pub struct RecordActivityParams {
    pub user_id: String,
    pub activity_type: String,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
}
