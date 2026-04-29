use super::super::models::{
    CreateConversationRequest, CreateConversationResponse,
    ConversationsListResponse, ConversationInfo, ParticipantInfo,
};
use super::super::repository::ConversationRepository;
use common::models::{Conversation, Participant, User};
use common::{AppError, Result};
use uuid::Uuid;
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

/// 对话服务
pub struct ConversationService {
    conversation_repository: Arc<ConversationRepository>,
    user_cache: Arc<RwLock<HashMap<Uuid, User>>>,
}

impl ConversationService {
    pub fn new(conversation_repository: Arc<ConversationRepository>) -> Self {
        Self {
            conversation_repository,
            user_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建对话
    pub async fn create_conversation(
        &self,
        request: CreateConversationRequest,
        created_by: Uuid,
    ) -> Result<CreateConversationResponse> {
        let conversation_id = Uuid::new_v4();

        // 创建对话
        let conversation = self
            .conversation_repository
            .create(
                conversation_id,
                request.name.clone(),
                request.description,
                request.is_group,
                created_by,
            )
            .await?;

        // 添加创建者
        self.conversation_repository
            .add_participant(conversation_id, created_by, "owner".to_string())
            .await?;

        // 添加其他参与者
        let mut participants = Vec::new();

        let creator_user = self.get_user_info(created_by).await?;
        participants.push(ParticipantInfo {
            user_id: created_by,
            username: creator_user.username,
            avatar_url: creator_user.avatar_url,
            role: "owner".to_string(),
            joined_at: Utc::now().timestamp(),
        });

        for participant_id in request.participant_ids {
            if participant_id != created_by {
                self.conversation_repository
                    .add_participant(conversation_id, participant_id, "member".to_string())
                    .await?;

                let user = self.get_user_info(participant_id).await?;
                participants.push(ParticipantInfo {
                    user_id: participant_id,
                    username: user.username,
                    avatar_url: user.avatar_url,
                    role: "member".to_string(),
                    joined_at: Utc::now().timestamp(),
                });
            }
        }

        Ok(CreateConversationResponse {
            conversation_id,
            name: conversation.name,
            description: conversation.description,
            is_group: conversation.is_group,
            participants,
            created_at: conversation.created_at.timestamp(),
        })
    }

    /// 获取对话列表
    pub async fn list_conversations(&self, user_id: Uuid) -> Result<ConversationsListResponse> {
        let conversations = self
            .conversation_repository
            .get_user_conversations(user_id)
            .await?;

        let mut conversation_infos = Vec::new();

        for conversation in conversations {
            let participants = self
                .conversation_repository
                .get_participants(conversation.id)
                .await?;

            conversation_infos.push(ConversationInfo {
                conversation_id: conversation.id,
                name: conversation.name,
                description: conversation.description,
                is_group: conversation.is_group,
                avatar_url: conversation.avatar_url,
                created_at: conversation.created_at.timestamp(),
                updated_at: conversation.updated_at.timestamp(),
                last_message_at: conversation.last_message_at.map(|t| t.timestamp()),
                participant_count: participants.len() as i32,
            });
        }

        Ok(ConversationsListResponse {
            conversations: conversation_infos,
            total: conversations.len() as i32,
        })
    }

    /// 获取对话详情
    pub async fn get_conversation(&self, conversation_id: Uuid) -> Result<ConversationInfo> {
        let conversation = self
            .conversation_repository
            .get_by_id(conversation_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Conversation not found".to_string()))?;

        let participants = self
            .conversation_repository
            .get_participants(conversation_id)
            .await?;

        Ok(ConversationInfo {
            conversation_id: conversation.id,
            name: conversation.name,
            description: conversation.description,
            is_group: conversation.is_group,
            avatar_url: conversation.avatar_url,
            created_at: conversation.created_at.timestamp(),
            updated_at: conversation.updated_at.timestamp(),
            last_message_at: conversation.last_message_at.map(|t| t.timestamp()),
            participant_count: participants.len() as i32,
        })
    }

    /// 获取用户信息（带缓存）
    async fn get_user_info(&self, user_id: Uuid) -> Result<User> {
        // 先从缓存中查找
        {
            let cache = self.user_cache.read().await;
            if let Some(user) = cache.get(&user_id) {
                return Ok(user.clone());
            }
        }

        // TODO: 从数据库加载
        // 暂时返回假数据
        Ok(User {
            id: user_id,
            username: format!("user_{}", user_id),
            email: format!("user_{}@example.com", user_id),
            password_hash: String::new(),
            avatar_url: None,
            status: Some("active".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }
}