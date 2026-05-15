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
    #[serde(rename = "voice")]
    Voice,
    #[serde(rename = "video")]
    Video,
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageType::Text => write!(f, "text"),
            MessageType::Image => write!(f, "image"),
            MessageType::File => write!(f, "file"),
            MessageType::System => write!(f, "system"),
            MessageType::Voice => write!(f, "voice"),
            MessageType::Video => write!(f, "video"),
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
            "voice" => Ok(MessageType::Voice),
            "video" => Ok(MessageType::Video),
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
    pub created_at: DateTime<Utc>,
}

/// 消息投递回执（per-user delivery tracking for group messages）
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct DeliveryReceipt {
    pub id: Uuid,
    pub message_id: Uuid,
    pub user_id: Uuid,
    /// 投递状态: delivered, read
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建投递回执请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateDeliveryReceiptRequest {
    pub message_id: Uuid,
    pub user_id: Uuid,
    pub status: String,
}

/// 投递回执统计
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DeliveryReceiptStats {
    pub message_id: Uuid,
    pub total_recipients: i64,
    pub delivered_count: i64,
    pub read_count: i64,
    pub pending_count: i64,
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

fn default_message_type() -> MessageType { MessageType::Text }

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
    /// 是否为阅后即焚消息
    #[serde(rename = "burnAfterReading", default)]
    pub burn_after_reading: bool,
    /// 阅读后焚毁时间（秒），None 表示不焚毁
    #[serde(rename = "burnAfterSeconds", skip_serializing_if = "Option::is_none")]
    pub burn_after_seconds: Option<i32>,
    /// 消息被焚毁时间，None 表示未焚毁
    #[serde(rename = "burnedAt", skip_serializing_if = "Option::is_none")]
    pub burned_at: Option<String>,
    /// 引用消息摘要信息（当 reply_to 不为空时返回）
    #[serde(rename = "quotedMessage", skip_serializing_if = "Option::is_none")]
    pub quoted_message: Option<QuotedMessageInfo>,
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
    pub burn_after_reading: bool,
    pub burn_after_seconds: Option<i32>,
    pub burned_at: Option<DateTime<Utc>>,
}

impl MessageEntity {
    /// 转换为 API 响应的 Message 格式
    pub fn to_message(&self) -> Message {
        self.to_message_with_quotes(None)
    }

    /// 转换为 API 响应的 Message 格式（带引用消息信息）
    pub fn to_message_with_quotes(&self, quoted_message: Option<QuotedMessageInfo>) -> Message {
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
            burn_after_reading: self.burn_after_reading,
            burn_after_seconds: self.burn_after_seconds,
            burned_at: self.burned_at.map(|t| t.to_rfc3339()),
            quoted_message,
        }
    }
}

/// 引用消息摘要信息（嵌入在消息响应中）
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct QuotedMessageInfo {
    #[serde(rename = "messageId")]
    pub message_id: String,
    /// 发送者ID
    #[serde(rename = "senderId")]
    pub sender_id: String,
    /// 发送者名称（可选，需要JOIN查询）
    #[serde(rename = "senderName", skip_serializing_if = "Option::is_none")]
    pub sender_name: Option<String>,
    /// 消息内容（截断预览，最多100字符）
    pub content: String,
    /// 消息类型
    #[serde(rename = "type")]
    pub type_: MessageType,
    /// 创建时间
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// 是否为阅后即焚（可选）
    /// 阅后即焚（可选）
    #[serde(rename = "burnAfterReading", default)]
    pub burn_after_reading: bool,
    /// 阅读后焚毁时间（秒），默认30秒
    #[serde(rename = "burnAfterSeconds", skip_serializing_if = "Option::is_none")]
    pub burn_after_seconds: Option<i32>,
    /// 媒体元数据（可选，用于 Voice/Video/Image/File 消息）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MediaMetadata>,
    /// 嵌套引用（支持多层引用展示，最多3层）
    #[serde(rename = "quotedMessage", skip_serializing_if = "Option::is_none")]
    pub quoted_message: Option<Box<QuotedMessageInfo>>,
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
    /// 阅后即焚（可选）
    #[serde(rename = "burnAfterReading", default)]
    pub burn_after_reading: bool,
    /// 阅读后焚毁时间（秒），默认30秒
    #[serde(rename = "burnAfterSeconds", skip_serializing_if = "Option::is_none")]
    pub burn_after_seconds: Option<i32>,
    /// 媒体元数据（可选，用于 Voice/Video/Image/File 消息）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MediaMetadata>,
}

/// 编辑消息请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct EditMessageRequest {
    pub content: String,
}

/// 媒体消息元数据
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MediaMetadata {
    /// 媒体时长（秒），用于 Voice/Video 消息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    /// 媒体宽度（像素），用于 Image/Video 消息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// 媒体高度（像素），用于 Image/Video 消息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// 缩略图 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    /// 文件大小（字节）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<u64>,
    /// MIME 类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// 原始文件名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_filename: Option<String>,
}

impl MediaMetadata {
    /// 创建语音消息元数据
    pub fn voice(duration: f64, file_size: u64, mime_type: &str) -> Self {
        Self {
            duration: Some(duration),
            width: None,
            height: None,
            thumbnail_url: None,
            file_size: Some(file_size),
            mime_type: Some(mime_type.to_string()),
            original_filename: None,
        }
    }

    /// 创建视频消息元数据
    pub fn video(duration: f64, width: u32, height: u32, file_size: u64, thumbnail_url: Option<String>) -> Self {
        Self {
            duration: Some(duration),
            width: Some(width),
            height: Some(height),
            thumbnail_url,
            file_size: Some(file_size),
            mime_type: Some("video/mp4".to_string()),
            original_filename: None,
        }
    }

    /// 创建图片消息元数据
    pub fn image(width: u32, height: u32, file_size: u64, mime_type: &str) -> Self {
        Self {
            duration: None,
            width: Some(width),
            height: Some(height),
            thumbnail_url: None,
            file_size: Some(file_size),
            mime_type: Some(mime_type.to_string()),
            original_filename: None,
        }
    }

    /// 创建文件消息元数据
    pub fn file(file_size: u64, mime_type: &str, original_filename: &str) -> Self {
        Self {
            duration: None,
            width: None,
            height: None,
            thumbnail_url: None,
            file_size: Some(file_size),
            mime_type: Some(mime_type.to_string()),
            original_filename: Some(original_filename.to_string()),
        }
    }

    /// 转换为 JSON Value
    pub fn to_json(&self) -> JsonValue {
        serde_json::to_value(self).unwrap_or_default()
    }
}

/// 带元数据的发送消息请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SendMessageWithMetadataRequest {
    pub content: String,
    #[serde(rename = "type")]
    pub type_: MessageType,
    /// 回复的消息 ID（可选）
    #[serde(rename = "replyTo", skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    /// 阅后即焚（可选）
    #[serde(rename = "burnAfterReading", default)]
    pub burn_after_reading: bool,
    /// 阅读后焚毁时间（秒），默认30秒
    #[serde(rename = "burnAfterSeconds", skip_serializing_if = "Option::is_none")]
    pub burn_after_seconds: Option<i32>,
    /// 媒体元数据（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MediaMetadata>,
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
    #[serde(rename = "invisible")]
    Invisible,
}

impl std::fmt::Display for OnlineStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OnlineStatus::Offline => write!(f, "offline"),
            OnlineStatus::Online => write!(f, "online"),
            OnlineStatus::Away => write!(f, "away"),
            OnlineStatus::Busy => write!(f, "busy"),
            OnlineStatus::Invisible => write!(f, "invisible"),
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
            "invisible" => Ok(OnlineStatus::Invisible),
            _ => Err(format!("无效的在线状态: {}", s)),
        }
    }
}

/// 更新在线状态请求
#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: OnlineStatus,
    #[serde(default)]
    pub status_message: Option<String>,
}

/// 创建消息参数（用于数据库插入）
pub struct CreateMessageParams {
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
    pub type_: MessageType,
    pub reply_to: Option<Uuid>,
    pub metadata: Option<JsonValue>,
    /// 阅后即焚
    pub burn_after_reading: bool,
    /// 阅读后焚毁时间（秒）
    pub burn_after_seconds: Option<i32>,
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

/// 定时消息状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum ScheduledStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "sent")]
    Sent,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "failed")]
    Failed,
}

impl std::fmt::Display for ScheduledStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScheduledStatus::Pending => write!(f, "pending"),
            ScheduledStatus::Sent => write!(f, "sent"),
            ScheduledStatus::Cancelled => write!(f, "cancelled"),
            ScheduledStatus::Failed => write!(f, "failed"),
        }
    }
}

/// 定时消息实体
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ScheduledMessage {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub conversation_id: Uuid,
    pub content: String,
    pub message_type: String,
    pub reply_to: Option<Uuid>,
    pub metadata: Option<JsonValue>,
    pub scheduled_at: DateTime<Utc>,
    pub status: String,
    pub sent_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建定时消息请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateScheduledMessageRequest {
    pub conversation_id: String,
    pub content: String,
    #[serde(rename = "type", default = "default_message_type")]
    pub type_: MessageType,
    pub reply_to: Option<String>,
    pub metadata: Option<JsonValue>,
    pub scheduled_at: DateTime<Utc>,
}

/// 更新定时消息请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateScheduledMessageRequest {
    pub content: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<MessageType>,
    pub reply_to: Option<String>,
    pub metadata: Option<JsonValue>,
    pub scheduled_at: Option<DateTime<Utc>>,
}

/// 定时消息查询参数
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ScheduledMessageQuery {
    pub status: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

/// 定时消息信息（API响应）
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ScheduledMessageInfo {
    pub id: String,
    pub sender_id: String,
    pub conversation_id: String,
    pub content: String,
    pub message_type: String,
    pub reply_to: Option<String>,
    pub metadata: Option<JsonValue>,
    pub scheduled_at: DateTime<Utc>,
    pub status: String,
    pub sent_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ScheduledMessage {
    /// 转换为API响应格式
    pub fn to_info(&self) -> ScheduledMessageInfo {
        ScheduledMessageInfo {
            id: self.id.to_string(),
            sender_id: self.sender_id.to_string(),
            conversation_id: self.conversation_id.to_string(),
            content: self.content.clone(),
            message_type: self.message_type.clone(),
            reply_to: self.reply_to.map(|id| id.to_string()),
            metadata: self.metadata.clone(),
            scheduled_at: self.scheduled_at,
            status: self.status.clone(),
            sent_at: self.sent_at,
            error_message: self.error_message.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
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
        assert_eq!("voice".parse::<MessageType>().unwrap(), MessageType::Voice);
        assert_eq!("video".parse::<MessageType>().unwrap(), MessageType::Video);
        assert_eq!("TEXT".parse::<MessageType>().unwrap(), MessageType::Text);
        assert_eq!("VOICE".parse::<MessageType>().unwrap(), MessageType::Voice);
    }

    #[test]
    fn test_message_type_from_str_invalid() {
        assert!("unknown".parse::<MessageType>().is_err());
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
        assert_eq!(OnlineStatus::Invisible.to_string(), "invisible");
    }

    #[test]
    fn test_online_status_from_str() {
        assert_eq!("offline".parse::<OnlineStatus>().unwrap(), OnlineStatus::Offline);
        assert_eq!("online".parse::<OnlineStatus>().unwrap(), OnlineStatus::Online);
        assert_eq!("away".parse::<OnlineStatus>().unwrap(), OnlineStatus::Away);
        assert_eq!("busy".parse::<OnlineStatus>().unwrap(), OnlineStatus::Busy);
        assert_eq!("invisible".parse::<OnlineStatus>().unwrap(), OnlineStatus::Invisible);
    }

    #[test]
    fn test_online_status_from_str_invalid() {
        assert!("unknown_status".parse::<OnlineStatus>().is_err());
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
            burn_after_reading: false,
            burn_after_seconds: None,
            burned_at: None,
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

    // === MediaMetadata 测试 ===

    #[test]
    fn test_media_metadata_voice() {
        let metadata = MediaMetadata::voice(30.5, 1024000, "audio/ogg");
        assert_eq!(metadata.duration, Some(30.5));
        assert_eq!(metadata.file_size, Some(1024000));
        assert_eq!(metadata.mime_type, Some("audio/ogg".to_string()));
        assert!(metadata.width.is_none());
        assert!(metadata.height.is_none());
    }

    #[test]
    fn test_media_metadata_video() {
        let metadata = MediaMetadata::video(120.0, 1920, 1080, 5120000, Some("https://example.com/thumb.jpg".to_string()));
        assert_eq!(metadata.duration, Some(120.0));
        assert_eq!(metadata.width, Some(1920));
        assert_eq!(metadata.height, Some(1080));
        assert_eq!(metadata.file_size, Some(5120000));
        assert_eq!(metadata.thumbnail_url, Some("https://example.com/thumb.jpg".to_string()));
        assert_eq!(metadata.mime_type, Some("video/mp4".to_string()));
    }

    #[test]
    fn test_media_metadata_image() {
        let metadata = MediaMetadata::image(800, 600, 512000, "image/png");
        assert_eq!(metadata.width, Some(800));
        assert_eq!(metadata.height, Some(600));
        assert_eq!(metadata.file_size, Some(512000));
        assert_eq!(metadata.mime_type, Some("image/png".to_string()));
        assert!(metadata.duration.is_none());
    }

    #[test]
    fn test_media_metadata_file() {
        let metadata = MediaMetadata::file(2048000, "application/pdf", "document.pdf");
        assert_eq!(metadata.file_size, Some(2048000));
        assert_eq!(metadata.mime_type, Some("application/pdf".to_string()));
        assert_eq!(metadata.original_filename, Some("document.pdf".to_string()));
    }

    #[test]
    fn test_media_metadata_serialization() {
        let metadata = MediaMetadata::voice(30.5, 1024000, "audio/ogg");
        let json = serde_json::to_value(&metadata).unwrap();
        assert!(json.is_object());
        assert_eq!(json["duration"], 30.5);
        assert_eq!(json["file_size"], 1024000);
        assert_eq!(json["mime_type"], "audio/ogg");
        // Optional fields should not be present
        assert!(json.get("width").is_none());
        assert!(json.get("height").is_none());
    }

    #[test]
    fn test_media_metadata_to_json() {
        let metadata = MediaMetadata::image(800, 600, 512000, "image/png");
        let json = metadata.to_json();
        assert!(json.is_object());
        assert_eq!(json["width"], 800);
        assert_eq!(json["height"], 600);
    }

    #[test]
    fn test_send_message_with_metadata_request() {
        let json = r#"{
            "content": "https://example.com/voice.ogg",
            "type": "voice",
            "metadata": {
                "duration": 30.5,
                "file_size": 1024000,
                "mime_type": "audio/ogg"
            }
        }"#;

        let request: SendMessageWithMetadataRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.content, "https://example.com/voice.ogg");
        assert_eq!(request.type_, MessageType::Voice);
        assert!(request.metadata.is_some());
        let metadata = request.metadata.unwrap();
        assert_eq!(metadata.duration, Some(30.5));
        assert_eq!(metadata.file_size, Some(1024000));
    }
}

/// 线程摘要信息
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ThreadSummary {
    /// 父消息 ID
    #[serde(rename = "parentId")]
    pub parent_id: String,
    /// 父消息内容
    #[serde(rename = "parentContent")]
    pub parent_content: String,
    /// 父消息发送者 ID
    #[serde(rename = "parentSenderId")]
    pub parent_sender_id: String,
    /// 父消息类型
    #[serde(rename = "parentType")]
    pub parent_type: String,
    /// 父消息创建时间
    #[serde(rename = "parentCreatedAt")]
    pub parent_created_at: String,
    /// 回复数量
    #[serde(rename = "replyCount")]
    pub reply_count: i64,
    /// 最后回复时间
    #[serde(rename = "lastReplyAt")]
    pub last_reply_at: String,
}

/// 线程详情（包含父消息和回复列表）
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ThreadDetail {
    /// 父消息
    pub parent: Message,
    /// 回复列表
    pub replies: Vec<Message>,
    /// 总回复数
    #[serde(rename = "totalReplies")]
    pub total_replies: i64,
}

/// 线程查询参数
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ThreadQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_thread_page_size")]
    pub limit: i64,
}

fn default_thread_page_size() -> i64 { 50 }
