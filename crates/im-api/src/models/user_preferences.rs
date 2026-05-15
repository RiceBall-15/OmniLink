//! 用户偏好设置数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 用户偏好设置数据库实体
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserPreferenceEntity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub category: String,
    pub key: String,
    pub value: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 用户偏好设置 API 响应
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserPreference {
    pub id: String,
    pub user_id: String,
    pub category: String,
    pub key: String,
    pub value: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

impl UserPreferenceEntity {
    pub fn to_api(&self) -> UserPreference {
        UserPreference {
            id: self.id.to_string(),
            user_id: self.user_id.to_string(),
            category: self.category.clone(),
            key: self.key.clone(),
            value: self.value.clone(),
            created_at: self.created_at.to_rfc3339(),
            updated_at: self.updated_at.to_rfc3339(),
        }
    }
}

/// 设置偏好请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SetPreferenceRequest {
    /// 偏好类别（如 "theme", "notification", "chat", "privacy"）
    pub category: String,
    /// 偏好键名
    pub key: String,
    /// 偏好值（任意 JSON 值）
    pub value: serde_json::Value,
}

/// 批量设置偏好请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct BatchSetPreferenceRequest {
    pub preferences: Vec<SetPreferenceRequest>,
}

/// 按类别查询偏好
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PreferenceQuery {
    /// 按类别筛选
    pub category: Option<String>,
}

/// 偏好类别汇总
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PreferenceCategorySummary {
    pub category: String,
    pub count: i64,
    pub keys: Vec<String>,
}

/// 所有偏好设置的汇总响应
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AllPreferencesResponse {
    pub preferences: Vec<UserPreference>,
    pub categories: Vec<PreferenceCategorySummary>,
    pub total_count: usize,
}
