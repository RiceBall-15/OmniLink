use super::{
    AIProvider, AIMessage, MessageRole, ChatOptions, ChatCompletion, StreamChunk
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use futures::StreamExt;

/// 智谱AI API 请求结构
#[derive(Debug, Serialize)]
struct ZhipuRequest {
    model: String,
    messages: Vec<ZhipuMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ZhipuMessage {
    role: String,
    content: String,
}

/// 智谱AI API 响应结构
#[derive(Debug, Deserialize)]
struct ZhipuResponse {
    choices: Vec<ZhipuChoice>,
    usage: Option<ZhipuUsage>,
    model: String,
}

#[derive(Debug, Deserialize)]
struct ZhipuChoice {
    message: Option<ZhipuMessageContent>,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ZhipuMessageContent {
    content: String,
}

#[derive(Debug, Deserialize)]
struct ZhipuUsage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

/// 流式响应结构
#[derive(Debug, Deserialize)]
struct ZhipuStreamResponse {
    choices: Vec<ZhipuStreamChoice>,
    _model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ZhipuStreamChoice {
    delta: ZhipuStreamDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ZhipuStreamDelta {
    content: Option<String>,
}

/// 智谱AI Provider
pub struct ZhipuProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl ZhipuProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: base_url.unwrap_or_else(|| {
                "https://open.bigmodel.cn/api/paas/v4".to_string()
            }),
        }
    }

    fn convert_messages(messages: &[AIMessage]) -> Vec<ZhipuMessage> {
        messages
            .iter()
            .map(|msg| {
                let role = match msg.role {
                    MessageRole::System => "system".to_string(),
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                };
                ZhipuMessage {
                    role,
                    content: msg.content.clone(),
                }
            })
            .collect()
    }
}

#[async_trait]
impl AIProvider for ZhipuProvider {
    fn name(&self) -> &str {
        "ZhipuAI"
    }

    async fn chat_completion(
        &self,
        messages: &[AIMessage],
        options: &ChatOptions,
    ) -> Result<ChatCompletion, Box<dyn std::error::Error + Send + Sync>> {
        let request = ZhipuRequest {
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
            return Err(format!("智谱AI API 错误: {}", error_text).into());
        }

        let result: ZhipuResponse = response.json().await?;
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
        let request = ZhipuRequest {
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
            return Err(format!("智谱AI API 错误: {}", error_text).into());
        }

        let stream = response.bytes_stream();
        let model = options.model.clone();

        let output_stream = stream.filter_map(move |result| {
            let model = model.clone();
            async move {
                match result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        let lines: Vec<&str> = text.split('\n').collect();
                        for line in lines {
                            if line.starts_with("data: ") {
                                let data = &line[6..];
                                if data.trim() == "[DONE]" {
                                    return Some(Ok(StreamChunk {
                                        content: String::new(),
                                        done: true,
                                        model: model.clone(),
                                    }));
                                }
                                if let Ok(resp) = serde_json::from_str::<ZhipuStreamResponse>(data) {
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
        // 智谱AI定价（元/千tokens）
        let (input_price, output_price) = match model {
            "glm-4" => (0.1, 0.1),
            "glm-4-air" => (0.001, 0.001),
            "glm-4-airx" => (0.01, 0.01),
            "glm-4-flash" => (0.0001, 0.0001),
            "glm-4v" => (0.1, 0.1),
            "chatglm_turbo" => (0.001, 0.001),
            "chatglm_pro" => (0.01, 0.01),
            "chatglm_std" => (0.005, 0.005),
            "chatglm_lite" => (0.0001, 0.0001),
            _ => (0.001, 0.001), // 默认价格
        };

        (prompt_tokens as f64 / 1000.0) * input_price
            + (completion_tokens as f64 / 1000.0) * output_price
    }
}
