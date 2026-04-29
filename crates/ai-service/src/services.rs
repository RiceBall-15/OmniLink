use super::super::providers::{AIProvider, AIMessage, MessageRole, ChatOptions, OpenAIProvider, AnthropicProvider, GoogleProvider};
use super::super::models::{
    ChatRequest, ChatResponse, ChatStreamResponse,
    CreateAssistantRequest, CreateAssistantResponse, AssistantsListResponse,
    UpdateAssistantRequest, AssistantInfo,
    ConversationHistoryResponse, MessageHistory,
    TokenUsageResponse, ModelUsage, ModelsResponse, ModelConfig
};
use super::super::repository::{AssistantRepository, TokenUsageRepository, TokenUsageSummary};
use common::{AppError, Result, Claims};
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// AI服务
pub struct AIService {
    assistant_repository: Arc<AssistantRepository>,
    token_usage_repository: Arc<TokenUsageRepository>,
    providers: Arc<RwLock<HashMap<String, Box<dyn AIProvider>>>>,
    model_configs: Arc<RwLock<Vec<ModelConfig>>>,
}

impl AIService {
    /// 创建AI服务
    pub fn new(
        assistant_repository: Arc<AssistantRepository>,
        token_usage_repository: Arc<TokenUsageRepository>,
    ) -> Self {
        let providers = Arc::new(RwLock::new(HashMap::new()));
        let model_configs = Arc::new(RwLock::new(Self::default_models()));

        Self {
            assistant_repository,
            token_usage_repository,
            providers,
            model_configs,
        }
    }

    /// 初始化提供商
    pub async fn init_providers(&self, api_keys: HashMap<String, String>) -> Result<()> {
        let mut providers = self.providers.write().await;

        // OpenAI
        if let Some(openai_key) = api_keys.get("openai") {
            providers.insert(
                "openai".to_string(),
                Box::new(OpenAIProvider::new(openai_key.clone(), None)),
            );
        }

        // Anthropic
        if let Some(anthropic_key) = api_keys.get("anthropic") {
            providers.insert(
                "anthropic".to_string(),
                Box::new(AnthropicProvider::new(anthropic_key.clone(), None)),
            );
        }

        // Google
        if let Some(google_key) = api_keys.get("google") {
            providers.insert(
                "google".to_string(),
                Box::new(GoogleProvider::new(google_key.clone(), None)),
            );
        }

        Ok(())
    }

    /// 获取默认模型配置
    fn default_models() -> Vec<ModelConfig> {
        vec![
            ModelConfig {
                id: "gpt-3.5-turbo".to_string(),
                name: "GPT-3.5 Turbo".to_string(),
                provider: "openai".to_string(),
                api_base: "https://api.openai.com/v1".to_string(),
                max_tokens: 4096,
                input_price_per_1k: 0.0015,
                output_price_per_1k: 0.002,
            },
            ModelConfig {
                id: "gpt-4".to_string(),
                name: "GPT-4".to_string(),
                provider: "openai".to_string(),
                api_base: "https://api.openai.com/v1".to_string(),
                max_tokens: 8192,
                input_price_per_1k: 0.03,
                output_price_per_1k: 0.06,
            },
            ModelConfig {
                id: "claude-3-sonnet".to_string(),
                name: "Claude 3 Sonnet".to_string(),
                provider: "anthropic".to_string(),
                api_base: "https://api.anthropic.com/v1".to_string(),
                max_tokens: 4096,
                input_price_per_1k: 0.003,
                output_price_per_1k: 0.015,
            },
            ModelConfig {
                id: "gemini-pro".to_string(),
                name: "Gemini Pro".to_string(),
                provider: "google".to_string(),
                api_base: "https://generativelanguage.googleapis.com/v1beta".to_string(),
                max_tokens: 2048,
                input_price_per_1k: 0.0005,
                output_price_per_1k: 0.0015,
            },
        ]
    }

    /// 获取提供商
    async fn get_provider(&self, provider_name: &str) -> Result<Arc<Box<dyn AIProvider>>> {
        let providers = self.providers.read().await;
        providers
            .get(provider_name)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("Provider {} not found", provider_name)))
    }

    /// 发送AI对话请求
    pub async fn chat(&self, request: ChatRequest, user_id: Uuid) -> Result<ChatResponse> {
        // 获取助手信息
        let assistant = self
            .assistant_repository
            .find_by_id(request.assistant_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Assistant not found".to_string()))?;

        // 获取对话历史（简化版，实际需要从数据库获取）
        let mut messages = vec![];

        // 添加系统提示
        if let Some(system_prompt) = assistant.system_prompt {
            messages.push(AIMessage {
                role: MessageRole::System,
                content: system_prompt,
            });
        }

        // 添加用户消息
        messages.push(AIMessage {
            role: MessageRole::User,
            content: request.message.clone(),
        });

        // 获取模型配置
        let model_configs = self.model_configs.read().await;
        let model_config = model_configs
            .iter()
            .find(|m| m.id == assistant.model_id)
            .ok_or_else(|| AppError::NotFound("Model config not found".to_string()))?;

        // 获取提供商
        let provider = self.get_provider(&model_config.provider).await?;

        // 构建对话选项
        let options = ChatOptions {
            model: assistant.model_id.clone(),
            temperature: request.temperature.or(assistant.temperature).or(Some(0.7)),
            max_tokens: request.max_tokens.or(assistant.max_tokens).or(Some(2048)),
            top_p: Some(1.0),
            presence_penalty: Some(0.0),
            frequency_penalty: Some(0.0),
        };

        // 调用AI API
        let completion = provider.chat_completion(&messages, &options).await?;

        // 保存消息到数据库（简化版）
        let message_id = Uuid::new_v4();

        // 记录Token使用
        self.token_usage_repository
            .upsert(
                user_id,
                Some(request.conversation_id),
                completion.model.clone(),
                1,
                completion.prompt_tokens,
                completion.completion_tokens,
                provider.calculate_cost(
                    completion.prompt_tokens,
                    completion.completion_tokens,
                    &completion.model,
                ),
            )
            .await?;

        Ok(ChatResponse {
            conversation_id: request.conversation_id,
            assistant_id: request.assistant_id,
            message_id,
            content: completion.content,
            model: completion.model,
            prompt_tokens: completion.prompt_tokens,
            completion_tokens: completion.completion_tokens,
            total_tokens: completion.total_tokens,
            estimated_cost: provider.calculate_cost(
                completion.prompt_tokens,
                completion.completion_tokens,
                &completion.model,
            ),
            created_at: Utc::now().timestamp(),
        })
    }

    /// 发送流式AI对话请求
    pub async fn chat_stream(&self, request: ChatRequest, user_id: Uuid) -> Result<ChatStreamResponse> {
        // 获取助手信息
        let assistant = self
            .assistant_repository
            .find_by_id(request.assistant_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Assistant not found".to_string()))?;

        // 获取对话历史
        let mut messages = vec![];

        // 添加系统提示
        if let Some(system_prompt) = assistant.system_prompt {
            messages.push(AIMessage {
                role: MessageRole::System,
                content: system_prompt,
            });
        }

        // 添加用户消息
        messages.push(AIMessage {
            role: MessageRole::User,
            content: request.message.clone(),
        });

        // 获取模型配置
        let model_configs = self.model_configs.read().await;
        let model_config = model_configs
            .iter()
            .find(|m| m.id == assistant.model_id)
            .ok_or_else(|| AppError::NotFound("Model config not found".to_string()))?;

        // 获取提供商
        let provider = self.get_provider(&model_config.provider).await?;

        // 构建对话选项
        let options = ChatOptions {
            model: assistant.model_id.clone(),
            temperature: request.temperature.or(assistant.temperature).or(Some(0.7)),
            max_tokens: request.max_tokens.or(assistant.max_tokens).or(Some(2048)),
            top_p: Some(1.0),
            presence_penalty: Some(0.0),
            frequency_penalty: Some(0.0),
        };

        let message_id = Uuid::new_v4();

        // 返回流式响应
        Ok(ChatStreamResponse {
            conversation_id: request.conversation_id,
            assistant_id: request.assistant_id,
            message_id,
            content: String::new(),
            delta: None,
            done: false,
            model: assistant.model_id,
        })
    }

    /// 创建AI助手
    pub async fn create_assistant(
        &self,
        request: CreateAssistantRequest,
        user_id: Uuid,
    ) -> Result<CreateAssistantResponse> {
        let assistant_id = Uuid::new_v4();
        let now = Utc::now();

        let assistant = self
            .assistant_repository
            .create(
                assistant_id,
                request.name.clone(),
                request.description,
                request.model_id.clone(),
                request.system_prompt,
                request.temperature,
                request.max_tokens,
                user_id,
            )
            .await?;

        Ok(CreateAssistantResponse {
            id: assistant.id,
            name: assistant.name,
            description: assistant.description,
            model_id: assistant.model_id,
            system_prompt: assistant.system_prompt,
            temperature: assistant.temperature,
            max_tokens: assistant.max_tokens,
            created_at: assistant.created_at.timestamp(),
        })
    }

    /// 获取用户的AI助手列表
    pub async fn list_assistants(&self, user_id: Uuid) -> Result<AssistantsListResponse> {
        let assistants = self
            .assistant_repository
            .find_by_user_id(user_id)
            .await?;

        let assistant_infos: Vec<AssistantInfo> = assistants
            .into_iter()
            .map(|a| AssistantInfo {
                id: a.id,
                name: a.name,
                description: a.description,
                model_id: a.model_id,
                system_prompt: a.system_prompt,
                temperature: a.temperature,
                max_tokens: a.max_tokens,
                created_at: a.created_at.timestamp(),
            })
            .collect();

        Ok(AssistantsListResponse {
            assistants: assistant_infos,
        })
    }

    /// 更新AI助手
    pub async fn update_assistant(
        &self,
        assistant_id: Uuid,
        request: UpdateAssistantRequest,
    ) -> Result<AssistantInfo> {
        let assistant = self
            .assistant_repository
            .update(
                assistant_id,
                request.name,
                request.description,
                request.model_id,
                request.system_prompt,
                request.temperature,
                request.max_tokens,
            )
            .await?;

        Ok(AssistantInfo {
            id: assistant.id,
            name: assistant.name,
            description: assistant.description,
            model_id: assistant.model_id,
            system_prompt: assistant.system_prompt,
            temperature: assistant.temperature,
            max_tokens: assistant.max_tokens,
            created_at: assistant.created_at.timestamp(),
        })
    }

    /// 删除AI助手
    pub async fn delete_assistant(&self, assistant_id: Uuid) -> Result<()> {
        self.assistant_repository
            .delete(assistant_id)
            .await?;

        Ok(())
    }

    /// 获取Token使用统计
    pub async fn get_token_usage(
        &self,
        user_id: Uuid,
        start_date: Option<String>,
        end_date: Option<String>,
    ) -> Result<TokenUsageResponse> {
        let total_summary = self
            .token_usage_repository
            .get_total_usage(user_id)
            .await?;

        let model_usages = self
            .token_usage_repository
            .get_user_usage(user_id, start_date, end_date)
            .await?;

        let model_configs = self.model_configs.read().await;

        let models: Vec<ModelUsage> = model_usages
            .into_iter()
            .map(|usage| {
                let model_config = model_configs
                    .iter()
                    .find(|m| m.id == usage.model_id);

                ModelUsage {
                    model_id: usage.model_id.clone(),
                    model_name: model_config
                        .map(|m| m.name.clone())
                        .unwrap_or_else(|| usage.model_id.clone()),
                    request_count: usage.request_count,
                    total_tokens: usage.total_tokens,
                    estimated_cost: usage.estimated_cost,
                }
            })
            .collect();

        Ok(TokenUsageResponse {
            total_tokens: total_summary.total_tokens,
            prompt_tokens: total_summary.prompt_tokens,
            completion_tokens: total_summary.completion_tokens,
            estimated_cost: total_summary.estimated_cost,
            request_count: total_summary.request_count,
            models,
        })
    }

    /// 获取支持的模型列表
    pub async fn list_models(&self) -> Result<ModelsResponse> {
        let model_configs = self.model_configs.read().await;

        Ok(ModelsResponse {
            models: model_configs.clone(),
        })
    }
}