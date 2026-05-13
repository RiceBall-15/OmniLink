//! 系统公告/通知模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 系统公告类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnnouncementType {
    Info,
    Warning,
    Maintenance,
    Update,
}

impl std::fmt::Display for AnnouncementType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnnouncementType::Info => write!(f, "info"),
            AnnouncementType::Warning => write!(f, "warning"),
            AnnouncementType::Maintenance => write!(f, "maintenance"),
            AnnouncementType::Update => write!(f, "update"),
        }
    }
}

impl std::str::FromStr for AnnouncementType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "info" => Ok(AnnouncementType::Info),
            "warning" => Ok(AnnouncementType::Warning),
            "maintenance" => Ok(AnnouncementType::Maintenance),
            "update" => Ok(AnnouncementType::Update),
            _ => Err(format!("Unknown announcement type: {}", s)),
        }
    }
}

/// 系统公告实体（数据库）
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AnnouncementEntity {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub type_: String,
    pub priority: i32,
    pub created_by: Uuid,
    pub is_active: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 带已读状态的公告实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AnnouncementWithReadStatus {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub type_: String,
    pub priority: i32,
    pub created_by: Uuid,
    pub is_active: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_read: Option<bool>,
    pub read_at: Option<DateTime<Utc>>,
}

impl AnnouncementWithReadStatus {
    pub fn to_announcement(&self) -> Announcement {
        Announcement {
            id: self.id.to_string(),
            title: self.title.clone(),
            content: self.content.clone(),
            type_: self.type_.clone(),
            priority: self.priority,
            created_by: self.created_by.to_string(),
            is_active: self.is_active,
            expires_at: self.expires_at.map(|dt| dt.to_rfc3339()),
            created_at: self.created_at.to_rfc3339(),
            updated_at: self.updated_at.to_rfc3339(),
            is_read: self.is_read,
            read_at: self.read_at.map(|dt| dt.to_rfc3339()),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Announcement {
    pub id: String,
    pub title: String,
    pub content: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub priority: i32,
    pub created_by: String,
    pub is_active: bool,
    pub expires_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_read: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_at: Option<String>,
}

impl AnnouncementEntity {
    /// 转换为 API 响应格式
    pub fn to_announcement(&self) -> Announcement {
        Announcement {
            id: self.id.to_string(),
            title: self.title.clone(),
            content: self.content.clone(),
            type_: self.type_.clone(),
            priority: self.priority,
            created_by: self.created_by.to_string(),
            is_active: self.is_active,
            expires_at: self.expires_at.map(|t| t.to_rfc3339()),
            created_at: self.created_at.to_rfc3339(),
            updated_at: self.updated_at.to_rfc3339(),
            is_read: None,
            read_at: None,
        }
    }

    /// 转换为 API 响应格式，包含已读状态
    pub fn to_announcement_with_read(&self, is_read: bool, read_at: Option<DateTime<Utc>>) -> Announcement {
        Announcement {
            id: self.id.to_string(),
            title: self.title.clone(),
            content: self.content.clone(),
            type_: self.type_.clone(),
            priority: self.priority,
            created_by: self.created_by.to_string(),
            is_active: self.is_active,
            expires_at: self.expires_at.map(|t| t.to_rfc3339()),
            created_at: self.created_at.to_rfc3339(),
            updated_at: self.updated_at.to_rfc3339(),
            is_read: Some(is_read),
            read_at: read_at.map(|t| t.to_rfc3339()),
        }
    }
}

/// 创建公告请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAnnouncementRequest {
    pub title: String,
    pub content: String,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub priority: Option<i32>,
    pub expires_at: Option<String>,
}

/// 公告已读记录
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AnnouncementRead {
    pub id: Uuid,
    pub announcement_id: Uuid,
    pub user_id: Uuid,
    pub read_at: DateTime<Utc>,
}
