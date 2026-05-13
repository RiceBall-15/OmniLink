use std::sync::Arc;
use std::collections::HashMap;
use std::pin::Pin;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;
use anyhow::{Result, anyhow};

use super::providers::{AIProvider, AIMessage, MessageRole, ChatOptions, OpenAIProvider, AnthropicProvider, GoogleProvider, QwenProvider, ZhipuProvider, ErnieProvider};
use super::models::{
    ChatRequest, ChatResponse,
    ModelsResponse, ModelConfig,
    CreateAssistantRequest, UpdateAssistantRequest, AssistantInfo, AssistantsListResponse,
    TokenUsageResponse, ModelUsage, MessageHistory, ConversationHistoryResponse, CreateAssistantResponse,
};
use super::repository::{AssistantRepository, TokenUsageRepository, ConversationMessageRepository};
use super::api_key_store::{ApiKeyStore, ApiKeyStatus};

/// AI服务
pub struct AIService {
    assistant_repository: Arc<AssistantRepository>,
    token_usage_repository: Arc<TokenUsageRepository>,
    conversation_message_repository: Arc<ConversationMessageRepository>,
    providers: Arc<RwLock<HashMap<String, Arc<dyn AIProvider>>>>,
    model_configs: Arc<RwLock<Vec<ModelConfig>>>,
    api_key_store: Arc<ApiKeyStore>,
}

impl AIService {
    /// 创建AI服务
    pub fn new(
        assistant_repository: Arc<AssistantRepository>,
        token_usage_repository: Arc<TokenUsageRepository>,
        conversation_message_repository: Arc<ConversationMessageRepository>,
    ) -> Self {
        let providers = Arc::new(RwLock::new(HashMap::new()));
        let model_configs = Arc::new(RwLock::new(Self::default_models()));
        let api_key_store = Arc::new(ApiKeyStore::new());

        Self {
            assistant_repository,
            token_usage_repository,
            conversation_message_repository,
            providers,
            model_configs,
            api_key_store,
        }
    }

    /// 获取 API 密钥存储引用（用于初始化时加载密钥）
    pub fn api_key_store(&self) -> &Arc<ApiKeyStore> {
        &self.api_key_store
    }

    /// 初始化提供商
    pub async fn init_providers(&self, api_keys: HashMap<String, String>) -> Result<()> {
        let mut providers = self.providers.write().await;

        // OpenAI
        if let Some(openai_key) = api_keys.get("openai") {
            providers.insert(
                "openai".to_string(),
                Arc::new(OpenAIProvider::new(openai_key.clone(), None)),
            );
        }

        // Anthropic
        if let Some(anthropic_key) = api_keys.get("anthropic") {
            providers.insert(
                "anthropic".to_string(),
                Arc::new(AnthropicProvider::new(anthropic_key.clone(), None)),
            );
        }

        // Google
        if let Some(google_key) = api_keys.get("google") {
            providers.insert(
                "google".to_string(),
                Arc::new(GoogleProvider::new(google_key.clone(), None)),
            );
        }

        // 通义千问 (Qwen)
        if let Some(qwen_key) = api_keys.get("qwen") {
            providers.insert(
                "qwen".to_string(),
                Arc::new(QwenProvider::new(qwen_key.clone(), None)),
            );
        }

        // 智谱AI (ZhipuAI)
        if let Some(zhipu_key) = api_keys.get("zhipu") {
            providers.insert(
                "zhipu".to_string(),
                Arc::new(ZhipuProvider::new(zhipu_key.clone(), None)),
            );
        }

        // 文心一言 (Ernie)
        if let Some(ernie_key) = api_keys.get("ernie") {
            if let Some(ernie_secret) = api_keys.get("ernie_secret") {
                providers.insert(
                    "ernie".to_string(),
                    Arc::new(ErnieProvider::new(ernie_key.clone(), ernie_secret.clone(), None)),
                );
            }
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
            // 通义千问模型
            ModelConfig {
                id: "qwen-turbo".to_string(),
                name: "通义千问-Turbo".to_string(),
                provider: "qwen".to_string(),
                api_base: "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string(),
                max_tokens: 8192,
                input_price_per_1k: 0.002,
                output_price_per_1k: 0.006,
            },
            ModelConfig {
                id: "qwen-plus".to_string(),
                name: "通义千问-Plus".to_string(),
                provider: "qwen".to_string(),
                api_base: "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string(),
                max_tokens: 131072,
                input_price_per_1k: 0.004,
                output_price_per_1k: 0.012,
            },
            ModelConfig {
                id: "qwen-max".to_string(),
                name: "通义千问-Max".to_string(),
                provider: "qwen".to_string(),
                api_base: "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string(),
                max_tokens: 32768,
                input_price_per_1k: 0.02,
                output_price_per_1k: 0.06,
            },
            // 智谱AI模型
            ModelConfig {
                id: "glm-4".to_string(),
                name: "GLM-4".to_string(),
                provider: "zhipu".to_string(),
                api_base: "https://open.bigmodel.cn/api/paas/v4".to_string(),
                max_tokens: 8192,
                input_price_per_1k: 0.1,
                output_price_per_1k: 0.1,
            },
            ModelConfig {
                id: "glm-4-flash".to_string(),
                name: "GLM-4-Flash".to_string(),
                provider: "zhipu".to_string(),
                api_base: "https://open.bigmodel.cn/api/paas/v4".to_string(),
                max_tokens: 8192,
                input_price_per_1k: 0.0001,
                output_price_per_1k: 0.0001,
            },
            ModelConfig {
                id: "glm-4-air".to_string(),
                name: "GLM-4-Air".to_string(),
                provider: "zhipu".to_string(),
                api_base: "https://open.bigmodel.cn/api/paas/v4".to_string(),
                max_tokens: 8192,
                input_price_per_1k: 0.001,
                output_price_per_1k: 0.001,
            },
            // 文心一言模型
            ModelConfig {
                id: "ernie-3.5-8k".to_string(),
                name: "文心一言-3.5".to_string(),
                provider: "ernie".to_string(),
                api_base: "https://aip.baidubce.com".to_string(),
                max_tokens: 8192,
                input_price_per_1k: 0.008,
                output_price_per_1k: 0.008,
            },
            ModelConfig {
                id: "ernie-4.0-8k".to_string(),
                name: "文心一言-4.0".to_string(),
                provider: "ernie".to_string(),
                api_base: "https://aip.baidubce.com".to_string(),
                max_tokens: 8192,
                input_price_per_1k: 0.12,
                output_price_per_1k: 0.12,
            },
            ModelConfig {
                id: "ernie-speed-8k".to_string(),
                name: "文心一言-Speed".to_string(),
                provider: "ernie".to_string(),
                api_base: "https://aip.baidubce.com".to_string(),
                max_tokens: 8192,
                input_price_per_1k: 0.001,
                output_price_per_1k: 0.001,
            },
        ]
    }

    /// 获取提供商
    async fn get_provider(&self, provider_name: &str) -> Result<Arc<dyn AIProvider>> {
        let providers = self.providers.read().await;
        providers
            .get(provider_name)
            .cloned()
            .ok_or_else(|| anyhow!("Provider {} not found", provider_name))
    }

    /// 发送AI对话请求
    pub async fn chat(&self, request: ChatRequest, user_id: Uuid) -> Result<ChatResponse> {
        // 获取助手信息
        let assistant = self
            .assistant_repository
            .find_by_id(request.assistant_id)
            .await?
            .ok_or_else(|| anyhow!("Assistant not found"))?;

        // 构建消息列表
        let mut messages = vec![];

        // 添加系统提示
        if let Some(ref system_prompt) = assistant.system_prompt {
            messages.push(AIMessage {
                role: MessageRole::System,
                content: system_prompt.clone(),
            });
        }

        // 加载对话历史（最近20条消息作为上下文）
        let history = self
            .conversation_message_repository
            .get_conversation_history(request.conversation_id, 20)
            .await
            .unwrap_or_default();

        for msg in &history {
            let role = if msg.sender_type == "user" {
                MessageRole::User
            } else if msg.sender_type == "assistant" {
                MessageRole::Assistant
            } else {
                continue; // skip system messages from history
            };
            let content = msg.content
                .as_str()
                .unwrap_or("")
                .to_string();
            if !content.is_empty() {
                messages.push(AIMessage { role, content });
            }
        }

        // 保存用户消息到数据库
        self.conversation_message_repository
            .save_message(
                request.conversation_id,
                user_id,
                "user",
                serde_json::json!(request.message),
            )
            .await?;

        // 添加当前用户消息
        messages.push(AIMessage {
            role: MessageRole::User,
            content: request.message.clone(),
        });

        // 获取模型配置（支持请求级别的模型覆盖）
        let effective_model_id = request.model_id.as_deref().unwrap_or(&assistant.model_id);
        let model_configs = self.model_configs.read().await;
        let model_config = model_configs
            .iter()
            .find(|m| m.id == effective_model_id)
            .ok_or_else(|| anyhow!("Model config not found: {}", effective_model_id))?;

        // 获取提供商
        let provider = self.get_provider(&model_config.provider).await?;

        // 构建对话选项
        let options = ChatOptions {
            model: effective_model_id.to_string(),
            temperature: request.temperature.or(assistant.temperature).or(Some(0.7)),
            max_tokens: request.max_tokens.or(assistant.max_tokens).or(Some(2048)),
            top_p: Some(1.0),
            presence_penalty: Some(0.0),
            frequency_penalty: Some(0.0),
        };

        // 调用AI API（带重试和指数退避）
        let max_retries = 3u32;
        let mut completion = None;
        for attempt in 0..=max_retries {
            match provider.chat_completion(&messages, &options).await {
                Ok(result) => {
                    completion = Some(result);
                    break;
                }
                Err(e) => {
                    tracing::warn!(
                        "chat_completion failed (attempt {}/{}): {}",
                        attempt + 1,
                        max_retries + 1,
                        e
                    );
                    if attempt < max_retries {
                        let delay_ms = 1000 * (1 << attempt);
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }
        let completion = completion.ok_or_else(|| anyhow!("AI API failed after max retries"))?;

        // 保存助手回复到数据库
        let saved_message = self.conversation_message_repository
            .save_message(
                request.conversation_id,
                assistant.id,
                "assistant",
                serde_json::json!(completion.content),
            )
            .await?;

        // 记录Token使用
        let cost = provider.calculate_cost(
            completion.prompt_tokens,
            completion.completion_tokens,
            &completion.model,
        );

        self.token_usage_repository
            .upsert(
                user_id,
                Some(request.conversation_id),
                completion.model.clone(),
                1,
                completion.prompt_tokens,
                completion.completion_tokens,
                cost,
            )
            .await?;

        Ok(ChatResponse {
            conversation_id: request.conversation_id,
            assistant_id: request.assistant_id,
            message_id: saved_message.id,
            content: completion.content,
            model: completion.model,
            prompt_tokens: completion.prompt_tokens,
            completion_tokens: completion.completion_tokens,
            total_tokens: completion.total_tokens,
            estimated_cost: cost,
            created_at: Utc::now().timestamp(),
        })
    }

    /// 发送流式AI对话请求
    pub async fn chat_stream(
        &self,
        request: ChatRequest,
        _user_id: Uuid,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = std::result::Result<crate::providers::StreamChunk, Box<dyn std::error::Error + Send + Sync>>> + Send>>> {
        // 获取助手信息
        let assistant = self
            .assistant_repository
            .find_by_id(request.assistant_id)
            .await?
            .ok_or_else(|| anyhow!("Assistant not found"))?;

        // 构建消息列表
        let mut messages = vec![];

        // 添加系统提示
        if let Some(system_prompt) = assistant.system_prompt {
            messages.push(AIMessage {
                role: MessageRole::System,
                content: system_prompt,
            });
        }

        // 加载对话历史（最近20条消息作为上下文）
        let history = self
            .conversation_message_repository
            .get_conversation_history(request.conversation_id, 20)
            .await
            .unwrap_or_default();

        for msg in &history {
            let role = if msg.sender_type == "user" {
                MessageRole::User
            } else if msg.sender_type == "assistant" {
                MessageRole::Assistant
            } else {
                continue;
            };
            let content = msg.content
                .as_str()
                .unwrap_or("")
                .to_string();
            if !content.is_empty() {
                messages.push(AIMessage { role, content });
            }
        }

        // 保存用户消息到数据库
        let _ = self.conversation_message_repository
            .save_message(
                request.conversation_id,
                _user_id,
                "user",
                serde_json::json!(request.message),
            )
            .await;

        // 添加用户消息
        messages.push(AIMessage {
            role: MessageRole::User,
            content: request.message.clone(),
        });

        // 获取模型配置（支持请求级别的模型覆盖）
        let effective_model_id = request.model_id.as_deref().unwrap_or(&assistant.model_id);
        let model_configs = self.model_configs.read().await;
        let model_config = model_configs
            .iter()
            .find(|m| m.id == effective_model_id)
            .ok_or_else(|| anyhow!("Model config not found: {}", effective_model_id))?;

        // 获取提供商
        let provider = self.get_provider(&model_config.provider).await?;

        // 构建对话选项
        let options = ChatOptions {
            model: effective_model_id.to_string(),
            temperature: request.temperature.or(assistant.temperature).or(Some(0.7)),
            max_tokens: request.max_tokens.or(assistant.max_tokens).or(Some(2048)),
            top_p: Some(1.0),
            presence_penalty: Some(0.0),
            frequency_penalty: Some(0.0),
        };

        // 调用AI提供商的流式API
        let stream = provider
            .chat_completion_stream(&messages, &options)
            .await
            .map_err(|e| anyhow!("{}", e))?;

        Ok(stream)
    }

    /// 创建AI助手
    pub async fn create_assistant(
        &self,
        request: CreateAssistantRequest,
        user_id: Uuid,
    ) -> Result<CreateAssistantResponse> {
        let assistant_id = Uuid::new_v4();
        let _now = Utc::now();

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

    /// Get assistant by ID
    pub async fn get_assistant(&self, assistant_id: Uuid) -> Result<Option<AssistantInfo>> {
        let assistant = self
            .assistant_repository
            .find_by_id(assistant_id)
            .await?;

        Ok(assistant.map(|a| AssistantInfo {
            id: a.id,
            name: a.name,
            description: a.description,
            model_id: a.model_id,
            system_prompt: a.system_prompt,
            temperature: a.temperature,
            max_tokens: a.max_tokens,
            created_at: a.created_at.timestamp(),
        }))
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

    /// 获取对话历史
    pub async fn get_conversation_history(
        &self,
        conversation_id: Uuid,
        page: i64,
        page_size: i64,
    ) -> Result<ConversationHistoryResponse> {
        let _offset = (page - 1).max(0) * page_size;
        let total = self
            .conversation_message_repository
            .count_messages(conversation_id)
            .await?;

        let messages = self
            .conversation_message_repository
            .get_conversation_history(conversation_id, page_size)
            .await
            .unwrap_or_default();

        let message_history: Vec<MessageHistory> = messages
            .into_iter()
            .map(|msg| MessageHistory {
                role: msg.sender_type.clone(),
                content: msg.content.as_str().unwrap_or("").to_string(),
                created_at: msg.created_at.timestamp(),
            })
            .collect();

        Ok(ConversationHistoryResponse {
            conversation_id,
            assistant_id: Uuid::nil(), // Will be filled from context
            messages: message_history,
            total_messages: total as i32,
        })
    }

    /// 清空对话历史
    pub async fn clear_conversation(&self, conversation_id: Uuid) -> Result<()> {
        self.conversation_message_repository
            .clear_conversation(conversation_id)
            .await?;
        Ok(())
    }

    // ===== API 密钥管理 =====

    /// 列出所有 API 密钥状态（脱敏）
    pub async fn list_api_keys(&self) -> Vec<ApiKeyStatus> {
        self.api_key_store.list_keys().await
    }

    /// 轮换 API 密钥
    ///
    /// # 返回
    /// true 如果之前存在密钥，false 如果是首次设置
    pub async fn rotate_api_key(&self, provider: &str, new_key: String) -> Result<bool> {
        let old = self.api_key_store.rotate_key(provider, new_key).await;
        Ok(old.is_some())
    }

    /// 回滚 API 密钥到上一个版本
    ///
    /// # 返回
    /// true 如果回滚成功，false 如果没有历史密钥
    pub async fn rollback_api_key(&self, provider: &str) -> Result<bool> {
        let success = self.api_key_store.rollback_key(provider).await;
        Ok(success)
    }

    /// 切换 API 密钥的启用/禁用状态
    ///
    /// # 返回
    /// 新的状态（true=启用, false=禁用）
    pub async fn toggle_api_key(&self, provider: &str) -> Result<bool> {
        // 先检查当前状态
        let keys = self.api_key_store.list_keys().await;
        let current = keys.iter().find(|k| k.provider == provider);

        match current {
            Some(entry) => {
                if entry.active {
                    self.api_key_store.disable_key(provider).await;
                    Ok(false)
                } else {
                    self.api_key_store.enable_key(provider).await;
                    Ok(true)
                }
            }
            None => Err(common::AppError::NotFound(format!("Provider not found: {}", provider))),
        }
    }

    /// 重新初始化 providers（密钥轮换后调用）
    ///
    /// 使用 ApiKeyStore 中的当前密钥重新初始化所有 providers。
    pub async fn reinit_providers(&self) -> Result<()> {
        let keys = self.api_key_store.get_all_keys().await;
        if !keys.is_empty() {
            self.init_providers(keys).await?;
        }
        Ok(())
    }
}