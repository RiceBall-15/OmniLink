use serde::{Deserialize, Serialize};
use validator::Validate;
use uuid::Uuid;

/// WebSocket连接请求
#[derive(Debug, Deserialize)]
pub struct WSConnectRequest {
    pub token: String,
    pub conversation_id: Option<Uuid>,
}

/// WebSocket消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WSMessage {
    #[serde(rename = "type")]
    pub message_type: WSMessageType,
    pub conversation_id: Option<Uuid>,
    pub message_id: Option<Uuid>,
    pub sender_id: Option<Uuid>,
    pub content: Option<String>,
    pub timestamp: Option<i64>,
    pub data: Option<serde_json::Value>,
}

/// WebSocket消息类型
/// 匹配前端 TypeScript 枚举: CONNECT, CONNECTED, MESSAGE, NEW_MESSAGE, PING, PONG, TYPING, READ, ERROR
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WSMessageType {
    // 连接相关
    Connect,      // "connect" - 客户端发起连接请求
    Connected,    // "connected" - 服务器确认连接成功
    Disconnect,   // "disconnect" - 断开连接

    // 消息相关
    Message,      // "message" - 普通消息
    NewMessage,   // "new_message" - 新消息通知
    Read,         // "read" - 已读回执
    Edit,         // "edit" - 消息编辑
    Recall,       // "recall" - 消息撤回
    Burn,         // "burn" - 阅后即焚消息已被焚毁
    QuoteReply,   // "quote_reply" - 引用回复通知

    // 心跳保活
    Ping,         // "ping" - 心跳探测
    Pong,         // "pong" - 心跳响应

    // 输入状态
    Typing,       // "typing" - 正在输入

    // 状态变更
    StatusChange, // "status_change" - 用户状态变更（上线/下线/忙碌/离开）

    // 认证相关
    TokenRefresh, // "token_refresh" - 刷新Token
    RefreshOk,    // "refresh_ok" - Token刷新成功

    // 错误
    Error,        // "error" - 错误消息
}

/// 消息发送请求
#[derive(Debug, Deserialize, Validate)]
pub struct SendMessageRequest {
    pub conversation_id: Uuid,
    #[validate(length(min = 1))]
    pub content: String,
    pub message_type: Option<String>, // text, image, audio, video
    pub reply_to: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
}

/// 消息发送响应
#[derive(Debug, Serialize)]
pub struct SendMessageResponse {
    pub message_id: Uuid,
    pub conversation_id: Uuid,
    pub content: String,
    pub message_type: String,
    pub sender_id: Uuid,
    pub created_at: i64,
}

/// 创建对话请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateConversationRequest {
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(length(min = 1))]
    pub participant_ids: Vec<Uuid>,
    pub is_group: bool,
    pub description: Option<String>,
}

/// 创建对话响应
#[derive(Debug, Serialize)]
pub struct CreateConversationResponse {
    pub conversation_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_group: bool,
    pub participants: Vec<ParticipantInfo>,
    pub created_at: i64,
}

/// 对话信息
#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationInfo {
    pub conversation_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_group: bool,
    pub avatar_url: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_message_at: Option<i64>,
    pub participant_count: i32,
}

/// 参与者信息
#[derive(Debug, Serialize, Deserialize)]
pub struct ParticipantInfo {
    pub user_id: Uuid,
    pub username: String,
    pub avatar_url: Option<String>,
    pub role: String, // owner, admin, member
    pub joined_at: i64,
}

/// 对话列表响应
#[derive(Debug, Serialize)]
pub struct ConversationsListResponse {
    pub conversations: Vec<ConversationInfo>,
    pub total: i32,
}

/// 消息历史请求
#[derive(Debug, Deserialize)]
pub struct MessageHistoryRequest {
    pub conversation_id: Uuid,
    pub limit: Option<i32>,
    pub before_message_id: Option<Uuid>,
}

/// 消息历史响应
#[derive(Debug, Serialize)]
pub struct MessageHistoryResponse {
    pub conversation_id: Uuid,
    pub messages: Vec<MessageInfo>,
    pub has_more: bool,
    pub total: i32,
}

/// 消息信息
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageInfo {
    pub message_id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub sender_username: String,
    pub sender_avatar_url: Option<String>,
    pub content: String,
    pub message_type: String,
    pub reply_to: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: i64,
    pub read_at: Option<i64>,
    pub delivered_at: Option<i64>,
}

/// 标记已读请求
#[derive(Debug, Deserialize)]
pub struct MarkReadRequest {
    pub conversation_id: Uuid,
    pub message_id: Uuid,
}

/// 消息历史查询参数
#[derive(Debug, Deserialize)]
pub struct MessageHistoryQuery {
    pub conversation_id: Uuid,
    pub limit: Option<i32>,
    pub before_message_id: Option<Uuid>,
}

/// 编辑消息请求
#[derive(Debug, Deserialize)]
pub struct EditMessageRequest {
    pub conversation_id: Uuid,
    pub message_id: Uuid,
    pub content: String,
}

/// 撤回消息请求
#[derive(Debug, Deserialize)]
pub struct RecallMessageRequest {
    pub conversation_id: Uuid,
    pub message_id: Uuid,
}

/// 对话列表查询参数
#[derive(Debug, Deserialize)]
pub struct ConversationsQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// 在线用户响应
#[derive(Debug, Serialize)]
pub struct OnlineUsersResponse {
    pub online_users: Vec<OnlineUserInfo>,
    pub total: i32,
}

/// 在线用户信息
#[derive(Debug, Serialize, Deserialize)]
pub struct OnlineUserInfo {
    pub user_id: Uuid,
    pub username: String,
    pub avatar_url: Option<String>,
    pub status: String, // online, away, busy, offline
    pub last_seen: i64,
}

/// 输入状态请求
#[derive(Debug, Deserialize)]
pub struct TypingRequest {
    pub conversation_id: Uuid,
    pub is_typing: bool,
}

/// 状态变更请求 (通过 WebSocket 发送)
#[derive(Debug, Deserialize)]
pub struct StatusChangeRequest {
    pub status: String,  // online, away, busy, offline
}

/// Token 刷新请求
#[derive(Debug, Deserialize)]
pub struct TokenRefreshRequest {
    pub token: String,  // 新的 access token
}

/// 批量状态查询请求
#[derive(Debug, Deserialize)]
pub struct BatchStatusQuery {
    pub user_ids: Vec<Uuid>,
}

/// 批量状态查询响应
#[derive(Debug, Serialize)]
pub struct BatchStatusResponse {
    pub statuses: Vec<UserStatusItem>,
}

/// 用户状态项
#[derive(Debug, Serialize)]
pub struct UserStatusItem {
    pub user_id: Uuid,
    pub status: String,
    pub last_seen: i64,
}
#[cfg(test)]
#[path = "models_test.rs"]
mod tests;
