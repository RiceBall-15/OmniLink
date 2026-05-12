use super::{
    AIProvider, AIMessage, MessageRole, ChatOptions, ChatCompletion, StreamChunk
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use futures::StreamExt;

/// 文心一言 API 请求结构
#[derive(Debug, Serialize)]
struct ErnieRequest {
    messages: Vec<ErnieMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ErnieMessage {
    role: String,
    content: String,
}

/// 文心一言 API 响应结构
#[derive(Debug, Deserialize)]
struct ErnieResponse {
    result: String,
    usage: Option<ErnieUsage>,
    #[serde(default)]
    error_code: Option<i32>,
    #[serde(default)]
    error_msg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ErnieUsage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

/// 流式响应结构
#[derive(Debug, Deserialize)]
struct ErnieStreamResponse {
    result: Option<String>,
    is_end: Option<bool>,
    usage: Option<ErnieUsage>,
    #[serde(default)]
    error_code: Option<i32>,
    #[serde(default)]
    error_msg: Option<String>,
}

/// 文心一言 Provider
pub struct ErnieProvider {
    client: Client,
    api_key: String,
    secret_key: String,
    base_url: String,
    access_token: Option<String>,
}

impl ErnieProvider {
    pub fn new(api_key: String, secret_key: String, base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            secret_key,
            base_url: base_url.unwrap_or_else(|| {
                "https://aip.baidubce.com".to_string()
            }),
            access_token: None,
        }
    }

    /// 获取 access_token
    async fn get_access_token(&mut self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(token) = &self.access_token {
            return Ok(token.clone());
        }

        let url = format!(
            "{}/oauth/2.0/token?grant_type=client_credentials&client_id={}&client_secret={}",
            self.base_url, self.api_key, self.secret_key
        );

        let response: serde_json::Value = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .send()
            .await?
            .json()
            .await?;

        let access_token = response["access_token"]
            .as_str()
            .ok_or("无法获取 access_token")?
            .to_string();

        self.access_token = Some(access_token.clone());
        Ok(access_token)
    }

    fn convert_messages(messages: &[AIMessage]) -> Vec<ErnieMessage> {
        messages
            .iter()
            .map(|msg| {
                let role = match msg.role {
                    MessageRole::System => "user".to_string(), // 文心一言不支持 system 角色
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                };
                ErnieMessage {
                    role,
                    content: msg.content.clone(),
                }
            })
            .collect()
    }

    fn get_endpoint(model: &str) -> &str {
        match model {
            "ernie-4.0-turbo-8k" => "completions/pro_2_0_8k",
            "ernie-4.0-8k" => "completions/eb_4_0_8k",
            "ernie-3.5-8k" => "completions/completions",
            "ernie-3.5-4k-0205" => "completions/ernie-3.5-4k-0205",
            "ernie-3.5-8k-0205" => "completions/ernie-3.5-8k-0205",
            "ernie-speed-8k" => "completions/ernie_speed",
            "ernie-speed-128k" => "completions/ernie-speed-128k",
            "ernie-lite-8k" => "completions/ernie_lite_8k",
            _ => "completions/completions", // 默认 ernie-3.5-8k
        }
    }
}

#[async_trait]
impl AIProvider for ErnieProvider {
    fn name(&self) -> &str {
        "Ernie"
    }

    async fn chat_completion(
        &mut self,
        messages: &[AIMessage],
        options: &ChatOptions,
    ) -> Result<ChatCompletion, Box<dyn std::error::Error + Send + Sync>> {
        let access_token = self.get_access_token().await?;
        let endpoint = Self::get_endpoint(&options.model);

        let request = ErnieRequest {
            messages: Self::convert_messages(messages),
            temperature: options.temperature,
            max_output_tokens: options.max_tokens,
            top_p: options.top_p,
            stream: false,
        };

        let url = format!(
            "{}/rpc/2.0/ai_custom/v1/wenxinworkshop/chat/{}?access_token={}",
            self.base_url, endpoint, access_token
        );

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("文心一言 API 错误: {}", error_text).into());
        }

        let result: ErnieResponse = response.json().await?;

        if let Some(error_code) = result.error_code {
            return Err(format!("文心一言错误 {}: {}", error_code, result.error_msg.unwrap_or_default()).into());
        }

        let usage = result.usage.ok_or("没有使用数据")?;

        Ok(ChatCompletion {
            content: result.result,
            model: options.model.clone(),
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
            finish_reason: "stop".to_string(),
        })
    }

    async fn chat_completion_stream(
        &mut self,
        messages: &[AIMessage],
        options: &ChatOptions,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<StreamChunk, Box<dyn std::error::Error + Send + Sync>>> + Send>>, Box<dyn std::error::Error + Send + Sync>> {
        let access_token = self.get_access_token().await?;
        let endpoint = Self::get_endpoint(&options.model);

        let request = ErnieRequest {
            messages: Self::convert_messages(messages),
            temperature: options.temperature,
            max_output_tokens: options.max_tokens,
            top_p: options.top_p,
            stream: true,
        };

        let url = format!(
            "{}/rpc/2.0/ai_custom/v1/wenxinworkshop/chat/{}?access_token={}",
            self.base_url, endpoint, access_token
        );

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("文心一言 API 错误: {}", error_text).into());
        }

        let stream = response.bytes_stream();
        let model = options.model.clone();

        let output_stream = stream.filter_map(move |result| {
            let model = model.clone();
            async move {
                match result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        // 文心一言流式响应是直接 JSON，不是 SSE 格式
                        if let Ok(resp) = serde_json::from_str::<ErnieStreamResponse>(&text) {
                            if let Some(error_code) = resp.error_code {
                                return Some(Err(format!("文心一言错误 {}: {}", error_code, resp.error_msg.unwrap_or_default()).into()));
                            }
                            let content = resp.result.unwrap_or_default();
                            let done = resp.is_end.unwrap_or(false);
                            Some(Ok(StreamChunk {
                                content,
                                done,
                                model,
                            }))
                        } else {
                            None
                        }
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
        // 文心一言定价（元/千tokens）
        let (input_price, output_price) = match model {
            "ernie-4.0-turbo-8k" => (0.02, 0.06),
            "ernie-4.0-8k" => (0.12, 0.12),
            "ernie-3.5-8k" => (0.008, 0.008),
            "ernie-3.5-4k-0205" => (0.004, 0.008),
            "ernie-3.5-8k-0205" => (0.004, 0.008),
            "ernie-speed-8k" => (0.001, 0.001),
            "ernie-speed-128k" => (0.001, 0.001),
            "ernie-lite-8k" => (0.001, 0.001),
            _ => (0.008, 0.008), // 默认 ernie-3.5-8k 价格
        };

        (prompt_tokens as f64 / 1000.0) * input_price
            + (completion_tokens as f64 / 1000.0) * output_price
    }
}
