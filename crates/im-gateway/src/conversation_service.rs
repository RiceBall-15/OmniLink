use crate::models::{
    CreateConversationRequest, CreateConversationResponse,
    ConversationsListResponse, ConversationInfo, ParticipantInfo,
};
use crate::repository::ConversationRepository;
use crate::user_repository::UserRepository;
use common::models::User;
use common::{AppError, Result};
use uuid::Uuid;
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

/// 对话服务
pub struct ConversationService {
    conversation_repository: Arc<ConversationRepository>,
    user_repository: Arc<UserRepository>,
    user_cache: Arc<RwLock<HashMap<Uuid, User>>>,
}

impl ConversationService {
    pub fn new(
        conversation_repository: Arc<ConversationRepository>,
        user_repository: Arc<UserRepository>,
    ) -> Self {
        Self {
            conversation_repository,
            user_repository,
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
            name: conversation.name.unwrap_or_default(),
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

        let total = conversations.len() as i32;
        let mut conversation_infos = Vec::new();

        for conversation in conversations {
            let participants = self
                .conversation_repository
                .get_participants(conversation.id)
                .await?;

            conversation_infos.push(ConversationInfo {
                conversation_id: conversation.id,
                name: conversation.name.unwrap_or_default(),
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
            total,
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
            name: conversation.name.unwrap_or_default(),
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

        // 从数据库加载
        match self.user_repository.find_by_id(user_id).await? {
            Some(user) => {
                // 缓存用户信息
                let mut cache = self.user_cache.write().await;
                cache.insert(user.id, user.clone());
                Ok(user)
            }
            None => {
                // 用户不存在，返回默认信息
                tracing::warn!("User {} not found in database, using default info", user_id);
                Ok(User {
                    id: user_id,
                    username: format!("user_{}", user_id),
                    email: format!("user_{}@example.com", user_id),
                    password_hash: String::new(),
                    avatar_url: None,
                    bio: None,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    last_login_at: None,
                })
            }
        }
    }

    /// 添加成员到对话
    pub async fn add_member(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
        operator_id: Uuid,
    ) -> Result<ParticipantInfo> {
        // 验证操作者是否是对话成员
        let participants = self.conversation_repository
            .get_participants(conversation_id)
            .await?;

        let operator = participants.iter().find(|p| p.user_id == operator_id);
        match operator {
            Some(p) if p.role == "owner" || p.role == "admin" => {}
            Some(_) => return Err(AppError::Authorization("Only owner or admin can add members".to_string())),
            None => return Err(AppError::Authorization("Not a conversation member".to_string())),
        }

        // 检查用户是否已在对话中
        if participants.iter().any(|p| p.user_id == user_id) {
            return Err(AppError::Validation("User already in conversation".to_string()));
        }

        // 添加成员
        self.conversation_repository
            .add_participant(conversation_id, user_id, "member".to_string())
            .await?;

        let user = self.get_user_info(user_id).await?;
        Ok(ParticipantInfo {
            user_id,
            username: user.username,
            avatar_url: user.avatar_url,
            role: "member".to_string(),
            joined_at: Utc::now().timestamp(),
        })
    }

    /// 从对话移除成员
    pub async fn remove_member(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
        operator_id: Uuid,
    ) -> Result<()> {
        let participants = self.conversation_repository
            .get_participants(conversation_id)
            .await?;

        // 验证操作者权限
        let operator = participants.iter().find(|p| p.user_id == operator_id);
        match operator {
            Some(p) if p.role == "owner" || p.role == "admin" => {}
            Some(_) if operator_id == user_id => {
                // 用户可以自己退出
            }
            Some(_) => return Err(AppError::Authorization("Only owner or admin can remove members".to_string())),
            None => return Err(AppError::Authorization("Not a conversation member".to_string())),
        }

        // 不能移除创建者
        if let Some(target) = participants.iter().find(|p| p.user_id == user_id) {
            if target.role == "owner" && operator_id != user_id {
                return Err(AppError::Authorization("Cannot remove conversation owner".to_string()));
            }
        }

        // 移除成员
        self.conversation_repository
            .remove_participant(conversation_id, user_id)
            .await?;

        Ok(())
    }

    /// 获取对话成员列表
    pub async fn get_members(&self, conversation_id: Uuid) -> Result<Vec<ParticipantInfo>> {
        let participants = self.conversation_repository
            .get_participants(conversation_id)
            .await?;

        let mut result = Vec::new();
        for p in participants {
            let user = self.get_user_info(p.user_id).await?;
            result.push(ParticipantInfo {
                user_id: p.user_id,
                username: user.username,
                avatar_url: user.avatar_url,
                role: p.role,
                joined_at: p.joined_at,
            });
        }

        Ok(result)
    }
}
