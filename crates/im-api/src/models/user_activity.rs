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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_activity_response_serialize() {
        let resp = UserActivityResponse {
            user_id: "test-uid".to_string(),
            total_messages: 150,
            messages_today: 5,
            messages_this_week: 40,
            messages_this_month: 120,
            total_files_uploaded: 10,
            active_conversations: 8,
            avg_messages_per_day: 5.0,
            last_active_at: Some("2026-05-16T01:00:00Z".to_string()),
            activity_pattern: vec![
                ActivityByHour { hour: 9, count: 25 },
                ActivityByHour { hour: 14, count: 30 },
            ],
            recent_activity: vec![
                RecentActivity {
                    activity_type: "message".to_string(),
                    description: "Sent a text message".to_string(),
                    created_at: "2026-05-16T01:00:00Z".to_string(),
                },
            ],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["user_id"], "test-uid");
        assert_eq!(json["total_messages"], 150);
        assert!(json["activity_pattern"].is_array());
        assert_eq!(json["activity_pattern"][0]["hour"], 9);
    }

    #[test]
    fn test_activity_by_hour() {
        let h = ActivityByHour { hour: 23, count: 0 };
        let json = serde_json::to_value(&h).unwrap();
        assert_eq!(json["hour"], 23);
        assert_eq!(json["count"], 0);
    }

    #[test]
    fn test_activity_query_defaults() {
        let json = r#"{}"#;
        let q: ActivityQuery = serde_json::from_str(json).unwrap();
        assert!(q.days.is_none());
    }

    #[test]
    fn test_activity_query_with_days() {
        let json = r#"{"days": 30}"#;
        let q: ActivityQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.days, Some(30));
    }

    #[test]
    fn test_record_activity_params() {
        let params = RecordActivityParams {
            user_id: "uid".to_string(),
            activity_type: "login".to_string(),
            details: Some(serde_json::json!({"ip": "127.0.0.1"})),
            ip_address: Some("127.0.0.1".to_string()),
        };
        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["activity_type"], "login");
        assert!(json["details"].is_object());
    }
}
