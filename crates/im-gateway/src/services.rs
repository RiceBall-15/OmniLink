use crate::models::{
    SendMessageRequest, SendMessageResponse, MessageHistoryResponse, MessageInfo,
    OnlineUsersResponse, OnlineUserInfo, WSMessage, WSMessageType,
};
use crate::repository::MessageRepository;
use crate::user_repository::UserRepository;
use crate::connection_manager::WSConnectionManager;
use crate::status_manager::OnlineStatusManager;
use common::models::{Message, User};
use common::{AppError, Result};
use uuid::Uuid;
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

/// IM服务
pub struct IMService {
    message_repository: Arc<MessageRepository>,
    user_repository: Arc<UserRepository>,
    connection_manager: Arc<WSConnectionManager>,
    status_manager: Arc<OnlineStatusManager>,
    user_cache: Arc<RwLock<HashMap<Uuid, User>>>,
}

impl IMService {
    pub fn new(
        message_repository: Arc<MessageRepository>,
        user_repository: Arc<UserRepository>,
        connection_manager: Arc<WSConnectionManager>,
        status_manager: Arc<OnlineStatusManager>,
    ) -> Self {
        Self {
            message_repository,
            user_repository,
            connection_manager,
            status_manager,
            user_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 发送消息
    pub async fn send_message(&self, request: SendMessageRequest, sender_id: Uuid) -> Result<SendMessageResponse> {
        let message_id = Uuid::new_v4();
        let message_type = request.message_type.unwrap_or_else(|| "text".to_string());

        // 创建消息
        let message = self
            .message_repository
            .create(
                message_id,
                request.conversation_id,
                sender_id,
                request.content.clone(),
                message_type.clone(),
                request.reply_to,
                request.metadata,
            )
            .await?;

        // 获取发送者信息
        let sender = self.get_user_info(sender_id).await?;

        // 发送给对话中的所有用户
        let ws_message = WSMessage {
            message_type: WSMessageType::Message,
            conversation_id: Some(request.conversation_id),
            message_id: Some(message_id),
            sender_id: Some(sender_id),
            content: Some(request.content),
            timestamp: Some(message.created_at.timestamp()),
            data: None,
        };

        self.connection_manager
            .send_to_conversation(request.conversation_id, ws_message)
            .await;

        // 获取对话参与者并标记已送达
        if let Ok(participants) = self.get_conversation_participants(request.conversation_id).await {
            for participant_id in participants {
                if participant_id != sender_id {
                    // 标记为已送达
                    if let Err(e) = self.message_repository.mark_as_delivered(message_id, participant_id).await {
                        tracing::warn!("Failed to mark message as delivered: {:?}", e);
                    }
                    
                    // 通知在线用户有新消息
                    let delivery_notification = WSMessage {
                        message_type: WSMessageType::NewMessage,
                        conversation_id: Some(request.conversation_id),
                        message_id: Some(message_id),
                        sender_id: Some(sender_id),
                        content: None,
                        timestamp: Some(message.created_at.timestamp()),
                        data: Some(serde_json::json!({
                            "message_id": message_id,
                            "conversation_id": request.conversation_id,
                        })),
                    };
                    self.connection_manager.send_to_user(participant_id, delivery_notification).await;
                }
            }
        }

        Ok(SendMessageResponse {
            message_id,
            conversation_id: request.conversation_id,
            content: message.content,
            message_type,
            sender_id,
            created_at: message.created_at.timestamp(),
        })
    }

    /// 获取消息历史
    pub async fn get_message_history(
        &self,
        conversation_id: Uuid,
        limit: i32,
        before_message_id: Option<Uuid>,
    ) -> Result<MessageHistoryResponse> {
        let messages = self
            .message_repository
            .get_conversation_messages(conversation_id, limit, before_message_id)
            .await?;

        let mut message_infos = Vec::new();

        for message in messages {
            let sender = self.get_user_info(message.sender_id).await?;

            message_infos.push(MessageInfo {
                message_id: message.id,
                conversation_id: message.conversation_id,
                sender_id: message.sender_id,
                sender_username: sender.username,
                sender_avatar_url: sender.avatar_url,
                content: message.content,
                message_type: message.message_type,
                reply_to: message.reply_to,
                metadata: message.metadata,
                created_at: message.created_at.timestamp(),
                read_at: message.read_at,
                delivered_at: message.delivered_at,
            });
        }

        // 反转消息顺序（最新的在前）
        message_infos.reverse();

        let has_more = message_infos.len() == limit as usize;
        let total = message_infos.len() as i32;

        Ok(MessageHistoryResponse {
            conversation_id,
            messages: message_infos,
            has_more,
            total,
        })
    }

    /// 标记消息已读
    pub async fn mark_as_read(&self, conversation_id: Uuid, message_id: Uuid, user_id: Uuid) -> Result<()> {
        // 标记数据库中的已读状态
        self.message_repository
            .mark_as_read(message_id, user_id)
            .await?;

        // 获取消息信息以找到发送者
        if let Ok(Some(message)) = self.message_repository.get_by_id(message_id).await {
            // 通知发送者消息已读
            let ws_message = WSMessage {
                message_type: WSMessageType::Read,
                conversation_id: Some(conversation_id),
                message_id: Some(message_id),
                sender_id: Some(user_id),
                content: None,
                timestamp: Some(Utc::now().timestamp()),
                data: Some(serde_json::json!({
                    "read_by": user_id,
                    "message_id": message_id,
                })),
            };

            // 发送给消息发送者
            self.connection_manager
                .send_to_user(message.sender_id, ws_message)
                .await;
        }

        Ok(())
    }

    /// 编辑消息
    pub async fn edit_message(&self, conversation_id: Uuid, message_id: Uuid, user_id: Uuid, new_content: String) -> Result<()> {
        // 获取原消息验证权限
        let message = self.message_repository.get_by_id(message_id).await?
            .ok_or_else(|| AppError::NotFound("消息不存在".to_string()))?;

        // 验证是否是消息发送者
        if message.sender_id != user_id {
            return Err(AppError::Authorization("只能编辑自己的消息".to_string()));
        }

        // 更新消息内容
        let _updated_message = self.message_repository.update_content(message_id, &new_content).await?;

        // 广播编辑事件到会话
        let ws_message = WSMessage {
            message_type: WSMessageType::Edit,
            conversation_id: Some(conversation_id),
            message_id: Some(message_id),
            sender_id: Some(user_id),
            content: Some(new_content),
            timestamp: Some(Utc::now().timestamp()),
            data: Some(serde_json::json!({
                "message_id": message_id,
                "conversation_id": conversation_id,
            })),
        };

        self.connection_manager.send_to_conversation(conversation_id, ws_message).await;

        Ok(())
    }

    /// 撤回消息
    pub async fn recall_message(&self, conversation_id: Uuid, message_id: Uuid, user_id: Uuid) -> Result<()> {
        // 获取原消息验证权限
        let message = self.message_repository.get_by_id(message_id).await?
            .ok_or_else(|| AppError::NotFound("消息不存在".to_string()))?;

        // 验证是否是消息发送者
        if message.sender_id != user_id {
            return Err(AppError::Authorization("只能撤回自己的消息".to_string()));
        }

        // 撤回消息
        self.message_repository.recall(message_id).await?;

        // 广播撤回事件到会话
        let ws_message = WSMessage {
            message_type: WSMessageType::Recall,
            conversation_id: Some(conversation_id),
            message_id: Some(message_id),
            sender_id: Some(user_id),
            content: None,
            timestamp: Some(Utc::now().timestamp()),
            data: Some(serde_json::json!({
                "message_id": message_id,
                "conversation_id": conversation_id,
            })),
        };

        self.connection_manager.send_to_conversation(conversation_id, ws_message).await;

        Ok(())
    }

    /// 获取在线用户
    pub async fn get_online_users(&self) -> Result<OnlineUsersResponse> {
        let user_ids = self.status_manager.get_online_user_ids().await;
        let mut online_users = Vec::new();

        for user_id in user_ids {
            if let Ok(user) = self.get_user_info(user_id).await {
                let status = self.status_manager.get_status(user_id).await;

                if let Some(status_info) = status {
                    online_users.push(OnlineUserInfo {
                        user_id,
                        username: user.username,
                        avatar_url: user.avatar_url,
                        status: format!("{:?}", status_info.status).to_lowercase(),
                        last_seen: status_info.last_seen,
                    });
                }
            }
        }

        let total = online_users.len() as i32;

        Ok(OnlineUsersResponse {
            online_users,
            total,
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
                self.cache_user(user.clone()).await;
                Ok(user)
            }
            None => {
                // 用户不存在，返回默认信息（避免阻塞消息流）
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

    /// 获取对话参与者ID列表
    async fn get_conversation_participants(&self, _conversation_id: Uuid) -> Result<Vec<Uuid>> {
        // 这里需要从数据库查询对话参与者
        // 暂时返回空列表，后续可以完善
        Ok(Vec::new())
    }

    /// 缓存用户信息
    pub async fn cache_user(&self, user: User) {
        let mut cache = self.user_cache.write().await;
        cache.insert(user.id, user);
    }
}
