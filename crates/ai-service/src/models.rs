use serde::{Deserialize, Serialize};
use validator::Validate;
use uuid::Uuid;
use chrono::Utc;

/// AI对话请求
#[derive(Debug, Deserialize, Validate)]
pub struct ChatRequest {
    #[validate(length(min = 1))]
    pub conversation_id: Uuid,

    #[validate(length(min = 1))]
    pub assistant_id: Uuid,

    #[validate(length(min = 1))]
    pub message: String,

    pub stream: Option<bool>,

    pub temperature: Option<f32>,

    pub max_tokens: Option<i32>,

    pub model_id: Option<String>,
}

/// AI对话响应
#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub conversation_id: Uuid,
    pub assistant_id: Uuid,
    pub message_id: Uuid,
    pub content: String,
    pub model: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub estimated_cost: f64,
    pub created_at: i64,
}

/// 流式AI对话响应
#[derive(Debug, Serialize)]
pub struct ChatStreamResponse {
    pub conversation_id: Uuid,
    pub assistant_id: Uuid,
    pub message_id: Uuid,
    pub content: String,
    pub delta: Option<String>,
    pub done: bool,
    pub model: String,
}

/// 创建AI助手请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateAssistantRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,

    pub description: Option<String>,

    #[validate(length(min = 1))]
    pub model_id: String,

    pub system_prompt: Option<String>,

    pub temperature: Option<f32>,

    pub max_tokens: Option<i32>,
}

/// 创建AI助手响应
#[derive(Debug, Serialize)]
pub struct CreateAssistantResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub model_id: String,
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub created_at: i64,
}

/// 获取AI助手列表响应
#[derive(Debug, Serialize)]
pub struct AssistantsListResponse {
    pub assistants: Vec<AssistantInfo>,
}

/// AI助手信息
#[derive(Debug, Serialize, Deserialize)]
pub struct AssistantInfo {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub model_id: String,
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub created_at: i64,
}

/// 更新AI助手请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateAssistantRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,

    pub description: Option<String>,

    pub model_id: Option<String>,

    pub system_prompt: Option<String>,

    pub temperature: Option<f32>,

    pub max_tokens: Option<i32>,
}

/// 消息历史记录
#[derive(Debug, Serialize)]
pub struct MessageHistory {
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

/// 对话历史响应
#[derive(Debug, Serialize)]
pub struct ConversationHistoryResponse {
    pub conversation_id: Uuid,
    pub assistant_id: Uuid,
    pub messages: Vec<MessageHistory>,
    pub total_messages: i32,
}

/// Token使用统计
#[derive(Debug, Serialize)]
pub struct TokenUsageResponse {
    pub total_tokens: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub estimated_cost: f64,
    pub request_count: i64,
    pub models: Vec<ModelUsage>,
}

/// 模型使用情况
#[derive(Debug, Serialize)]
pub struct ModelUsage {
    pub model_id: String,
    pub model_name: String,
    pub request_count: i64,
    pub total_tokens: i64,
    pub estimated_cost: f64,
}

/// 模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub api_base: String,
    pub max_tokens: i32,
    pub input_price_per_1k: f64,
    pub output_price_per_1k: f64,
}

/// 支持的模型列表
#[derive(Debug, Serialize)]
pub struct ModelsResponse {
    pub models: Vec<ModelConfig>,
}