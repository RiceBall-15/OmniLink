use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use serde_json::Value as JsonValue;

/// 消息类型（与前端枚举匹配）
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum MessageType {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "file")]
    File,
    #[serde(rename = "system")]
    System,
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageType::Text => write!(f, "text"),
            MessageType::Image => write!(f, "image"),
            MessageType::File => write!(f, "file"),
            MessageType::System => write!(f, "system"),
        }
    }
}

impl std::str::FromStr for MessageType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(MessageType::Text),
            "image" => Ok(MessageType::Image),
            "file" => Ok(MessageType::File),
            "system" => Ok(MessageType::System),
            _ => Err(format!("无效的消息类型: {}", s)),
        }
    }
}

/// 消息状态（与前端枚举匹配）
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum MessageStatus {
    #[serde(rename = "sending")]
    Sending,
    #[serde(rename = "sent")]
    Sent,
    #[serde(rename = "delivered")]
    Delivered,
    #[serde(rename = "read")]
    Read,
    #[serde(rename = "failed")]
    Failed,
}

impl std::fmt::Display for MessageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageStatus::Sending => write!(f, "sending"),
            MessageStatus::Sent => write!(f, "sent"),
            MessageStatus::Delivered => write!(f, "delivered"),
            MessageStatus::Read => write!(f, "read"),
            MessageStatus::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for MessageStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sending" => Ok(MessageStatus::Sending),
            "sent" => Ok(MessageStatus::Sent),
            "delivered" => Ok(MessageStatus::Delivered),
            "read" => Ok(MessageStatus::Read),
            "failed" => Ok(MessageStatus::Failed),
            _ => Err(format!("无效的消息状态: {}", s)),
        }
    }
}

/// 消息实体（与前端接口完全匹配）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "senderId")]
    pub sender_id: String,
    pub content: String,
    #[serde(rename = "type")]
    pub type_: MessageType,
    pub status: MessageStatus,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "readAt", skip_serializing_if = "Option::is_none")]
    pub read_at: Option<String>,
    #[serde(rename = "replyTo", skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonValue>,
}

/// 数据库中的消息实体
#[derive(Debug, Clone, FromRow)]
pub struct MessageEntity {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
    #[sqlx(rename = "type")]
    pub type_: String,
    pub status: String,
    pub reply_to: Option<Uuid>,
    pub metadata: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}

impl MessageEntity {
    /// 转换为 API 响应的 Message 格式
    pub fn to_message(&self) -> Message {
        Message {
            id: self.id.to_string(),
            conversation_id: self.conversation_id.to_string(),
            sender_id: self.sender_id.to_string(),
            content: self.content.clone(),
            type_: self.type_.parse().unwrap_or(MessageType::Text),
            status: self.status.parse().unwrap_or(MessageStatus::Sent),
            created_at: self.created_at.to_rfc3339(),
            updated_at: self.updated_at.to_rfc3339(),
            read_at: self.read_at.map(|t| t.to_rfc3339()),
            reply_to: self.reply_to.map(|u| u.to_string()),
            metadata: self.metadata.clone(),
        }
    }
}

/// 发送消息请求
#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
    #[serde(rename = "type")]
    pub type_: MessageType,
}

/// 编辑消息请求
#[derive(Debug, Deserialize)]
pub struct EditMessageRequest {
    pub content: String,
}

/// 在线状态（与前端枚举匹配）
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum OnlineStatus {
    #[serde(rename = "offline")]
    Offline,
    #[serde(rename = "online")]
    Online,
    #[serde(rename = "away")]
    Away,
    #[serde(rename = "busy")]
    Busy,
}

impl std::fmt::Display for OnlineStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OnlineStatus::Offline => write!(f, "offline"),
            OnlineStatus::Online => write!(f, "online"),
            OnlineStatus::Away => write!(f, "away"),
            OnlineStatus::Busy => write!(f, "busy"),
        }
    }
}

impl std::str::FromStr for OnlineStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "offline" => Ok(OnlineStatus::Offline),
            "online" => Ok(OnlineStatus::Online),
            "away" => Ok(OnlineStatus::Away),
            "busy" => Ok(OnlineStatus::Busy),
            _ => Err(format!("无效的在线状态: {}", s)),
        }
    }
}

/// 更新在线状态请求
#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: OnlineStatus,
}

/// 创建消息参数（用于数据库插入）
pub struct CreateMessageParams {
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
    pub type_: MessageType,
    pub reply_to: Option<Uuid>,
    pub metadata: Option<JsonValue>,
}
