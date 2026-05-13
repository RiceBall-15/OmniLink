//! 快捷回复模板模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 快捷回复实体（数据库）
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct QuickReplyEntity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub content: String,
    pub category: String,
    pub sort_order: i32,
    pub is_global: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 快捷回复 API 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickReply {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub content: String,
    pub category: String,
    pub sort_order: i32,
    pub is_global: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl QuickReplyEntity {
    /// 转换为 API 响应格式
    pub fn to_quick_reply(&self) -> QuickReply {
        QuickReply {
            id: self.id.to_string(),
            user_id: self.user_id.to_string(),
            title: self.title.clone(),
            content: self.content.clone(),
            category: self.category.clone(),
            sort_order: self.sort_order,
            is_global: self.is_global,
            created_at: self.created_at.to_rfc3339(),
            updated_at: self.updated_at.to_rfc3339(),
        }
    }
}

/// 创建快捷回复请求
#[derive(Debug, Deserialize)]
pub struct CreateQuickReplyRequest {
    pub title: String,
    pub content: String,
    pub category: Option<String>,
    pub sort_order: Option<i32>,
}

/// 更新快捷回复请求
#[derive(Debug, Deserialize)]
pub struct UpdateQuickReplyRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub category: Option<String>,
    pub sort_order: Option<i32>,
}
