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

/// 默认偏好模板
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PreferenceTemplate {
    pub category: String,
    pub key: String,
    pub default_value: serde_json::Value,
    pub description: String,
}

/// 获取默认偏好模板的响应
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DefaultTemplatesResponse {
    pub templates: Vec<PreferenceTemplate>,
    pub total_count: usize,
}

/// 返回系统默认偏好模板列表
pub fn get_default_templates() -> Vec<PreferenceTemplate> {
    vec![
        // 主题设置
        PreferenceTemplate {
            category: "theme".to_string(),
            key: "mode".to_string(),
            default_value: serde_json::json!("light"),
            description: "界面主题模式（light/dark/auto）".to_string(),
        },
        PreferenceTemplate {
            category: "theme".to_string(),
            key: "accent_color".to_string(),
            default_value: serde_json::json!("#1890ff"),
            description: "主题强调色".to_string(),
        },
        PreferenceTemplate {
            category: "theme".to_string(),
            key: "font_size".to_string(),
            default_value: serde_json::json!("medium"),
            description: "字体大小（small/medium/large）".to_string(),
        },
        // 通知设置
        PreferenceTemplate {
            category: "notification".to_string(),
            key: "enabled".to_string(),
            default_value: serde_json::json!(true),
            description: "是否启用通知".to_string(),
        },
        PreferenceTemplate {
            category: "notification".to_string(),
            key: "sound".to_string(),
            default_value: serde_json::json!(true),
            description: "是否启用通知声音".to_string(),
        },
        PreferenceTemplate {
            category: "notification".to_string(),
            key: "show_preview".to_string(),
            default_value: serde_json::json!(true),
            description: "是否在通知中显示消息预览".to_string(),
        },
        PreferenceTemplate {
            category: "notification".to_string(),
            key: "dnd_start".to_string(),
            default_value: serde_json::json!("22:00"),
            description: "免打扰开始时间".to_string(),
        },
        PreferenceTemplate {
            category: "notification".to_string(),
            key: "dnd_end".to_string(),
            default_value: serde_json::json!("08:00"),
            description: "免打扰结束时间".to_string(),
        },
        // 聊天设置
        PreferenceTemplate {
            category: "chat".to_string(),
            key: "enter_to_send".to_string(),
            default_value: serde_json::json!(true),
            description: "Enter键发送消息（false则Ctrl+Enter发送）".to_string(),
        },
        PreferenceTemplate {
            category: "chat".to_string(),
            key: "show_typing_indicator".to_string(),
            default_value: serde_json::json!(true),
            description: "是否显示对方正在输入状态".to_string(),
        },
        PreferenceTemplate {
            category: "chat".to_string(),
            key: "message_grouping".to_string(),
            default_value: serde_json::json!(true),
            description: "是否将连续消息分组显示".to_string(),
        },
        PreferenceTemplate {
            category: "chat".to_string(),
            key: "auto_download_media".to_string(),
            default_value: serde_json::json!(true),
            description: "是否自动下载媒体文件".to_string(),
        },
        // 隐私设置
        PreferenceTemplate {
            category: "privacy".to_string(),
            key: "show_online_status".to_string(),
            default_value: serde_json::json!(true),
            description: "是否显示在线状态".to_string(),
        },
        PreferenceTemplate {
            category: "privacy".to_string(),
            key: "show_read_receipts".to_string(),
            default_value: serde_json::json!(true),
            description: "是否发送已读回执".to_string(),
        },
        PreferenceTemplate {
            category: "privacy".to_string(),
            key: "allow_search_by_email".to_string(),
            default_value: serde_json::json!(true),
            description: "是否允许通过邮箱搜索到我".to_string(),
        },
        // AI助手设置
        PreferenceTemplate {
            category: "ai".to_string(),
            key: "default_model".to_string(),
            default_value: serde_json::json!("gpt-3.5-turbo"),
            description: "默认AI模型".to_string(),
        },
        PreferenceTemplate {
            category: "ai".to_string(),
            key: "stream_response".to_string(),
            default_value: serde_json::json!(true),
            description: "是否启用流式响应".to_string(),
        },
        PreferenceTemplate {
            category: "ai".to_string(),
            key: "max_context_messages".to_string(),
            default_value: serde_json::json!(20),
            description: "AI对话上下文消息数量".to_string(),
        },
    ]
}
