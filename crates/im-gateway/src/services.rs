use super::super::models::{
    SendMessageRequest, SendMessageResponse, MessageHistoryResponse, MessageInfo,
    OnlineUsersResponse, OnlineUserInfo, WSMessage, WSMessageType,
};
use super::super::repository::MessageRepository;
use super::super::connection_manager::WSConnectionManager;
use super::super::status_manager::OnlineStatusManager;
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
    connection_manager: Arc<WSConnectionManager>,
    status_manager: Arc<OnlineStatusManager>,
    user_cache: Arc<RwLock<HashMap<Uuid, User>>>,
}

impl IMService {
    pub fn new(
        message_repository: Arc<MessageRepository>,
        connection_manager: Arc<WSConnectionManager>,
        status_manager: Arc<OnlineStatusManager>,
    ) -> Self {
        Self {
            message_repository,
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

        // 获取参与者列表
        // TODO: 获取对话参与者并标记已送达

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

        Ok(MessageHistoryResponse {
            conversation_id,
            messages: message_infos,
            has_more,
            total: messages.len() as i32,
        })
    }

    /// 标记消息已读
    pub async fn mark_as_read(&self, conversation_id: Uuid, message_id: Uuid, user_id: Uuid) -> Result<()> {
        // 标记数据库中的已读状态
        self.message_repository
            .mark_as_read(message_id, user_id)
            .await?;

        // 通知发送者消息已读
        let ws_message = WSMessage {
            message_type: WSMessageType::MessageRead,
            conversation_id: Some(conversation_id),
            message_id: Some(message_id),
            sender_id: Some(user_id),
            content: None,
            timestamp: Some(Utc::now().timestamp()),
            data: None,
        };

        // TODO: 获取消息发送者并发送通知

        Ok(())
    }

    /// 获取在线用户
    pub async fn get_online_users(&self) -> Result<OnlineUsersResponse> {
        let user_ids = self.status_manager.get_online_user_ids().await;
        let mut online_users = Vec::new();

        for user_id in user_ids {
            if let Some(user) = self.get_user_info(user_id).await {
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

        Ok(OnlineUsersResponse {
            online_users,
            total: online_users.len() as i32,
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

    /// 缓存用户信息
    pub async fn cache_user(&self, user: User) {
        let mut cache = self.user_cache.write().await;
        cache.insert(user.id, user);
    }
}