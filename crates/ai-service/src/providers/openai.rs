use super::{
    AIProvider, AIMessage, MessageRole, ChatOptions, ChatCompletion, StreamChunk
};
use async_trait::async_trait;
use async_openai::{
    types::{
        ChatCompletionRequestMessage,
        CreateChatCompletionRequestArgs,
        Role,
    },
    Client,
    config::OpenAIConfig,
};
use std::pin::Pin;
use futures::StreamExt;

/// OpenAI provider
pub struct OpenAIProvider {
    client: Client<OpenAIConfig>,
}

impl OpenAIProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        let mut config = OpenAIConfig::new().with_api_key(&api_key);
        if let Some(base) = base_url {
            config = config.with_api_base(&base);
        }

        Self {
            client: Client::with_config(config),
        }
    }

    fn convert_messages(messages: &[AIMessage]) -> Vec<ChatCompletionRequestMessage> {
        messages
            .iter()
            .map(|msg| {
                let role = match msg.role {
                    MessageRole::System => Role::System,
                    MessageRole::User => Role::User,
                    MessageRole::Assistant => Role::Assistant,
                };
                ChatCompletionRequestMessage {
                    role,
                    content: Some(msg.content.clone()),
                    name: None,
                    function_call: None,
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

        let mut builder = CreateChatCompletionRequestArgs::default();
        builder.model(options.model.clone())
            .messages(openai_messages);

        if let Some(temp) = options.temperature {
            builder.temperature(temp);
        }
        if let Some(max_tokens) = options.max_tokens {
            builder.max_tokens(max_tokens as u16);
        }
        if let Some(top_p) = options.top_p {
            builder.top_p(top_p);
        }
        if let Some(presence_penalty) = options.presence_penalty {
            builder.presence_penalty(presence_penalty);
        }
        if let Some(frequency_penalty) = options.frequency_penalty {
            builder.frequency_penalty(frequency_penalty);
        }

        let request = builder.build()?;
        let response = self.client.chat().create(request).await?;

        let choice = response.choices.first().ok_or("No response choices")?;
        let usage = response.usage.ok_or("No usage data")?;

        Ok(ChatCompletion {
            content: choice.message.content.clone().unwrap_or_default(),
            model: response.model,
            prompt_tokens: usage.prompt_tokens as i32,
            completion_tokens: usage.completion_tokens as i32,
            total_tokens: usage.total_tokens as i32,
            finish_reason: format!("{:?}", choice.finish_reason),
        })
    }

    async fn chat_completion_stream(
        &self,
        messages: &[AIMessage],
        options: &ChatOptions,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<StreamChunk, Box<dyn std::error::Error + Send + Sync>>> + Send>>, Box<dyn std::error::Error + Send + Sync>> {
        let openai_messages = Self::convert_messages(messages);

        let mut builder = CreateChatCompletionRequestArgs::default();
        builder.model(options.model.clone())
            .messages(openai_messages);

        if let Some(temp) = options.temperature {
            builder.temperature(temp);
        }
        if let Some(max_tokens) = options.max_tokens {
            builder.max_tokens(max_tokens as u16);
        }
        if let Some(top_p) = options.top_p {
            builder.top_p(top_p);
        }
        if let Some(presence_penalty) = options.presence_penalty {
            builder.presence_penalty(presence_penalty);
        }
        if let Some(frequency_penalty) = options.frequency_penalty {
            builder.frequency_penalty(frequency_penalty);
        }

        let request = builder.build()?;
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

    fn count_tokens(&self, text: &str, _model: &str) -> i32 {
        use tiktoken_rs::cl100k_base;
        let bpe = cl100k_base().unwrap();
        let tokens = bpe.encode_with_special_tokens(text);
        tokens.len() as i32
    }

    fn calculate_cost(&self, prompt_tokens: i32, completion_tokens: i32, model: &str) -> f64 {
        let (input_price, output_price) = match model {
            "gpt-4" | "gpt-4-0314" => (0.03, 0.06),
            "gpt-4-32k" | "gpt-4-32k-0314" => (0.06, 0.12),
            "gpt-3.5-turbo" | "gpt-3.5-turbo-0301" => (0.0015, 0.002),
            "gpt-3.5-turbo-16k" => (0.003, 0.004),
            _ => (0.0015, 0.002),
        };

        (prompt_tokens as f64 / 1000.0) * input_price
            + (completion_tokens as f64 / 1000.0) * output_price
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{AIMessage, MessageRole, ChatOptions};

    // === convert_messages 测试 ===

    #[test]
    fn test_convert_messages_single_user() {
        let messages = vec![AIMessage {
            role: MessageRole::User,
            content: "Hello!".to_string(),
        }];

        let result = OpenAIProvider::convert_messages(&messages);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, Role::User);
        assert_eq!(result[0].content, Some("Hello!".to_string()));
    }

    #[test]
    fn test_convert_messages_all_roles() {
        let messages = vec![
            AIMessage { role: MessageRole::System, content: "You are a helpful assistant".to_string() },
            AIMessage { role: MessageRole::User, content: "What is Rust?".to_string() },
            AIMessage { role: MessageRole::Assistant, content: "Rust is a programming language".to_string() },
        ];

        let result = OpenAIProvider::convert_messages(&messages);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].role, Role::System);
        assert_eq!(result[1].role, Role::User);
        assert_eq!(result[2].role, Role::Assistant);
    }

    #[test]
    fn test_convert_messages_empty() {
        let messages: Vec<AIMessage> = vec![];
        let result = OpenAIProvider::convert_messages(&messages);
        assert!(result.is_empty());
    }

    #[test]
    fn test_convert_messages_preserves_content() {
        let messages = vec![
            AIMessage { role: MessageRole::User, content: "你好世界".to_string() },
            AIMessage { role: MessageRole::Assistant, content: "👋 你好！".to_string() },
        ];

        let result = OpenAIProvider::convert_messages(&messages);
        assert_eq!(result[0].content, Some("你好世界".to_string()));
        assert_eq!(result[1].content, Some("👋 你好！".to_string()));
    }

    // === calculate_cost 测试 ===

    #[test]
    fn test_calculate_cost_gpt4() {
        let provider = OpenAIProvider::new("test-key".to_string(), None);
        let cost = provider.calculate_cost(1000, 500, "gpt-4");
        // input: 1000/1000 * 0.03 = 0.03
        // output: 500/1000 * 0.06 = 0.03
        // total: 0.06
        assert!((cost - 0.06).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_cost_gpt35_turbo() {
        let provider = OpenAIProvider::new("test-key".to_string(), None);
        let cost = provider.calculate_cost(1000, 1000, "gpt-3.5-turbo");
        // input: 1000/1000 * 0.0015 = 0.0015
        // output: 1000/1000 * 0.002 = 0.002
        // total: 0.0035
        assert!((cost - 0.0035).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_cost_unknown_model() {
        let provider = OpenAIProvider::new("test-key".to_string(), None);
        let cost = provider.calculate_cost(1000, 1000, "unknown-model");
        // 应使用默认价格
        assert!((cost - 0.0035).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_cost_zero_tokens() {
        let provider = OpenAIProvider::new("test-key".to_string(), None);
        let cost = provider.calculate_cost(0, 0, "gpt-4");
        assert!((cost - 0.0).abs() < f64::EPSILON);
    }

    // === count_tokens 测试 ===

    #[test]
    fn test_count_tokens_english() {
        let provider = OpenAIProvider::new("test-key".to_string(), None);
        let tokens = provider.count_tokens("Hello, world!", "gpt-4");
        assert!(tokens > 0);
        assert!(tokens < 10); // 短文本应该少于10个token
    }

    #[test]
    fn test_count_tokens_chinese() {
        let provider = OpenAIProvider::new("test-key".to_string(), None);
        let tokens = provider.count_tokens("你好世界", "gpt-4");
        assert!(tokens > 0);
    }

    #[test]
    fn test_count_tokens_empty() {
        let provider = OpenAIProvider::new("test-key".to_string(), None);
        let tokens = provider.count_tokens("", "gpt-4");
        assert_eq!(tokens, 0);
    }

    #[test]
    fn test_count_tokens_longer_text() {
        let provider = OpenAIProvider::new("test-key".to_string(), None);
        let short = provider.count_tokens("Hello", "gpt-4");
        let long = provider.count_tokens("Hello, this is a longer text for testing token counting", "gpt-4");
        assert!(long > short);
    }

    // === name 测试 ===

    #[test]
    fn test_provider_name() {
        let provider = OpenAIProvider::new("test-key".to_string(), None);
        assert_eq!(provider.name(), "OpenAI");
    }

    // === new 测试 ===

    #[test]
    fn test_new_with_custom_base_url() {
        let provider = OpenAIProvider::new(
            "test-key".to_string(),
            Some("https://custom-api.example.com".to_string()),
        );
        assert_eq!(provider.name(), "OpenAI");
    }

    #[test]
    fn test_new_with_default_base_url() {
        let provider = OpenAIProvider::new("test-key".to_string(), None);
        assert_eq!(provider.name(), "OpenAI");
    }
}
