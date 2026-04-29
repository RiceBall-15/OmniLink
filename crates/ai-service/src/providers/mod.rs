pub mod openai;
pub mod anthropic;
pub mod google;

pub use openai::OpenAIProvider;
pub use anthropic::AnthropicProvider;
pub use google::GoogleProvider;

use async_trait::async_trait;
use std::pin::Pin;
use std::task::{Context, Poll};

/// 消息角色
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// AI消息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AIMessage {
    pub role: MessageRole,
    pub content: String,
}

/// AI对话选项
#[derive(Debug, Clone)]
pub struct ChatOptions {
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub top_p: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub frequency_penalty: Option<f32>,
}

impl Default for ChatOptions {
    fn default() -> Self {
        Self {
            model: "gpt-3.5-turbo".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(2048),
            top_p: Some(1.0),
            presence_penalty: Some(0.0),
            frequency_penalty: Some(0.0),
        }
    }
}

/// AI对话响应
#[derive(Debug, Clone)]
pub struct ChatCompletion {
    pub content: String,
    pub model: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub finish_reason: String,
}

/// 流式响应块
#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub content: String,
    pub done: bool,
    pub model: String,
}

/// AI提供商trait
#[async_trait]
pub trait AIProvider: Send + Sync {
    /// 获取提供商名称
    fn name(&self) -> &str;

    /// 发送对话请求
    async fn chat_completion(
        &self,
        messages: &[AIMessage],
        options: &ChatOptions,
    ) -> Result<ChatCompletion, Box<dyn std::error::Error + Send + Sync>>;

    /// 发送流式对话请求
    async fn chat_completion_stream(
        &self,
        messages: &[AIMessage],
        options: &ChatOptions,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<StreamChunk, Box<dyn std::error::Error + Send + Sync>>> + Send>>, Box<dyn std::error::Error + Send + Sync>>;

    /// 计算Token数量
    fn count_tokens(&self, text: &str, model: &str) -> i32;

    /// 计算费用
    fn calculate_cost(&self, prompt_tokens: i32, completion_tokens: i32, model: &str) -> f64;
}