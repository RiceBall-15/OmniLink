use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use serde_json::Value as JsonValue;

/// 消息类型（与前端枚举匹配）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
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

/// 消息表情回应
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MessageReaction {
    pub id: Uuid,
    pub message_id: Uuid,
    pub user_id: Uuid,
    pub emoji: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 添加表情回应请求
#[derive(Debug, Deserialize)]
pub struct AddReactionRequest {
    pub emoji: String,
}

/// 表情回应统计
#[derive(Debug, Serialize)]
pub struct ReactionSummary {
    pub emoji: String,
    pub count: i64,
    pub users: Vec<Uuid>,
}

/// 消息收藏/书签
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MessageBookmark {
    pub id: Uuid,
    pub user_id: Uuid,
    pub message_id: Uuid,
    pub conversation_id: Uuid,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 添加收藏请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AddBookmarkRequest {
    /// 收藏备注（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// 收藏列表查询参数
#[derive(Debug, Deserialize)]
pub struct BookmarkQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub limit: i64,
}

fn default_page() -> i64 { 1 }
fn default_page_size() -> i64 { 50 }

/// 收藏信息（包含消息详情）
#[derive(Debug, Serialize)]
pub struct BookmarkInfo {
    pub id: String,
    #[serde(rename = "messageId")]
    pub message_id: String,
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "senderId")]
    pub sender_id: String,
    pub content: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub note: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "bookmarkedAt")]
    pub bookmarked_at: String,
}

/// 草稿消息
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DraftMessage {
    pub id: Uuid,
    pub user_id: Uuid,
    pub conversation_id: Uuid,
    pub content: String,
    #[sqlx(rename = "type")]
    pub type_: String,
    pub reply_to: Option<Uuid>,
    pub metadata: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 保存草稿请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SaveDraftRequest {
    pub content: String,
    #[serde(rename = "type", default = "default_draft_type")]
    pub type_: String,
    #[serde(rename = "replyTo", skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonValue>,
}

fn default_draft_type() -> String { "text".to_string() }

/// 草稿信息（API 响应）
#[derive(Debug, Serialize)]
pub struct DraftInfo {
    pub id: String,
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    pub content: String,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "replyTo", skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonValue>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

impl DraftMessage {
    pub fn to_draft_info(&self) -> DraftInfo {
        DraftInfo {
            id: self.id.to_string(),
            conversation_id: self.conversation_id.to_string(),
            content: self.content.clone(),
            type_: self.type_.clone(),
            reply_to: self.reply_to.map(|u| u.to_string()),
            metadata: self.metadata.clone(),
            created_at: self.created_at.to_rfc3339(),
            updated_at: self.updated_at.to_rfc3339(),
        }
    }
}

/// 草稿查询参数
#[derive(Debug, Deserialize)]
pub struct DraftQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub limit: i64,
}

/// 消息实体（与前端接口完全匹配）
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
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
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SendMessageRequest {
    pub content: String,
    #[serde(rename = "type")]
    pub type_: MessageType,
    /// 回复的消息 ID（可选）
    #[serde(rename = "replyTo", skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
}

/// 编辑消息请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct EditMessageRequest {
    pub content: String,
}

/// 在线状态（与前端枚举匹配）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
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

/// 批量发送消息请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct BatchSendMessageRequest {
    pub conversation_id: String,
    pub messages: Vec<SendMessageRequest>,
}

/// 批量删除消息请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct BatchDeleteMessagesRequest {
    pub message_ids: Vec<String>,
}

/// 批量标记已读请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct BatchMarkReadRequest {
    pub conversation_ids: Vec<String>,
}

/// 批量操作结果
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BatchOperationResult {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    pub errors: Vec<BatchOperationError>,
}

/// 批量操作错误
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BatchOperationError {
    pub index: usize,
    pub id: Option<String>,
    pub error: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    // === MessageType 测试 ===

    #[test]
    fn test_message_type_display() {
        assert_eq!(MessageType::Text.to_string(), "text");
        assert_eq!(MessageType::Image.to_string(), "image");
        assert_eq!(MessageType::File.to_string(), "file");
        assert_eq!(MessageType::System.to_string(), "system");
    }

    #[test]
    fn test_message_type_from_str() {
        assert_eq!("text".parse::<MessageType>().unwrap(), MessageType::Text);
        assert_eq!("image".parse::<MessageType>().unwrap(), MessageType::Image);
        assert_eq!("file".parse::<MessageType>().unwrap(), MessageType::File);
        assert_eq!("system".parse::<MessageType>().unwrap(), MessageType::System);
        assert_eq!("TEXT".parse::<MessageType>().unwrap(), MessageType::Text);
    }

    #[test]
    fn test_message_type_from_str_invalid() {
        assert!("video".parse::<MessageType>().is_err());
        assert!("".parse::<MessageType>().is_err());
    }

    #[test]
    fn test_message_type_serialization() {
        let json = serde_json::to_string(&MessageType::Text).unwrap();
        assert_eq!(json, "\"text\"");
        let json = serde_json::to_string(&MessageType::Image).unwrap();
        assert_eq!(json, "\"image\"");
    }

    #[test]
    fn test_message_type_deserialization() {
        let msg_type: MessageType = serde_json::from_str("\"text\"").unwrap();
        assert_eq!(msg_type, MessageType::Text);
        let msg_type: MessageType = serde_json::from_str("\"file\"").unwrap();
        assert_eq!(msg_type, MessageType::File);
    }

    // === MessageStatus 测试 ===

    #[test]
    fn test_message_status_display() {
        assert_eq!(MessageStatus::Sending.to_string(), "sending");
        assert_eq!(MessageStatus::Sent.to_string(), "sent");
        assert_eq!(MessageStatus::Delivered.to_string(), "delivered");
        assert_eq!(MessageStatus::Read.to_string(), "read");
        assert_eq!(MessageStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_message_status_from_str() {
        assert_eq!("sending".parse::<MessageStatus>().unwrap(), MessageStatus::Sending);
        assert_eq!("sent".parse::<MessageStatus>().unwrap(), MessageStatus::Sent);
        assert_eq!("delivered".parse::<MessageStatus>().unwrap(), MessageStatus::Delivered);
        assert_eq!("read".parse::<MessageStatus>().unwrap(), MessageStatus::Read);
        assert_eq!("failed".parse::<MessageStatus>().unwrap(), MessageStatus::Failed);
    }

    #[test]
    fn test_message_status_from_str_invalid() {
        assert!("unknown".parse::<MessageStatus>().is_err());
        assert!("".parse::<MessageStatus>().is_err());
    }

    // === OnlineStatus 测试 ===

    #[test]
    fn test_online_status_display() {
        assert_eq!(OnlineStatus::Offline.to_string(), "offline");
        assert_eq!(OnlineStatus::Online.to_string(), "online");
        assert_eq!(OnlineStatus::Away.to_string(), "away");
        assert_eq!(OnlineStatus::Busy.to_string(), "busy");
    }

    #[test]
    fn test_online_status_from_str() {
        assert_eq!("offline".parse::<OnlineStatus>().unwrap(), OnlineStatus::Offline);
        assert_eq!("online".parse::<OnlineStatus>().unwrap(), OnlineStatus::Online);
        assert_eq!("away".parse::<OnlineStatus>().unwrap(), OnlineStatus::Away);
        assert_eq!("busy".parse::<OnlineStatus>().unwrap(), OnlineStatus::Busy);
    }

    #[test]
    fn test_online_status_from_str_invalid() {
        assert!("invisible".parse::<OnlineStatus>().is_err());
        assert!("".parse::<OnlineStatus>().is_err());
    }

    // === Message 测试 ===

    #[test]
    fn test_message_serialization() {
        let msg = Message {
            id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            conversation_id: "660e8400-e29b-41d4-a716-446655440000".to_string(),
            sender_id: "770e8400-e29b-41d4-a716-446655440000".to_string(),
            content: "Hello".to_string(),
            type_: MessageType::Text,
            status: MessageStatus::Sent,
            created_at: "2026-05-13T00:00:00Z".to_string(),
            updated_at: "2026-05-13T00:00:00Z".to_string(),
            read_at: None,
            reply_to: None,
            metadata: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"conversationId\""));
        assert!(json.contains("\"senderId\""));
        assert!(json.contains("\"createdAt\""));
        assert!(!json.contains("\"readAt\"")); // skip_serializing_if = None
        assert!(!json.contains("\"replyTo\""));
    }

    #[test]
    fn test_message_deserialization() {
        let json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "conversationId": "660e8400-e29b-41d4-a716-446655440000",
            "senderId": "770e8400-e29b-41d4-a716-446655440000",
            "content": "Hello",
            "type": "text",
            "status": "sent",
            "createdAt": "2026-05-13T00:00:00Z",
            "updatedAt": "2026-05-13T00:00:00Z"
        }"#;

        let msg: Message = serde_json::from_str(json).unwrap();
        assert_eq!(msg.content, "Hello");
        assert_eq!(msg.type_, MessageType::Text);
        assert_eq!(msg.status, MessageStatus::Sent);
    }

    // === SendMessageRequest 测试 ===

    #[test]
    fn test_send_message_request_deserialization() {
        let json = r#"{
            "content": "Hello World",
            "type": "text"
        }"#;

        let request: SendMessageRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.content, "Hello World");
        assert_eq!(request.type_, MessageType::Text);
    }

    #[test]
    fn test_send_message_request_image_type() {
        let json = r#"{
            "content": "image_url",
            "type": "image"
        }"#;

        let request: SendMessageRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.type_, MessageType::Image);
    }

    // === EditMessageRequest 测试 ===

    #[test]
    fn test_edit_message_request_deserialization() {
        let json = r#"{"content": "Updated message"}"#;
        let request: EditMessageRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.content, "Updated message");
    }
}
