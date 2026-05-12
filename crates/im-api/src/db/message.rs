use sqlx::PgPool;
use uuid::Uuid;
use chrono::{Utc, Duration};
use anyhow::Result;

use crate::models::message::{
    MessageEntity, CreateMessageParams, MessageStatus,
};

/// 创建消息
pub async fn create_message(pool: &PgPool, params: CreateMessageParams) -> Result<MessageEntity> {
    let now = Utc::now();
    let type_str = params.type_.to_string();
    let status = MessageStatus::Sent.to_string();

    let id = Uuid::new_v4();

    sqlx::query_as::<_, MessageEntity>(
        r#"
        INSERT INTO messages (id, conversation_id, sender_id, content, type, status, reply_to, metadata, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING *
        "#
    )
    .bind(id)
    .bind(params.conversation_id)
    .bind(params.sender_id)
    .bind(&params.content)
    .bind(&type_str)
    .bind(&status)
    .bind(params.reply_to)
    .bind(params.metadata)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("创建消息失败: {}", e))
}

/// 获取会话的消息列表（分页）
pub async fn get_messages_by_conversation(
    pool: &PgPool,
    conversation_id: &Uuid,
    page: i64,
    limit: i64,
) -> Result<Vec<MessageEntity>> {
    let offset = (page - 1) * limit;

    let messages = sqlx::query_as::<_, MessageEntity>(
        r#"
        SELECT * FROM messages
        WHERE conversation_id = $1
        ORDER BY created_at ASC
        LIMIT $2 OFFSET $3
        "#
    )
    .bind(conversation_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取消息列表失败: {}", e))?;

    Ok(messages)
}

/// 根据 ID 获取消息
pub async fn get_message_by_id(pool: &PgPool, message_id: &Uuid) -> Result<Option<MessageEntity>> {
    let message = sqlx::query_as::<_, MessageEntity>(
        r#"
        SELECT * FROM messages
        WHERE id = $1
        "#
    )
    .bind(message_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取消息失败: {}", e))?;

    Ok(message)
}

/// 更新消息内容
pub async fn update_message_content(
    pool: &PgPool,
    message_id: &Uuid,
    new_content: &str,
) -> Result<MessageEntity> {
    let now = Utc::now();

    let message = sqlx::query_as::<_, MessageEntity>(
        r#"
        UPDATE messages
        SET content = $1, updated_at = $2
        WHERE id = $3
        RETURNING *
        "#
    )
    .bind(new_content)
    .bind(now)
    .bind(message_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("更新消息失败: {}", e))?;

    Ok(message)
}

/// 撤回消息（更新内容为撤回标记）
pub async fn recall_message(pool: &PgPool, message_id: &Uuid) -> Result<MessageEntity> {
    let now = Utc::now();
    let recalled_content = "此消息已撤回";

    let message = sqlx::query_as::<_, MessageEntity>(
        r#"
        UPDATE messages
        SET content = $1, updated_at = $2
        WHERE id = $3
        RETURNING *
        "#
    )
    .bind(recalled_content)
    .bind(now)
    .bind(message_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("撤回消息失败: {}", e))?;

    Ok(message)
}

/// 标记会话的所有消息为已读（针对特定用户）
pub async fn mark_conversation_as_read(
    pool: &PgPool,
    conversation_id: &Uuid,
    user_id: &Uuid,
) -> Result<()> {
    let now = Utc::now();

    sqlx::query(
        r#"
        UPDATE messages
        SET read_at = $1, status = 'read'
        WHERE conversation_id = $2
        AND sender_id != $3
        AND read_at IS NULL
        "#
    )
    .bind(now)
    .bind(conversation_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("标记消息已读失败: {}", e))?;

    Ok(())
}

/// 检查消息是否可以编辑（2分钟内）
pub fn can_edit_message(message: &MessageEntity, user_id: &Uuid) -> bool {
    // 只能编辑自己的消息
    if message.sender_id != *user_id {
        return false;
    }

    // 检查是否在2分钟内
    let now = Utc::now();
    let time_diff = now.signed_duration_since(message.created_at);

    time_diff < Duration::minutes(2)
}

/// 检查消息是否可以撤回（2分钟内）
pub fn can_recall_message(message: &MessageEntity, user_id: &Uuid) -> bool {
    // 只能撤回自己的消息
    if message.sender_id != *user_id {
        return false;
    }

    // 检查是否在2分钟内
    let now = Utc::now();
    let time_diff = now.signed_duration_since(message.created_at);

    time_diff < Duration::minutes(2)
}

/// 获取会话的最后一条消息
pub async fn get_last_message(pool: &PgPool, conversation_id: &Uuid) -> Result<Option<MessageEntity>> {
    let message = sqlx::query_as::<_, MessageEntity>(
        r#"
        SELECT * FROM messages
        WHERE conversation_id = $1
        ORDER BY created_at DESC
        LIMIT 1
        "#
    )
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取最后一条消息失败: {}", e))?;

    Ok(message)
}

/// 统计会话的未读消息数量
pub async fn count_unread_messages(
    pool: &PgPool,
    conversation_id: &Uuid,
    user_id: &Uuid,
) -> Result<i64> {
    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM messages
        WHERE conversation_id = $1
        AND sender_id != $2
        AND read_at IS NULL
        "#
    )
    .bind(conversation_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("统计未读消息失败: {}", e))?;

    Ok(count)
}

/// 搜索会话中的消息（关键词搜索）
pub async fn search_messages_in_conversation(
    pool: &PgPool,
    conversation_id: &Uuid,
    keyword: &str,
    page: i64,
    limit: i64,
) -> Result<Vec<MessageEntity>> {
    let offset = (page - 1) * limit;
    let search_pattern = format!("%{}%", keyword);

    let messages = sqlx::query_as::<_, MessageEntity>(
        r#"
        SELECT * FROM messages
        WHERE conversation_id = $1
        AND content ILIKE $2
        ORDER BY created_at DESC
        LIMIT $3 OFFSET $4
        "#
    )
    .bind(conversation_id)
    .bind(&search_pattern)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("搜索消息失败: {}", e))?;

    Ok(messages)
}

/// 搜索用户所有会话中的消息
pub async fn search_user_messages(
    pool: &PgPool,
    user_id: &Uuid,
    keyword: &str,
    page: i64,
    limit: i64,
) -> Result<Vec<MessageEntity>> {
    let offset = (page - 1) * limit;
    let search_pattern = format!("%{}%", keyword);

    let messages = sqlx::query_as::<_, MessageEntity>(
        r#"
        SELECT m.* FROM messages m
        JOIN conversation_participants cp ON m.conversation_id = cp.conversation_id
        WHERE cp.user_id = $1
        AND m.content ILIKE $2
        ORDER BY m.created_at DESC
        LIMIT $3 OFFSET $4
        "#
    )
    .bind(user_id)
    .bind(&search_pattern)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("搜索消息失败: {}", e))?;

    Ok(messages)
}

/// 获取消息统计
pub async fn get_message_stats(
    pool: &PgPool,
    conversation_id: &Uuid,
) -> Result<MessageStats> {
    let stats = sqlx::query_as::<_, MessageStats>(
        r#"
        SELECT 
            COUNT(*) as total_count,
            COUNT(DISTINCT sender_id) as sender_count,
            MIN(created_at) as first_message_at,
            MAX(created_at) as last_message_at
        FROM messages
        WHERE conversation_id = $1
        "#
    )
    .bind(conversation_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取消息统计失败: {}", e))?;

    Ok(stats)
}

/// 消息统计结构
#[derive(Debug, sqlx::FromRow)]
pub struct MessageStats {
    pub total_count: i64,
    pub sender_count: i64,
    pub first_message_at: Option<chrono::NaiveDateTime>,
    pub last_message_at: Option<chrono::NaiveDateTime>,
}
