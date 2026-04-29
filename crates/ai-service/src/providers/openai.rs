use super::super::providers::{
    AIProvider, AIMessage, MessageRole, ChatOptions, ChatCompletion, StreamChunk
};
use async_trait::async_trait;
use async_openai::{
    types::{
        ChatCompletionRequestMessage,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::{Stream, StreamExt, TryStreamExt};
use std::sync::Arc;

/// OpenAI提供商
pub struct OpenAIProvider {
    client: Client<String>,
    api_key: String,
    base_url: String,
}

impl OpenAIProvider {
    /// 创建OpenAI提供商
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        let base_url = base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        Self {
            client: Client::with_config(
                async_openai::config::OpenAIConfig::new()
                    .with_api_key(&api_key)
                    .with_api_base(&base_url)
            ),
            api_key,
            base_url,
        }
    }

    /// 将内部消息转换为OpenAI消息
    fn convert_messages(messages: &[AIMessage]) -> Vec<ChatCompletionRequestMessage> {
        messages
            .iter()
            .map(|msg| match msg.role {
                MessageRole::System => {
                    ChatCompletionRequestMessage::System(
                        async_openai::types::System {
                            content: msg.content.clone(),
                            name: None,
                        }
                    )
                }
                MessageRole::User => {
                    ChatCompletionRequestMessage::User(
                        async_openai::types::User {
                            content: msg.content.clone(),
                            name: None,
                        }
                    )
                }
                MessageRole::Assistant => {
                    ChatCompletionRequestMessage::Assistant(
                        async_openai::types::Assistant {
                            content: Some(msg.content.clone()),
                            name: None,
                            function_call: None,
                            tool_calls: None,
                        }
                    )
                }
            })
            .collect()
    }
}

#[async_trait]
impl AIProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "OpenAI"
    }

    async fn chat_completion(
        &self,
        messages: &[AIMessage],
        options: &ChatOptions,
    ) -> Result<ChatCompletion, Box<dyn std::error::Error + Send + Sync>> {
        let openai_messages = Self::convert_messages(messages);

        let mut request = CreateChatCompletionRequestArgs::default()
            .model(options.model.clone())
            .messages(openai_messages);

        if let Some(temp) = options.temperature {
            request = request.temperature(temp);
        }
        if let Some(max_tokens) = options.max_tokens {
            request = request.max_tokens(max_tokens as u16);
        }
        if let Some(top_p) = options.top_p {
            request = request.top_p(top_p);
        }
        if let Some(presence_penalty) = options.presence_penalty {
            request = request.presence_penalty(presence_penalty);
        }
        if let Some(frequency_penalty) = options.frequency_penalty {
            request = request.frequency_penalty(frequency_penalty);
        }

        let request = request.build()?;

        let response = self.client.chat().create(request).await?;

        let choice = response.choices.first().ok_or("No response")?;
        let usage = response.usage.ok_or("No usage data")?;

        Ok(ChatCompletion {
            content: choice.message.content.clone().unwrap_or_default(),
            model: response.model,
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
            finish_reason: choice.finish_reason.clone().unwrap_or_else(|| "stop".to_string()),
        })
    }

    async fn chat_completion_stream(
        &self,
        messages: &[AIMessage],
        options: &ChatOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, Box<dyn std::error::Error + Send + Sync>>> + Send>>, Box<dyn std::error::Error + Send + Sync>> {
        let openai_messages = Self::convert_messages(messages);

        let mut request = CreateChatCompletionRequestArgs::default()
            .model(options.model.clone())
            .messages(openai_messages);

        if let Some(temp) = options.temperature {
            request = request.temperature(temp);
        }
        if let Some(max_tokens) = options.max_tokens {
            request = request.max_tokens(max_tokens as u16);
        }
        if let Some(top_p) = options.top_p {
            request = request.top_p(top_p);
        }
        if let Some(presence_penalty) = options.presence_penalty {
            request = request.presence_penalty(presence_penalty);
        }
        if let Some(frequency_penalty) = options.frequency_penalty {
            request = request.frequency_penalty(frequency_penalty);
        }

        let request = request.build()?;

        let stream = self.client.chat().create_stream(request).await?;

        let model = options.model.clone();

        let output_stream = stream.map(move |result| {
            match result {
                Ok(chunk) => {
                    if let Some(choice) = chunk.choices.first() {
                        let content = choice.delta.content.clone().unwrap_or_default();
                        let done = choice.finish_reason.is_some();

                        Ok(StreamChunk {
                            content,
                            done,
                            model: model.clone(),
                        })
                    } else {
                        Ok(StreamChunk {
                            content: String::new(),
                            done: true,
                            model: model.clone(),
                        })
                    }
                }
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
            }
        });

        Ok(Box::pin(output_stream))
    }

    fn count_tokens(&self, text: &str, model: &str) -> i32 {
        // 使用tiktoken计算token数
        use tiktoken_rs::cl100k_base;

        let bpe = cl100k_base().unwrap();
        let tokens = bpe.encode_with_special_tokens(text);

        tokens.len() as i32
    }

    fn calculate_cost(&self, prompt_tokens: i32, completion_tokens: i32, model: &str) -> f64 {
        // OpenAI定价 (每1000 tokens)
        let (input_price, output_price) = match model {
            "gpt-4" | "gpt-4-0314" => (0.03, 0.06),
            "gpt-4-32k" | "gpt-4-32k-0314" => (0.06, 0.12),
            "gpt-3.5-turbo" | "gpt-3.5-turbo-0301" => (0.0015, 0.002),
            "gpt-3.5-turbo-16k" => (0.003, 0.004),
            _ => (0.0015, 0.002), // 默认价格
        };

        let input_cost = (prompt_tokens as f64 / 1000.0) * input_price;
        let output_cost = (completion_tokens as f64 / 1000.0) * output_price;

        input_cost + output_cost
    }
}