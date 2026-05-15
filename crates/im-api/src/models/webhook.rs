//! Webhook 集成框架数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Webhook 事件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
pub enum WebhookEventType {
    /// 新消息
    MessageCreated,
    /// 消息已读
    MessageRead,
    /// 用户上线
    UserOnline,
    /// 用户下线
    UserOffline,
    /// 新会话创建
    ConversationCreated,
    /// 反馈提交
    FeedbackSubmitted,
    /// 公告发布
    AnnouncementCreated,
}

impl std::fmt::Display for WebhookEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MessageCreated => write!(f, "message.created"),
            Self::MessageRead => write!(f, "message.read"),
            Self::UserOnline => write!(f, "user.online"),
            Self::UserOffline => write!(f, "user.offline"),
            Self::ConversationCreated => write!(f, "conversation.created"),
            Self::FeedbackSubmitted => write!(f, "feedback.submitted"),
            Self::AnnouncementCreated => write!(f, "announcement.created"),
        }
    }
}

impl std::str::FromStr for WebhookEventType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "message.created" => Ok(Self::MessageCreated),
            "message.read" => Ok(Self::MessageRead),
            "user.online" => Ok(Self::UserOnline),
            "user.offline" => Ok(Self::UserOffline),
            "conversation.created" => Ok(Self::ConversationCreated),
            "feedback.submitted" => Ok(Self::FeedbackSubmitted),
            "announcement.created" => Ok(Self::AnnouncementCreated),
            _ => Err(format!("未知事件类型: {}", s)),
        }
    }
}

/// Webhook 注册信息
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WebhookEntity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub url: String,
    pub secret: Option<String>,
    pub events: Vec<String>,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Webhook API 响应
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct Webhook {
    pub id: String,
    pub user_id: String,
    pub url: String,
    pub events: Vec<String>,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl WebhookEntity {
    pub fn to_api(&self) -> Webhook {
        Webhook {
            id: self.id.to_string(),
            user_id: self.user_id.to_string(),
            url: self.url.clone(),
            events: self.events.clone(),
            description: self.description.clone(),
            is_active: self.is_active,
            created_at: self.created_at.to_rfc3339(),
            updated_at: self.updated_at.to_rfc3339(),
        }
    }
}

/// 注册 Webhook 请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateWebhookRequest {
    /// 目标 URL（必须是 HTTPS）
    pub url: String,
    /// 可选签名密钥
    pub secret: Option<String>,
    /// 订阅的事件类型列表
    pub events: Vec<String>,
    /// 描述
    pub description: Option<String>,
}

/// 更新 Webhook 请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateWebhookRequest {
    pub url: Option<String>,
    pub secret: Option<String>,
    pub events: Option<Vec<String>>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

/// Webhook 投递日志实体
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WebhookDeliveryEntity {
    pub id: Uuid,
    pub webhook_id: Uuid,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub response_status: Option<i32>,
    pub response_body: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
    pub delivered_at: DateTime<Utc>,
}

/// Webhook 投递日志 API 响应
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WebhookDelivery {
    pub id: String,
    pub webhook_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub response_status: Option<i32>,
    pub success: bool,
    pub error_message: Option<String>,
    pub delivered_at: String,
}

impl WebhookDeliveryEntity {
    pub fn to_api(&self) -> WebhookDelivery {
        WebhookDelivery {
            id: self.id.to_string(),
            webhook_id: self.webhook_id.to_string(),
            event_type: self.event_type.clone(),
            payload: self.payload.clone(),
            response_status: self.response_status,
            success: self.success,
            error_message: self.error_message.clone(),
            delivered_at: self.delivered_at.to_rfc3339(),
        }
    }
}

/// Webhook 事件负载（发送到目标 URL 的 JSON 结构）
#[derive(Debug, Serialize)]
pub struct WebhookPayload {
    /// 事件类型
    pub event: String,
    /// 时间戳
    pub timestamp: String,
    /// 事件数据
    pub data: serde_json::Value,
}

/// Webhook 查询参数
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct WebhookQuery {
    pub is_active: Option<bool>,
}

/// Webhook 投递日志查询参数
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct WebhookDeliveryQuery {
    pub event_type: Option<String>,
    pub success: Option<bool>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}
