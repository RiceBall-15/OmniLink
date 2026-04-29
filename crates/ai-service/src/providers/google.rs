use super::super::providers::{
    AIProvider, AIMessage, MessageRole, ChatOptions, ChatCompletion, StreamChunk
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::{Stream, StreamExt, TryStreamExt};

/// Google提供商
pub struct GoogleProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl GoogleProvider {
    /// 创建Google提供商
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        let base_url = base_url.unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string());

        Self {
            client: Client::new(),
            api_key,
            base_url,
        }
    }

    /// 将内部消息转换为Google消息
    fn convert_messages(messages: &[AIMessage]) -> Vec<GoogleMessage> {
        messages
            .iter()
            .map(|msg| GoogleMessage {
                role: match msg.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "model".to_string(),
                    MessageRole::System => "user".to_string(), // Google将系统消息放在用户消息中
                },
                parts: vec![Part {
                    text: msg.content.clone(),
                }],
            })
            .collect()
    }
}

#[async_trait]
impl AIProvider for GoogleProvider {
    fn name(&self) -> &str {
        "Google"
    }

    async fn chat_completion(
        &self,
        messages: &[AIMessage],
        options: &ChatOptions,
    ) -> Result<ChatCompletion, Box<dyn std::error::Error + Send + Sync>> {
        let google_messages = Self::convert_messages(messages);

        let request = GoogleRequest {
            contents: google_messages,
            generation_config: Some(GenerationConfig {
                temperature: options.temperature,
                max_output_tokens: options.max_tokens,
                top_p: options.top_p,
            }),
        };

        let model_name = if options.model.starts_with("models/") {
            options.model.clone()
        } else {
            format!("models/{}", options.model)
        };

        let url = format!(
            "{}/v1beta/{}:generateContent?key={}",
            self.base_url, model_name, self.api_key
        );

        let response = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        let response_text = response.text().await?;

        let google_response: GoogleResponse = serde_json::from_str(&response_text)?;

        // 提取内容
        let content = google_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .unwrap_or_default();

        // 简化的token估算
        let prompt_tokens = messages.iter().map(|m| m.content.len() / 4).sum::<usize>() as i32;
        let completion_tokens = content.len() as i32 / 4;

        Ok(ChatCompletion {
            content,
            model: options.model.clone(),
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
            finish_reason: "stop".to_string(),
        })
    }

    async fn chat_completion_stream(
        &self,
        messages: &[AIMessage],
        options: &ChatOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, Box<dyn std::error::Error + Send + Sync>>> + Send>>, Box<dyn std::error::Error + Send + Sync>> {
        let google_messages = Self::convert_messages(messages);

        let request = GoogleRequest {
            contents: google_messages,
            generation_config: Some(GenerationConfig {
                temperature: options.temperature,
                max_output_tokens: options.max_tokens,
                top_p: options.top_p,
            }),
        };

        let model_name = if options.model.starts_with("models/") {
            options.model.clone()
        } else {
            format!("models/{}", options.model)
        };

        let url = format!(
            "{}/v1beta/{}:streamGenerateContent?key={}",
            self.base_url, model_name, self.api_key
        );

        let response = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        let byte_stream = response.bytes_stream();
        let model = options.model.clone();

        let output_stream = byte_stream.map(move |result| {
            match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
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
        // 简化版本，实际需要使用Google的tokenizer
        let chars = text.chars();
        (chars.count() as f64 * 0.3) as i32
    }

    fn calculate_cost(&self, prompt_tokens: i32, completion_tokens: i32, model: &str) -> f64 {
        // Google定价 (每1000 tokens)
        let (input_price, output_price) = match model {
            "gemini-pro" | "gemini-1.0-pro" => (0.0005, 0.0015),
            "gemini-pro-vision" | "gemini-1.0-pro-vision" => (0.00025, 0.0005),
            "gemini-ultra" => (0.0, 0.0), // 价格待定
            _ => (0.0005, 0.0015), // 默认价格
        };

        let input_cost = (prompt_tokens as f64 / 1000.0) * input_price;
        let output_cost = (completion_tokens as f64 / 1000.0) * output_price;

        input_cost + output_cost
    }
}

/// Google请求格式
#[derive(Debug, Serialize)]
struct GoogleRequest {
    contents: Vec<GoogleMessage>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

/// Google消息格式
#[derive(Debug, Serialize, Deserialize)]
struct GoogleMessage {
    role: String,
    parts: Vec<Part>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Part {
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(rename = "maxOutputTokens", skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
}

/// Google响应格式
#[derive(Debug, Deserialize)]
struct GoogleResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: GoogleContent,
}

#[derive(Debug, Deserialize)]
struct GoogleContent {
    parts: Vec<Part>,
}