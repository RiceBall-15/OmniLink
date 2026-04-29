use common::models::{Conversation, Message, Participant};
use common::{AppError, Result};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use chrono::Utc;

/// 消息仓库
pub struct MessageRepository {
    pool: Pool<Postgres>,
}

impl MessageRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// 创建消息
    pub async fn create(
        &self,
        message_id: Uuid,
        conversation_id: Uuid,
        sender_id: Uuid,
        content: String,
        message_type: String,
        reply_to: Option<Uuid>,
        metadata: Option<serde_json::Value>,
    ) -> Result<Message> {
        let now = Utc::now();

        let message = sqlx::query_as::<_, Message>(
            r#"
            INSERT INTO messages (id, conversation_id, sender_id, content, message_type, reply_to, metadata, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#
        )
        .bind(message_id)
        .bind(conversation_id)
        .bind(sender_id)
        .bind(content)
        .bind(message_type)
        .bind(reply_to)
        .bind(metadata)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e))?;

        // 更新对话的最后消息时间
        sqlx::query(
            "UPDATE conversations SET last_message_at = $1, updated_at = $2 WHERE id = $3"
        )
        .bind(now)
        .bind(now)
        .bind(conversation_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e))?;

        Ok(message)
    }

    /// 获取对话的消息历史
    pub async fn get_conversation_messages(
        &self,
        conversation_id: Uuid,
        limit: i32,
        before_message_id: Option<Uuid>,
    ) -> Result<Vec<Message>> {
        let mut query = String::from(
            "SELECT * FROM messages WHERE conversation_id = $1"
        );
        let mut param_count = 1;

        if let Some(before_id) = before_message_id {
            param_count += 1;
            query.push_str(&format!(" AND created_at < (SELECT created_at FROM messages WHERE id = ${})", param_count));
        }

        query.push_str(" ORDER BY created_at DESC LIMIT $");
        param_count += 1;
        query.push_str(&param_count.to_string()));

        let mut query_builder = sqlx::query_as::<_, Message>(&query);
        query_builder = query_builder.bind(conversation_id);

        if let Some(before_id) = before_message_id {
            query_builder = query_builder.bind(before_id);
        }

        query_builder = query_builder.bind(limit);

        query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(e))
    }

    /// 获取单条消息
    pub async fn get_by_id(&self, message_id: Uuid) -> Result<Option<Message>> {
        let message = sqlx::query_as::<_, Message>(
            "SELECT * FROM messages WHERE id = $1"
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e))?;

        Ok(message)
    }

    /// 标记消息已读
    pub async fn mark_as_read(&self, message_id: Uuid, user_id: Uuid) -> Result<()> {
        let now = Utc::now();

        // 创建已读记录
        sqlx::query(
            r#"
            INSERT INTO message_reads (message_id, user_id, read_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (message_id, user_id)
            DO UPDATE SET read_at = EXCLUDED.read_at
            "#
        )
        .bind(message_id)
        .bind(user_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e))?;

        Ok(())
    }

    /// 标记消息已送达
    pub async fn mark_as_delivered(&self, message_id: Uuid, user_id: Uuid) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO message_deliveries (message_id, user_id, delivered_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (message_id, user_id)
            DO UPDATE SET delivered_at = EXCLUDED.delivered_at
            "#
        )
        .bind(message_id)
        .bind(user_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e))?;

        Ok(())
    }

    /// 获取未读消息数
    pub async fn get_unread_count(&self, user_id: Uuid, conversation_id: Option<Uuid>) -> Result<i64> {
        let result = if let Some(conv_id) = conversation_id {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*) FROM messages m
                WHERE m.conversation_id = $1
                AND m.sender_id != $2
                AND NOT EXISTS (
                    SELECT 1 FROM message_reads mr
                    WHERE mr.message_id = m.id AND mr.user_id = $2
                )
                "#
            )
            .bind(conv_id)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(e))?
        } else {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*) FROM messages m
                JOIN conversation_participants cp ON m.conversation_id = cp.conversation_id
                WHERE cp.user_id = $1
                AND m.sender_id != $1
                AND NOT EXISTS (
                    SELECT 1 FROM message_reads mr
                    WHERE mr.message_id = m.id AND mr.user_id = $1
                )
                "#
            )
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(e))?
        };

        Ok(result)
    }
}

/// 对话仓库
pub struct ConversationRepository {
    pool: Pool<Postgres>,
}

impl ConversationRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// 创建对话
    pub async fn create(
        &self,
        conversation_id: Uuid,
        name: String,
        description: Option<String>,
        is_group: bool,
        created_by: Uuid,
    ) -> Result<Conversation> {
        let now = Utc::now();

        let conversation = sqlx::query_as::<_, Conversation>(
            r#"
            INSERT INTO conversations (id, name, description, is_group, created_by, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#
        )
        .bind(conversation_id)
        .bind(name)
        .bind(description)
        .bind(is_group)
        .bind(created_by)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e))?;

        Ok(conversation)
    }

    /// 获取对话
    pub async fn get_by_id(&self, conversation_id: Uuid) -> Result<Option<Conversation>> {
        let conversation = sqlx::query_as::<_, Conversation>(
            "SELECT * FROM conversations WHERE id = $1"
        )
        .bind(conversation_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e))?;

        Ok(conversation)
    }

    /// 获取用户的对话列表
    pub async fn get_user_conversations(&self, user_id: Uuid) -> Result<Vec<Conversation>> {
        let conversations = sqlx::query_as::<_, Conversation>(
            r#"
            SELECT DISTINCT c.* FROM conversations c
            JOIN conversation_participants cp ON c.id = cp.conversation_id
            WHERE cp.user_id = $1
            ORDER BY c.last_message_at DESC NULLS LAST, c.updated_at DESC
            "#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e))?;

        Ok(conversations)
    }

    /// 添加参与者
    pub async fn add_participant(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
        role: String,
    ) -> Result<Participant> {
        let now = Utc::now();

        let participant = sqlx::query_as::<_, Participant>(
            r#"
            INSERT INTO conversation_participants (conversation_id, user_id, role, joined_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (conversation_id, user_id)
            DO UPDATE SET role = EXCLUDED.role
            RETURNING *
            "#
        )
        .bind(conversation_id)
        .bind(user_id)
        .bind(role)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e))?;

        Ok(participant)
    }

    /// 获取对话参与者
    pub async fn get_participants(&self, conversation_id: Uuid) -> Result<Vec<Participant>> {
        let participants = sqlx::query_as::<_, Participant>(
            "SELECT * FROM conversation_participants WHERE conversation_id = $1 ORDER BY joined_at"
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e))?;

        Ok(participants)
    }

    /// 移除参与者
    pub async fn remove_participant(&self, conversation_id: Uuid, user_id: Uuid) -> Result<()> {
        sqlx::query(
            "DELETE FROM conversation_participants WHERE conversation_id = $1 AND user_id = $2"
        )
        .bind(conversation_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e))?;

        Ok(())
    }
}