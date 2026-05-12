use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sqlx::FromRow;

use crate::models::message::Message;

/// 会话类型（与前端匹配）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum ConversationType {
    #[serde(rename = "direct")]
    Direct,
    #[serde(rename = "group")]
    Group,
    #[serde(rename = "ai")]
    Ai,
}

impl std::fmt::Display for ConversationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversationType::Direct => write!(f, "direct"),
            ConversationType::Group => write!(f, "group"),
            ConversationType::Ai => write!(f, "ai"),
        }
    }
}

impl std::str::FromStr for ConversationType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "direct" => Ok(ConversationType::Direct),
            "group" => Ok(ConversationType::Group),
            "ai" => Ok(ConversationType::Ai),
            _ => Err(format!("无效的会话类型: {}", s)),
        }
    }
}

/// 会话实体（与前端接口完全匹配）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: ConversationType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message: Option<Message>,
    #[serde(rename = "unreadCount")]
    pub unread_count: i32,
    #[serde(rename = "isPinned")]
    pub is_pinned: bool,
    #[serde(rename = "isMuted")]
    pub is_muted: bool,
    #[serde(rename = "isArchived")]
    pub is_archived: bool,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

/// 数据库中的会话实体
#[derive(Debug, Clone, FromRow)]
pub struct ConversationEntity {
    pub id: Uuid,
    #[sqlx(rename = "type")]
    pub type_: String,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub created_by: Option<Uuid>,
    pub unread_count: i32,
    pub is_pinned: bool,
    pub is_muted: bool,
    pub is_archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ConversationEntity {
    /// 转换为 API 响应的 Conversation 格式
    pub fn to_conversation(&self) -> Conversation {
        Conversation {
            id: self.id.to_string(),
            type_: self.type_.parse().unwrap_or(ConversationType::Direct),
            name: self.name.clone(),
            avatar: self.avatar.clone(),
            last_message: None,
            unread_count: self.unread_count,
            is_pinned: self.is_pinned,
            is_muted: self.is_muted,
            is_archived: self.is_archived,
            created_at: self.created_at.to_rfc3339(),
            updated_at: self.updated_at.to_rfc3339(),
        }
    }
}

/// 创建会话请求
#[derive(Debug, Deserialize)]
pub struct CreateConversationRequest {
    #[serde(rename = "type")]
    pub type_: ConversationType,
    pub name: Option<String>,
    #[serde(rename = "participantIds")]
    pub participant_ids: Option<Vec<String>>,
}

/// 创建会话参数（用于数据库插入）
pub struct CreateConversationParams {
    pub type_: ConversationType,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub created_by: Uuid,
    pub participant_ids: Vec<Uuid>,
}

/// 更新会话请求
#[derive(Debug, Deserialize)]
pub struct UpdateConversationRequest {
    pub name: Option<String>,
    pub avatar: Option<String>,
    #[serde(rename = "isPinned")]
    pub is_pinned: Option<bool>,
    #[serde(rename = "isMuted")]
    pub is_muted: Option<bool>,
    #[serde(rename = "isArchived")]
    pub is_archived: Option<bool>,
}

/// 会话搜索请求参数
#[derive(Debug, Deserialize)]
pub struct SearchConversationsQuery {
    pub q: String,
    #[serde(default)]
    pub include_archived: bool,
}

/// 会话标签
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConversationTag {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 创建标签请求
#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    pub color: Option<String>,
}

/// 会话-标签关联
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConversationTagLink {
    pub conversation_id: Uuid,
    pub tag_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// 会话排序选项
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ConversationSortBy {
    #[default]
    UpdatedAt,
    CreatedAt,
    Name,
    UnreadCount,
}

impl std::fmt::Display for ConversationSortBy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversationSortBy::UpdatedAt => write!(f, "updated_at"),
            ConversationSortBy::CreatedAt => write!(f, "created_at"),
            ConversationSortBy::Name => write!(f, "name"),
            ConversationSortBy::UnreadCount => write!(f, "unread_count"),
        }
    }
}

/// 会话排序方向
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    #[default]
    Desc,
    Asc,
}

impl std::fmt::Display for SortOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortOrder::Desc => write!(f, "DESC"),
            SortOrder::Asc => write!(f, "ASC"),
        }
    }
}

/// 获取会话列表查询参数
#[derive(Debug, Deserialize, Default)]
pub struct GetConversationsQuery {
    pub sort_by: Option<ConversationSortBy>,
    pub order: Option<SortOrder>,
    pub tag_id: Option<String>,
    #[serde(default)]
    pub include_archived: bool,
}
