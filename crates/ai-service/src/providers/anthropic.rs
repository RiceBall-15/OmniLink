use super::super::providers::{
    AIProvider, AIMessage, MessageRole, ChatOptions, ChatCompletion, StreamChunk
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use futures::{Stream, StreamExt};

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

    fn count_tokens(&self, text: &str, _model: &str) -> i32 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{AIMessage, MessageRole};

    // === convert_messages 测试 ===

    #[test]
    fn test_convert_messages_single_user() {
        let messages = vec![AIMessage {
            role: MessageRole::User,
            content: "Hello!".to_string(),
        }];

        let result = AnthropicProvider::convert_messages(&messages);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, "user");
        assert_eq!(result[0].content, "Hello!");
    }

    #[test]
    fn test_convert_messages_all_roles() {
        let messages = vec![
            AIMessage { role: MessageRole::System, content: "System prompt".to_string() },
            AIMessage { role: MessageRole::User, content: "User message".to_string() },
            AIMessage { role: MessageRole::Assistant, content: "Assistant response".to_string() },
        ];

        let result = AnthropicProvider::convert_messages(&messages);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[2].role, "assistant");
    }

    #[test]
    fn test_convert_messages_empty() {
        let messages: Vec<AIMessage> = vec![];
        let result = AnthropicProvider::convert_messages(&messages);
        assert!(result.is_empty());
    }

    #[test]
    fn test_convert_messages_preserves_content() {
        let messages = vec![
            AIMessage { role: MessageRole::User, content: "你好世界".to_string() },
            AIMessage { role: MessageRole::Assistant, content: "👋 你好！".to_string() },
        ];

        let result = AnthropicProvider::convert_messages(&messages);
        assert_eq!(result[0].content, "你好世界");
        assert_eq!(result[1].content, "👋 你好！");
    }

    // === calculate_cost 测试 ===

    #[test]
    fn test_calculate_cost_claude3_opus() {
        let provider = AnthropicProvider::new("test-key".to_string(), None);
        let cost = provider.calculate_cost(1000, 500, "claude-3-opus");
        // input: 1000/1000 * 0.015 = 0.015
        // output: 500/1000 * 0.075 = 0.0375
        // total: 0.0525
        assert!((cost - 0.0525).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_cost_claude3_sonnet() {
        let provider = AnthropicProvider::new("test-key".to_string(), None);
        let cost = provider.calculate_cost(1000, 1000, "claude-3-sonnet");
        // input: 1000/1000 * 0.003 = 0.003
        // output: 1000/1000 * 0.015 = 0.015
        // total: 0.018
        assert!((cost - 0.018).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_cost_claude3_haiku() {
        let provider = AnthropicProvider::new("test-key".to_string(), None);
        let cost = provider.calculate_cost(1000, 1000, "claude-3-haiku");
        // input: 1000/1000 * 0.00025 = 0.00025
        // output: 1000/1000 * 0.00125 = 0.00125
        // total: 0.0015
        assert!((cost - 0.0015).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_cost_unknown_model() {
        let provider = AnthropicProvider::new("test-key".to_string(), None);
        let cost = provider.calculate_cost(1000, 1000, "unknown-model");
        // 应使用默认价格 (0.003, 0.015)
        assert!((cost - 0.018).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_cost_zero_tokens() {
        let provider = AnthropicProvider::new("test-key".to_string(), None);
        let cost = provider.calculate_cost(0, 0, "claude-3-opus");
        assert!((cost - 0.0).abs() < f64::EPSILON);
    }

    // === count_tokens 测试 ===

    #[test]
    fn test_count_tokens_english() {
        let provider = AnthropicProvider::new("test-key".to_string(), None);
        let tokens = provider.count_tokens("Hello, world!", "claude-3-opus");
        assert!(tokens > 0);
    }

    #[test]
    fn test_count_tokens_chinese() {
        let provider = AnthropicProvider::new("test-key".to_string(), None);
        let tokens = provider.count_tokens("你好世界", "claude-3-opus");
        assert!(tokens > 0);
    }

    #[test]
    fn test_count_tokens_empty() {
        let provider = AnthropicProvider::new("test-key".to_string(), None);
        let tokens = provider.count_tokens("", "claude-3-opus");
        assert_eq!(tokens, 0);
    }

    #[test]
    fn test_count_tokens_longer_text() {
        let provider = AnthropicProvider::new("test-key".to_string(), None);
        let short = provider.count_tokens("Hello", "claude-3-opus");
        let long = provider.count_tokens("Hello, this is a longer text for testing token counting", "claude-3-opus");
        assert!(long > short);
    }

    // === name 测试 ===

    #[test]
    fn test_provider_name() {
        let provider = AnthropicProvider::new("test-key".to_string(), None);
        assert_eq!(provider.name(), "Anthropic");
    }

    // === new 测试 ===

    #[test]
    fn test_new_with_custom_base_url() {
        let provider = AnthropicProvider::new(
            "test-key".to_string(),
            Some("https://custom-api.example.com".to_string()),
        );
        assert_eq!(provider.name(), "Anthropic");
    }

    #[test]
    fn test_new_with_default_base_url() {
        let provider = AnthropicProvider::new("test-key".to_string(), None);
        assert_eq!(provider.name(), "Anthropic");
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
    _block_type: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: i32,
    output_tokens: i32,
}