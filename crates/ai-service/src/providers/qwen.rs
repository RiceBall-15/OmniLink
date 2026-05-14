use super::{
    AIProvider, AIMessage, MessageRole, ChatOptions, ChatCompletion, StreamChunk
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use futures::StreamExt;

/// 通义千问 API 请求结构
#[derive(Debug, Serialize)]
struct QwenRequest {
    model: String,
    messages: Vec<QwenMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct QwenMessage {
    role: String,
    content: String,
}

/// 通义千问 API 响应结构
#[derive(Debug, Deserialize)]
struct QwenResponse {
    choices: Vec<QwenChoice>,
    usage: Option<QwenUsage>,
    model: String,
}

#[derive(Debug, Deserialize)]
struct QwenChoice {
    message: Option<QwenMessageContent>,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QwenMessageContent {
    content: String,
}

#[derive(Debug, Deserialize)]
struct QwenUsage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

/// 流式响应结构
#[derive(Debug, Deserialize)]
struct QwenStreamResponse {
    choices: Vec<QwenStreamChoice>,
    _model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QwenStreamChoice {
    delta: QwenStreamDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QwenStreamDelta {
    content: Option<String>,
}

/// 通义千问 Provider
pub struct QwenProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl QwenProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: base_url.unwrap_or_else(|| {
                "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string()
            }),
        }
    }

    fn convert_messages(messages: &[AIMessage]) -> Vec<QwenMessage> {
        messages
            .iter()
            .map(|msg| {
                let role = match msg.role {
                    MessageRole::System => "system".to_string(),
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                };
                QwenMessage {
                    role,
                    content: msg.content.clone(),
                }
            })
            .collect()
    }
}

#[async_trait]
impl AIProvider for QwenProvider {
    fn name(&self) -> &str {
        "Qwen"
    }

    async fn chat_completion(
        &self,
        messages: &[AIMessage],
        options: &ChatOptions,
    ) -> Result<ChatCompletion, Box<dyn std::error::Error + Send + Sync>> {
        let request = QwenRequest {
            model: options.model.clone(),
            messages: Self::convert_messages(messages),
            temperature: options.temperature,
            max_tokens: options.max_tokens,
            top_p: options.top_p,
            stream: false,
        };

        let url = format!("{}/chat/completions", self.base_url);
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("通义千问 API 错误: {}", error_text).into());
        }

        let result: QwenResponse = response.json().await?;
        let choice = result.choices.first().ok_or("没有响应选项")?;
        let usage = result.usage.ok_or("没有使用数据")?;

        Ok(ChatCompletion {
            content: choice.message.as_ref().map(|m| m.content.clone()).unwrap_or_default(),
            model: result.model,
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
            finish_reason: choice.finish_reason.clone().unwrap_or_else(|| "unknown".to_string()),
        })
    }

    async fn chat_completion_stream(
        &self,
        messages: &[AIMessage],
        options: &ChatOptions,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<StreamChunk, Box<dyn std::error::Error + Send + Sync>>> + Send>>, Box<dyn std::error::Error + Send + Sync>> {
        let request = QwenRequest {
            model: options.model.clone(),
            messages: Self::convert_messages(messages),
            temperature: options.temperature,
            max_tokens: options.max_tokens,
            top_p: options.top_p,
            stream: true,
        };

        let url = format!("{}/chat/completions", self.base_url);
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("通义千问 API 错误: {}", error_text).into());
        }

        let stream = response.bytes_stream();
        let model = options.model.clone();

        let output_stream = stream.filter_map(move |result| {
            let model = model.clone();
            async move {
                match result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        // 解析 SSE 格式
                        let lines: Vec<&str> = text.split('\n').collect();
                        for line in lines {
                            if let Some(data) = line.strip_prefix("data: ") {
                                if data.trim() == "[DONE]" {
                                    return Some(Ok(StreamChunk {
                                        content: String::new(),
                                        done: true,
                                        model: model.clone(),
                                    }));
                                }
                                if let Ok(resp) = serde_json::from_str::<QwenStreamResponse>(data) {
                                    if let Some(choice) = resp.choices.first() {
                                        let content = choice.delta.content.clone().unwrap_or_default();
                                        let done = choice.finish_reason.is_some();
                                        return Some(Ok(StreamChunk {
                                            content,
                                            done,
                                            model: model.clone(),
                                        }));
                                    }
                                }
                            }
                        }
                        None
                    }
                    Err(e) => Some(Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)),
                }
            }
        });

        Ok(Box::pin(output_stream))
    }

    fn count_tokens(&self, text: &str, _model: &str) -> i32 {
        // 简单估算：中文约1.5字符/token，英文约4字符/token
        let chinese_chars = text.chars().filter(|c| *c as u32 > 0x4E00).count() as f64;
        let other_chars = text.len() as f64 - chinese_chars;
        (chinese_chars / 1.5 + other_chars / 4.0) as i32
    }

    fn calculate_cost(&self, prompt_tokens: i32, completion_tokens: i32, model: &str) -> f64 {
        // 通义千问定价（元/千tokens）
        let (input_price, output_price) = match model {
            "qwen-turbo" => (0.002, 0.006),
            "qwen-plus" => (0.004, 0.012),
            "qwen-max" => (0.02, 0.06),
            "qwen-max-longcontext" => (0.02, 0.06),
            _ => (0.002, 0.006), // 默认 qwen-turbo 价格
        };

        (prompt_tokens as f64 / 1000.0) * input_price
            + (completion_tokens as f64 / 1000.0) * output_price
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{AIMessage, MessageRole};

    // === convert_messages 测试 ===

    #[test]
    fn test_convert_messages_user() {
        let messages = vec![
            AIMessage {
                role: MessageRole::User,
                content: "Hello".to_string(),
            }
        ];

        let result = QwenProvider::convert_messages(&messages);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, "user");
        assert_eq!(result[0].content, "Hello");
    }

    #[test]
    fn test_convert_messages_assistant() {
        let messages = vec![
            AIMessage {
                role: MessageRole::Assistant,
                content: "Hi!".to_string(),
            }
        ];

        let result = QwenProvider::convert_messages(&messages);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, "assistant");
    }

    #[test]
    fn test_convert_messages_system() {
        let messages = vec![
            AIMessage {
                role: MessageRole::System,
                content: "You are a helpful assistant".to_string(),
            }
        ];

        let result = QwenProvider::convert_messages(&messages);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, "system");
    }

    // === calculate_cost 测试 ===

    #[test]
    fn test_calculate_cost_qwen_turbo() {
        let provider = QwenProvider::new("test-key".to_string(), None);
        let cost = provider.calculate_cost(1000, 500, "qwen-turbo");
        // input: 1000/1000 * 0.002 = 0.002
        // output: 500/1000 * 0.006 = 0.003
        // total: 0.005
        assert!((cost - 0.005).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_cost_qwen_max() {
        let provider = QwenProvider::new("test-key".to_string(), None);
        let cost = provider.calculate_cost(1000, 1000, "qwen-max");
        // input: 1000/1000 * 0.02 = 0.02
        // output: 1000/1000 * 0.06 = 0.06
        // total: 0.08
        assert!((cost - 0.08).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_cost_unknown_model() {
        let provider = QwenProvider::new("test-key".to_string(), None);
        let cost = provider.calculate_cost(1000, 1000, "unknown-model");
        // 应使用默认价格 (0.002, 0.006)
        // input: 1000/1000 * 0.002 = 0.002
        // output: 1000/1000 * 0.006 = 0.006
        // total: 0.008
        assert!((cost - 0.008).abs() < f64::EPSILON);
    }

    // === count_tokens 测试 ===

    #[test]
    fn test_count_tokens_english() {
        let provider = QwenProvider::new("test-key".to_string(), None);
        let tokens = provider.count_tokens("Hello, world!", "qwen-turbo");
        assert!(tokens > 0);
    }

    #[test]
    fn test_count_tokens_chinese() {
        let provider = QwenProvider::new("test-key".to_string(), None);
        let tokens = provider.count_tokens("你好世界", "qwen-turbo");
        assert!(tokens > 0);
    }

    #[test]
    fn test_count_tokens_empty() {
        let provider = QwenProvider::new("test-key".to_string(), None);
        let tokens = provider.count_tokens("", "qwen-turbo");
        assert_eq!(tokens, 0);
    }

    // === name 测试 ===

    #[test]
    fn test_provider_name() {
        let provider = QwenProvider::new("test-key".to_string(), None);
        assert_eq!(provider.name(), "Qwen");
    }
}
