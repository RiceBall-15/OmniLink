use super::super::providers::{
    AIProvider, AIMessage, MessageRole, ChatOptions, ChatCompletion, StreamChunk
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::{Stream, StreamExt, TryStreamExt};

/// Anthropic提供商
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl AnthropicProvider {
    /// 创建Anthropic提供商
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        let base_url = base_url.unwrap_or_else(|| "https://api.anthropic.com".to_string());

        Self {
            client: Client::new(),
            api_key,
            base_url,
        }
    }

    /// 将内部消息转换为Anthropic消息
    fn convert_messages(messages: &[AIMessage]) -> Vec<AnthropicMessage> {
        messages
            .iter()
            .filter_map(|msg| {
                match msg.role {
                    MessageRole::System => Some(AnthropicMessage {
                        role: "system".to_string(),
                        content: msg.content.clone(),
                    }),
                    MessageRole::User => Some(AnthropicMessage {
                        role: "user".to_string(),
                        content: msg.content.clone(),
                    }),
                    MessageRole::Assistant => Some(AnthropicMessage {
                        role: "assistant".to_string(),
                        content: msg.content.clone(),
                    }),
                }
            })
            .collect()
    }
}

#[async_trait]
impl AIProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "Anthropic"
    }

    async fn chat_completion(
        &self,
        messages: &[AIMessage],
        options: &ChatOptions,
    ) -> Result<ChatCompletion, Box<dyn std::error::Error + Send + Sync>> {
        let anthropic_messages = Self::convert_messages(messages);

        let request = AnthropicRequest {
            model: options.model.clone(),
            messages: anthropic_messages,
            max_tokens: options.max_tokens.unwrap_or(2048),
            temperature: options.temperature,
            top_p: options.top_p,
        };

        let response = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        let response_text = response.text().await?;

        let anthropic_response: AnthropicResponse = serde_json::from_str(&response_text)?;

        Ok(ChatCompletion {
            content: anthropic_response.content[0].text.clone(),
            model: options.model.clone(),
            prompt_tokens: anthropic_response.usage.input_tokens,
            completion_tokens: anthropic_response.usage.output_tokens,
            total_tokens: anthropic_response.usage.input_tokens + anthropic_response.usage.output_tokens,
            finish_reason: "stop".to_string(),
        })
    }

    async fn chat_completion_stream(
        &self,
        messages: &[AIMessage],
        options: &ChatOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, Box<dyn std::error::Error + Send + Sync>>> + Send>>, Box<dyn std::error::Error + Send + Sync>> {
        let anthropic_messages = Self::convert_messages(messages);

        let request = AnthropicRequest {
            model: options.model.clone(),
            messages: anthropic_messages,
            max_tokens: options.max_tokens.unwrap_or(2048),
            temperature: options.temperature,
            top_p: options.top_p,
        };

        let response = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        let byte_stream = response.bytes_stream();
        let model = options.model.clone();

        let output_stream = byte_stream.map(move |result| {
            match result {
                Ok(bytes) => {
                    // 解析SSE事件
                    let text = String::from_utf8_lossy(&bytes);
                    // 简化版本，实际需要解析SSE格式
                    Ok(StreamChunk {
                        content: text.to_string(),
                        done: false,
                        model: model.clone(),
                    })
                }
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
            }
        });

        Ok(Box::pin(output_stream))
    }

    fn count_tokens(&self, text: &str, model: &str) -> i32 {
        // Claude使用GPT-4类似的tokenizer
        // 简化版本，实际需要使用Claude的tokenizer
        let chars = text.chars();
        (chars.count() as f64 * 0.3) as i32
    }

    fn calculate_cost(&self, prompt_tokens: i32, completion_tokens: i32, model: &str) -> f64 {
        // Anthropic定价 (每1000 tokens)
        let (input_price, output_price) = match model {
            "claude-3-opus" => (0.015, 0.075),
            "claude-3-sonnet" => (0.003, 0.015),
            "claude-3-haiku" => (0.00025, 0.00125),
            "claude-2.1" => (0.008, 0.024),
            "claude-2.0" => (0.008, 0.024),
            _ => (0.003, 0.015), // 默认价格
        };

        let input_cost = (prompt_tokens as f64 / 1000.0) * input_price;
        let output_cost = (completion_tokens as f64 / 1000.0) * output_price;

        input_cost + output_cost
    }
}

/// Anthropic请求格式
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: i32,
    temperature: Option<f32>,
    top_p: Option<f32>,
}

/// Anthropic消息格式
#[derive(Debug, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

/// Anthropic响应格式
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: i32,
    output_tokens: i32,
}