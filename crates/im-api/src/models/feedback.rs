//! 用户反馈系统模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 反馈类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum FeedbackType {
    Bug,
    Feature,
    Other,
}

impl std::fmt::Display for FeedbackType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeedbackType::Bug => write!(f, "bug"),
            FeedbackType::Feature => write!(f, "feature"),
            FeedbackType::Other => write!(f, "other"),
        }
    }
}

impl std::str::FromStr for FeedbackType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bug" => Ok(FeedbackType::Bug),
            "feature" => Ok(FeedbackType::Feature),
            "other" => Ok(FeedbackType::Other),
            _ => Err(format!("无效的反馈类型: {}", s)),
        }
    }
}

/// 反馈状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum FeedbackStatus {
    Pending,
    Processing,
    Resolved,
    Rejected,
}

impl std::fmt::Display for FeedbackStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeedbackStatus::Pending => write!(f, "pending"),
            FeedbackStatus::Processing => write!(f, "processing"),
            FeedbackStatus::Resolved => write!(f, "resolved"),
            FeedbackStatus::Rejected => write!(f, "rejected"),
        }
    }
}

impl std::str::FromStr for FeedbackStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(FeedbackStatus::Pending),
            "processing" => Ok(FeedbackStatus::Processing),
            "resolved" => Ok(FeedbackStatus::Resolved),
            "rejected" => Ok(FeedbackStatus::Rejected),
            _ => Err(format!("无效的反馈状态: {}", s)),
        }
    }
}

/// 反馈优先级枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum FeedbackPriority {
    Low,
    Medium,
    High,
    Urgent,
}

impl std::fmt::Display for FeedbackPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeedbackPriority::Low => write!(f, "low"),
            FeedbackPriority::Medium => write!(f, "medium"),
            FeedbackPriority::High => write!(f, "high"),
            FeedbackPriority::Urgent => write!(f, "urgent"),
        }
    }
}

impl std::str::FromStr for FeedbackPriority {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(FeedbackPriority::Low),
            "medium" => Ok(FeedbackPriority::Medium),
            "high" => Ok(FeedbackPriority::High),
            "urgent" => Ok(FeedbackPriority::Urgent),
            _ => Err(format!("无效的优先级: {}", s)),
        }
    }
}

/// 用户反馈数据库实体
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserFeedbackEntity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub feedback_type: String,
    pub content: String,
    pub contact_email: Option<String>,
    pub status: String,
    pub priority: String,
    pub admin_reply: Option<String>,
    pub replied_by: Option<Uuid>,
    pub replied_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 用户反馈 API 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFeedback {
    pub id: String,
    pub user_id: String,
    pub feedback_type: String,
    pub content: String,
    pub contact_email: Option<String>,
    pub status: String,
    pub priority: String,
    pub admin_reply: Option<String>,
    pub replied_by: Option<String>,
    pub replied_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl UserFeedbackEntity {
    pub fn to_user_feedback(&self) -> UserFeedback {
        UserFeedback {
            id: self.id.to_string(),
            user_id: self.user_id.to_string(),
            feedback_type: self.feedback_type.clone(),
            content: self.content.clone(),
            contact_email: self.contact_email.clone(),
            status: self.status.clone(),
            priority: self.priority.clone(),
            admin_reply: self.admin_reply.clone(),
            replied_by: self.replied_by.map(|id| id.to_string()),
            replied_at: self.replied_at.map(|t| t.to_rfc3339()),
            created_at: self.created_at.to_rfc3339(),
            updated_at: self.updated_at.to_rfc3339(),
        }
    }
}

/// 创建反馈请求
#[derive(Debug, Deserialize)]
pub struct CreateFeedbackRequest {
    pub feedback_type: String,
    pub content: String,
    pub contact_email: Option<String>,
    pub priority: Option<String>,
}

/// 更新反馈状态请求（管理员）
#[derive(Debug, Deserialize)]
pub struct UpdateFeedbackRequest {
    pub status: Option<String>,
    pub priority: Option<String>,
    pub admin_reply: Option<String>,
}

/// 反馈查询参数
#[derive(Debug, Deserialize)]
pub struct FeedbackQuery {
    pub feedback_type: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 反馈统计
#[derive(Debug, Serialize)]
pub struct FeedbackStats {
    pub total: i64,
    pub pending: i64,
    pub processing: i64,
    pub resolved: i64,
    pub rejected: i64,
    pub by_type: FeedbackTypeStats,
}

#[derive(Debug, Serialize)]
pub struct FeedbackTypeStats {
    pub bug: i64,
    pub feature: i64,
    pub other: i64,
}
