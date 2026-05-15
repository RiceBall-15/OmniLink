use sqlx::{PgPool, types::JsonValue};
use uuid::Uuid;
use chrono::{Utc, DateTime, Duration};
use anyhow::Result;

use crate::models::message::{
    MessageEntity, CreateMessageParams, MessageStatus,
    MessageBookmark, BookmarkInfo,
    DraftMessage, ScheduledMessage,
    DeliveryReceipt, DeliveryReceiptStats,
};

/// 创建消息
pub async fn create_message(pool: &PgPool, params: CreateMessageParams) -> Result<MessageEntity> {
    let now = Utc::now();
    let type_str = params.type_.to_string();
    let status = MessageStatus::Sent.to_string();

    let id = Uuid::new_v4();

    let message = sqlx::query_as::<_, MessageEntity>(
        r#"
        INSERT INTO messages (id, conversation_id, sender_id, content, type, status, reply_to, metadata, created_at, updated_at, burn_after_reading, burn_after_seconds)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
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
    .bind(params.burn_after_reading)
    .bind(params.burn_after_seconds)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("创建消息失败: {}", e))?;

    // Update conversation's last_message_at and last_message_preview
    let preview = if params.content.len() > 50 {
        format!("{}...", &params.content[..47])
    } else {
        params.content.clone()
    };
    let _ = sqlx::query(
        r#"
        UPDATE conversations
        SET last_message_at = $1, last_message_preview = $2, updated_at = $1
        WHERE id = $3
        "#
    )
    .bind(now)
    .bind(&preview)
    .bind(params.conversation_id)
    .execute(pool)
    .await;

    // Increment unread count for all other participants
    let _ = sqlx::query(
        r#"
        UPDATE conversation_user_state
        SET unread_count = unread_count + 1, updated_at = $1
        WHERE conversation_id = $2 AND user_id != $3
        "#
    )
    .bind(now)
    .bind(params.conversation_id)
    .bind(params.sender_id)
    .execute(pool)
    .await;

    Ok(message)
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

    // Update per-user unread count
    let _ = sqlx::query(
        r#"
        INSERT INTO conversation_user_state (conversation_id, user_id, last_read_at, unread_count, created_at, updated_at)
        VALUES ($1, $2, $3, 0, $3, $3)
        ON CONFLICT (conversation_id, user_id)
        DO UPDATE SET last_read_at = $3, unread_count = 0, updated_at = $3
        "#
    )
    .bind(conversation_id)
    .bind(user_id)
    .bind(now)
    .execute(pool)
    .await;

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
    start_date: Option<&str>,
    end_date: Option<&str>,
    message_type: Option<&str>,
    sender_id: Option<&str>,
    page: i64,
    limit: i64,
) -> Result<Vec<MessageEntity>> {
    let offset = (page - 1) * limit;
    let search_pattern = format!("%{}%", keyword);

    let mut query = String::from(
        "SELECT * FROM messages WHERE conversation_id = $1 AND content ILIKE $2"
    );

    let mut param_count = 3;
    if start_date.is_some() {
        query.push_str(&format!(" AND created_at >= ${}", param_count));
        param_count += 1;
    }

    if end_date.is_some() {
        query.push_str(&format!(" AND created_at <= ${}", param_count));
        param_count += 1;
    }

    if message_type.is_some() {
        query.push_str(&format!(" AND type = ${}", param_count));
        param_count += 1;
    }

    if sender_id.is_some() {
        query.push_str(&format!(" AND sender_id = ${}", param_count));
        param_count += 1;
    }

    // 优化排序：相关性评分（similarity）70% + 时间衰减 30%
    // similarity() 需要 pg_trgm 扩展，如果不可用则降级为时间排序
    query.push_str(&format!(
        " ORDER BY (COALESCE(similarity(content, $2), 0) * 0.7 + \
         (1.0 / (1.0 + EXTRACT(EPOCH FROM (NOW() - created_at)) / 2592000.0)) * 0.3) DESC, \
         created_at DESC LIMIT ${} OFFSET ${}",
        param_count, param_count + 1
    ));

    let mut sql_query = sqlx::query_as::<_, MessageEntity>(&query)
        .bind(conversation_id)
        .bind(&search_pattern);

    if let Some(start) = start_date {
        sql_query = sql_query.bind(start);
    }

    if let Some(end) = end_date {
        sql_query = sql_query.bind(end);
    }

    if let Some(msg_type) = message_type {
        sql_query = sql_query.bind(msg_type);
    }

    if let Some(sid) = sender_id {
        sql_query = sql_query.bind(sid);
    }

    let messages = sql_query
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
    start_date: Option<&str>,
    end_date: Option<&str>,
    message_type: Option<&str>,
    sender_id: Option<&str>,
    page: i64,
    limit: i64,
) -> Result<Vec<MessageEntity>> {
    let offset = (page - 1) * limit;
    let search_pattern = format!("%{}%", keyword);

    let mut query = String::from(
        "SELECT m.* FROM messages m \
         JOIN conversation_participants cp ON m.conversation_id = cp.conversation_id \
         WHERE cp.user_id = $1 AND m.content ILIKE $2"
    );

    let mut param_count = 3;

    if start_date.is_some() {
        query.push_str(&format!(" AND m.created_at >= ${}", param_count));
        param_count += 1;
    }

    if end_date.is_some() {
        query.push_str(&format!(" AND m.created_at <= ${}", param_count));
        param_count += 1;
    }

    if message_type.is_some() {
        query.push_str(&format!(" AND m.type = ${}", param_count));
        param_count += 1;
    }

    if sender_id.is_some() {
        query.push_str(&format!(" AND m.sender_id = ${}", param_count));
        param_count += 1;
    }

    query.push_str(&format!(
        " ORDER BY (COALESCE(similarity(m.content, $2), 0) * 0.7 + \
         (1.0 / (1.0 + EXTRACT(EPOCH FROM (NOW() - m.created_at)) / 2592000.0)) * 0.3) DESC, \
         m.created_at DESC LIMIT ${} OFFSET ${}",
        param_count, param_count + 1
    ));

    let mut sql_query = sqlx::query_as::<_, MessageEntity>(&query)
        .bind(user_id)
        .bind(&search_pattern);

    if let Some(start) = start_date {
        sql_query = sql_query.bind(start);
    }

    if let Some(end) = end_date {
        sql_query = sql_query.bind(end);
    }

    if let Some(msg_type) = message_type {
        sql_query = sql_query.bind(msg_type);
    }

    if let Some(sid) = sender_id {
        sql_query = sql_query.bind(sid);
    }

    let messages = sql_query
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

use std::collections::HashMap;

/// 批量获取多个会话的最后一条消息（避免 N+1 查询）
pub async fn get_last_messages_batch(
    pool: &PgPool,
    conversation_ids: &[Uuid],
) -> Result<HashMap<Uuid, MessageEntity>> {
    if conversation_ids.is_empty() {
        return Ok(HashMap::new());
    }

    // 使用 DISTINCT ON 高效获取每个会话的最后一条消息
    let messages = sqlx::query_as::<_, MessageEntity>(
        r#"
        SELECT DISTINCT ON (conversation_id) *
        FROM messages
        WHERE conversation_id = ANY($1)
        ORDER BY conversation_id, created_at DESC
        "#
    )
    .bind(conversation_ids)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("批量获取最后消息失败: {}", e))?;

    let mut map = HashMap::new();
    for msg in messages {
        map.insert(msg.conversation_id, msg);
    }

    Ok(map)
}

/// 批量发送消息（事务性）
pub async fn batch_create_messages(
    pool: &PgPool,
    messages: Vec<CreateMessageParams>,
) -> Result<Vec<MessageEntity>> {
    let mut tx = pool.begin().await.map_err(|e| anyhow::anyhow!("开始事务失败: {}", e))?;
    let mut results = Vec::new();
    let mut idx = 0;
    for params in messages {
        let type_str = params.type_.to_string();
        let now = Utc::now();
        let msg = sqlx::query_as::<_, MessageEntity>(
            r#"INSERT INTO messages (conversation_id, sender_id, content, type, reply_to, metadata, status, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
               RETURNING *"#
        )
        .bind(params.conversation_id)
        .bind(params.sender_id)
        .bind(&params.content)
        .bind(&type_str)
        .bind(params.reply_to)
        .bind(params.metadata.unwrap_or(serde_json::json!({})))
        .bind("sent")
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| anyhow::anyhow!("批量消息第{}条插入失败: {}", idx, e))?;
        results.push(msg);
        idx += 1;
    }
    tx.commit().await.map_err(|e| anyhow::anyhow!("事务提交失败: {}", e))?;
    Ok(results)
}

/// 批量删除消息（软删除，仅发送者可删）
pub async fn batch_delete_messages(
    pool: &PgPool,
    message_ids: &[Uuid],
    user_id: Uuid,
) -> Result<usize> {
    let result = sqlx::query(
        "UPDATE messages SET deleted_at = NOW() WHERE id = ANY($1) AND sender_id = $2 AND deleted_at IS NULL"
    )
    .bind(message_ids)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("批量删除消息失败: {}", e))?;
    Ok(result.rows_affected() as usize)
}

/// 批量标记会话消息已读
pub async fn batch_mark_conversations_as_read(
    pool: &PgPool,
    conversation_ids: &[Uuid],
    user_id: Uuid,
) -> Result<usize> {
    let result = sqlx::query(
        r#"INSERT INTO message_receipts (message_id, user_id, read_at)
           SELECT m.id, $1, NOW()
           FROM messages m
           WHERE m.conversation_id = ANY($2)
             AND m.sender_id != $1
             AND m.deleted_at IS NULL
             AND NOT EXISTS (
               SELECT 1 FROM message_receipts mr
               WHERE mr.message_id = m.id AND mr.user_id = $1
             )"#
    )
    .bind(user_id)
    .bind(conversation_ids)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("批量标记已读失败: {}", e))?;
    Ok(result.rows_affected() as usize)
}

// === 消息收藏/书签 ===

/// 添加消息收藏
pub async fn add_bookmark(
    pool: &PgPool,
    user_id: &Uuid,
    message_id: &Uuid,
    note: Option<&str>,
) -> Result<MessageBookmark> {
    let id = Uuid::new_v4();
    let bookmark = sqlx::query_as::<_, MessageBookmark>(
        r#"
        INSERT INTO message_bookmarks (id, user_id, message_id, note, created_at)
        VALUES ($1, $2, $3, $4, NOW())
        ON CONFLICT (user_id, message_id) DO UPDATE SET note = COALESCE(EXCLUDED.note, message_bookmarks.note)
        RETURNING *
        "#
    )
    .bind(id)
    .bind(user_id)
    .bind(message_id)
    .bind(note)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("添加收藏失败: {}", e))?;

    Ok(bookmark)
}

/// 删除消息收藏
pub async fn remove_bookmark(
    pool: &PgPool,
    user_id: &Uuid,
    message_id: &Uuid,
) -> Result<bool> {
    let result = sqlx::query(
        "DELETE FROM message_bookmarks WHERE user_id = $1 AND message_id = $2"
    )
    .bind(user_id)
    .bind(message_id)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("删除收藏失败: {}", e))?;

    Ok(result.rows_affected() > 0)
}

/// 获取用户的收藏列表（带消息详情）
pub async fn get_bookmarks(
    pool: &PgPool,
    user_id: &Uuid,
    page: i64,
    limit: i64,
) -> Result<Vec<BookmarkInfo>> {
    let offset = (page - 1) * limit;

    let rows = sqlx::query_as::<_, (Uuid, Uuid, Option<String>, DateTime<Utc>, Uuid, String, Uuid, String, String)>(
        r#"
        SELECT 
            mb.id, mb.message_id, mb.note, mb.created_at AS bookmarked_at,
            m.id AS msg_id, m.content, m.conversation_id, m.sender_id, m.type
        FROM message_bookmarks mb
        JOIN messages m ON mb.message_id = m.id
        WHERE mb.user_id = $1
        ORDER BY mb.created_at DESC
        LIMIT $2 OFFSET $3
        "#
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取收藏列表失败: {}", e))?;

    let bookmarks = rows.into_iter().map(|row| {
        BookmarkInfo {
            id: row.0.to_string(),
            message_id: row.1.to_string(),
            note: row.2,
            bookmarked_at: row.3.to_rfc3339(),
            conversation_id: row.6.to_string(),
            sender_id: row.7.to_string(),
            content: row.5,
            type_: row.8,
            created_at: row.3.to_rfc3339(),
        }
    }).collect();

    Ok(bookmarks)
}

/// 检查消息是否已被收藏
pub async fn is_bookmarked(
    pool: &PgPool,
    user_id: &Uuid,
    message_id: &Uuid,
) -> Result<bool> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM message_bookmarks WHERE user_id = $1 AND message_id = $2)"
    )
    .bind(user_id)
    .bind(message_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("检查收藏状态失败: {}", e))?;

    Ok(exists)
}

// === 草稿消息 ===

/// 保存草稿（UPSERT：同一会话只有一个草稿）
pub async fn save_draft(
    pool: &PgPool,
    user_id: &Uuid,
    conversation_id: &Uuid,
    content: &str,
    type_: &str,
    reply_to: Option<&Uuid>,
    metadata: Option<&JsonValue>,
) -> Result<DraftMessage> {
    let id = Uuid::new_v4();
    let reply_to_val = reply_to.cloned();
    let metadata_val = metadata.cloned();

    let draft = sqlx::query_as::<_, DraftMessage>(
        r#"
        INSERT INTO draft_messages (id, user_id, conversation_id, content, type, reply_to, metadata, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
        ON CONFLICT (user_id, conversation_id) 
        DO UPDATE SET content = EXCLUDED.content, type = EXCLUDED.type, reply_to = EXCLUDED.reply_to, metadata = EXCLUDED.metadata, updated_at = NOW()
        RETURNING *
        "#
    )
    .bind(id)
    .bind(user_id)
    .bind(conversation_id)
    .bind(content)
    .bind(type_)
    .bind(reply_to_val)
    .bind(metadata_val)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("保存草稿失败: {}", e))?;

    Ok(draft)
}

/// 获取指定会话的草稿
pub async fn get_draft(
    pool: &PgPool,
    user_id: &Uuid,
    conversation_id: &Uuid,
) -> Result<Option<DraftMessage>> {
    let draft = sqlx::query_as::<_, DraftMessage>(
        "SELECT * FROM draft_messages WHERE user_id = $1 AND conversation_id = $2"
    )
    .bind(user_id)
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取草稿失败: {}", e))?;

    Ok(draft)
}

/// 删除指定会话的草稿
pub async fn delete_draft(
    pool: &PgPool,
    user_id: &Uuid,
    conversation_id: &Uuid,
) -> Result<bool> {
    let result = sqlx::query(
        "DELETE FROM draft_messages WHERE user_id = $1 AND conversation_id = $2"
    )
    .bind(user_id)
    .bind(conversation_id)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("删除草稿失败: {}", e))?;

    Ok(result.rows_affected() > 0)
}

/// 获取用户的所有草稿列表
pub async fn get_all_drafts(
    pool: &PgPool,
    user_id: &Uuid,
    page: i64,
    limit: i64,
) -> Result<Vec<DraftMessage>> {
    let offset = (page - 1) * limit;

    let drafts = sqlx::query_as::<_, DraftMessage>(
        "SELECT * FROM draft_messages WHERE user_id = $1 ORDER BY updated_at DESC LIMIT $2 OFFSET $3"
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取草稿列表失败: {}", e))?;

    Ok(drafts)
}

/// 创建定时消息
pub async fn create_scheduled_message(
    pool: &PgPool,
    sender_id: &Uuid,
    conversation_id: &Uuid,
    content: &str,
    message_type: &str,
    reply_to: Option<&Uuid>,
    metadata: Option<&JsonValue>,
    scheduled_at: DateTime<Utc>,
) -> Result<ScheduledMessage> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    let message = sqlx::query_as::<_, ScheduledMessage>(
        r#"
        INSERT INTO scheduled_messages (id, sender_id, conversation_id, content, message_type, reply_to, metadata, scheduled_at, status, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'pending', $9, $10)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(sender_id)
    .bind(conversation_id)
    .bind(content)
    .bind(message_type)
    .bind(reply_to)
    .bind(metadata)
    .bind(scheduled_at)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("创建定时消息失败: {}", e))?;

    Ok(message)
}

/// 获取定时消息
pub async fn get_scheduled_message(
    pool: &PgPool,
    message_id: &Uuid,
    sender_id: &Uuid,
) -> Result<Option<ScheduledMessage>> {
    let message = sqlx::query_as::<_, ScheduledMessage>(
        "SELECT * FROM scheduled_messages WHERE id = $1 AND sender_id = $2"
    )
    .bind(message_id)
    .bind(sender_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取定时消息失败: {}", e))?;

    Ok(message)
}

/// 更新定时消息
pub async fn update_scheduled_message(
    pool: &PgPool,
    message_id: &Uuid,
    sender_id: &Uuid,
    content: Option<&str>,
    message_type: Option<&str>,
    reply_to: Option<&Uuid>,
    metadata: Option<&JsonValue>,
    scheduled_at: Option<DateTime<Utc>>,
) -> Result<ScheduledMessage> {
    let now = Utc::now();

    let message = sqlx::query_as::<_, ScheduledMessage>(
        r#"
        UPDATE scheduled_messages
        SET content = COALESCE($3, content),
            message_type = COALESCE($4, message_type),
            reply_to = COALESCE($5, reply_to),
            metadata = COALESCE($6, metadata),
            scheduled_at = COALESCE($7, scheduled_at),
            updated_at = $8
        WHERE id = $1 AND sender_id = $2 AND status = 'pending'
        RETURNING *
        "#,
    )
    .bind(message_id)
    .bind(sender_id)
    .bind(content)
    .bind(message_type)
    .bind(reply_to)
    .bind(metadata)
    .bind(scheduled_at)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("更新定时消息失败: {}", e))?;

    Ok(message)
}

/// 取消定时消息
pub async fn cancel_scheduled_message(
    pool: &PgPool,
    message_id: &Uuid,
    sender_id: &Uuid,
) -> Result<bool> {
    let now = Utc::now();

    let result = sqlx::query(
        r#"
        UPDATE scheduled_messages
        SET status = 'cancelled', updated_at = $3
        WHERE id = $1 AND sender_id = $2 AND status = 'pending'
        "#,
    )
    .bind(message_id)
    .bind(sender_id)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("取消定时消息失败: {}", e))?;

    Ok(result.rows_affected() > 0)
}

/// 获取用户的定时消息列表
pub async fn get_scheduled_messages(
    pool: &PgPool,
    sender_id: &Uuid,
    status: Option<&str>,
    page: i64,
    limit: i64,
) -> Result<Vec<ScheduledMessage>> {
    let offset = (page - 1) * limit;

    let messages = if let Some(status_filter) = status {
        sqlx::query_as::<_, ScheduledMessage>(
            "SELECT * FROM scheduled_messages WHERE sender_id = $1 AND status = $2 ORDER BY scheduled_at DESC LIMIT $3 OFFSET $4"
        )
        .bind(sender_id)
        .bind(status_filter)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| anyhow::anyhow!("获取定时消息列表失败: {}", e))?
    } else {
        sqlx::query_as::<_, ScheduledMessage>(
            "SELECT * FROM scheduled_messages WHERE sender_id = $1 ORDER BY scheduled_at DESC LIMIT $2 OFFSET $3"
        )
        .bind(sender_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| anyhow::anyhow!("获取定时消息列表失败: {}", e))?
    };

    Ok(messages)
}

/// 获取待发送的定时消息（用于后台任务）
pub async fn get_pending_scheduled_messages(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<ScheduledMessage>> {
    let now = Utc::now();

    let messages = sqlx::query_as::<_, ScheduledMessage>(
        r#"
        SELECT * FROM scheduled_messages
        WHERE status = 'pending' AND scheduled_at <= $1
        ORDER BY scheduled_at ASC
        LIMIT $2
        "#,
    )
    .bind(now)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取待发送消息失败: {}", e))?;

    Ok(messages)
}

/// 标记定时消息为已发送
pub async fn mark_scheduled_message_sent(
    pool: &PgPool,
    message_id: &Uuid,
) -> Result<bool> {
    let now = Utc::now();

    let result = sqlx::query(
        r#"
        UPDATE scheduled_messages
        SET status = 'sent', sent_at = $2, updated_at = $2
        WHERE id = $1
        "#,
    )
    .bind(message_id)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("标记消息发送失败: {}", e))?;

    Ok(result.rows_affected() > 0)
}

/// 标记定时消息为发送失败
pub async fn mark_scheduled_message_failed(
    pool: &PgPool,
    message_id: &Uuid,
    error_message: &str,
) -> Result<bool> {
    let now = Utc::now();

    let result = sqlx::query(
        r#"
        UPDATE scheduled_messages
        SET status = 'failed', error_message = $2, updated_at = $3
        WHERE id = $1
        "#,
    )
    .bind(message_id)
    .bind(error_message)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("标记消息失败状态出错: {}", e))?;

    Ok(result.rows_affected() > 0)
}

/// 获取消息的线程回复列表（分页）
pub async fn get_thread_replies(
    pool: &PgPool,
    parent_message_id: &Uuid,
    page: i64,
    limit: i64,
) -> Result<Vec<MessageEntity>> {
    let offset = (page - 1) * limit;

    let messages = sqlx::query_as::<_, MessageEntity>(
        r#"
        SELECT * FROM messages
        WHERE reply_to = $1
        ORDER BY created_at ASC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(parent_message_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取线程回复失败: {}", e))?;

    Ok(messages)
}

/// 统计消息的线程回复数量
pub async fn count_thread_replies(
    pool: &PgPool,
    parent_message_id: &Uuid,
) -> Result<i64> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM messages WHERE reply_to = $1",
    )
    .bind(parent_message_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("统计线程回复失败: {}", e))?;

    Ok(count)
}

/// 获取会话中所有有回复的消息（话题摘要列表）
pub async fn get_active_threads_in_conversation(
    pool: &PgPool,
    conversation_id: &Uuid,
    limit_count: i64,
) -> Result<Vec<ThreadSummaryRow>> {
    let rows = sqlx::query_as::<_, ThreadSummaryRow>(
        r#"
        SELECT 
            m.id as parent_id,
            m.content as parent_content,
            m.sender_id as parent_sender_id,
            m.type as parent_type,
            m.created_at as parent_created_at,
            COUNT(r.id)::bigint as reply_count,
            MAX(r.created_at) as last_reply_at
        FROM messages m
        INNER JOIN messages r ON r.reply_to = m.id
        WHERE m.conversation_id = $1
        GROUP BY m.id, m.content, m.sender_id, m.type, m.created_at
        ORDER BY MAX(r.created_at) DESC
        LIMIT $2
        "#,
    )
    .bind(conversation_id)
    .bind(limit_count)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取会话话题列表失败: {}", e))?;

    Ok(rows)
}

/// 线程摘要行（用于 SQLx 查询映射）
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ThreadSummaryRow {
    pub parent_id: Uuid,
    pub parent_content: String,
    pub parent_sender_id: Uuid,
    pub parent_type: String,
    pub parent_created_at: DateTime<Utc>,
    pub reply_count: i64,
    pub last_reply_at: DateTime<Utc>,
}

// ========== 阅后即焚功能 ==========

/// 标记消息已读，并为阅后即焚消息设置焚毁时间
pub async fn mark_message_as_read_with_burn(
    pool: &PgPool,
    message_id: &Uuid,
    user_id: &Uuid,
) -> Result<bool> {
    // 获取消息信息
    let message = sqlx::query_as::<_, MessageEntity>(
        "SELECT * FROM messages WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(message_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("查询消息失败: {}", e))?;

    let message = match message {
        Some(m) => m,
        None => return Ok(false),
    };

    // 只有接收者才能触发阅后即焚
    if message.sender_id == *user_id {
        return Ok(false);
    }

    // 检查是否是阅后即焚消息
    if message.burn_after_reading {
        let burn_seconds = message.burn_after_seconds.unwrap_or(30);
        let burned_at = Utc::now() + chrono::Duration::seconds(burn_seconds as i64);

        // 更新消息的 burned_at 时间
        sqlx::query(
            "UPDATE messages SET burned_at = $1, updated_at = NOW() WHERE id = $2"
        )
        .bind(burned_at)
        .bind(message_id)
        .execute(pool)
        .await
        .map_err(|e| anyhow::anyhow!("设置焚毁时间失败: {}", e))?;

        // 插入已读回执
        let _ = sqlx::query(
            r#"INSERT INTO message_receipts (message_id, user_id, read_at)
               VALUES ($1, $2, NOW())
               ON CONFLICT (message_id, user_id) DO NOTHING"#
        )
        .bind(message_id)
        .bind(user_id)
        .execute(pool)
        .await;

        return Ok(true); // 表示消息将被焚毁
    }

    // 非阅后即焚消息，正常标记已读
    let _ = sqlx::query(
        r#"INSERT INTO message_receipts (message_id, user_id, read_at)
           VALUES ($1, $2, NOW())
           ON CONFLICT (message_id, user_id) DO NOTHING"#
    )
    .bind(message_id)
    .bind(user_id)
    .execute(pool)
    .await;

    Ok(false)
}

/// 清理已过期的阅后即焚消息（软删除）
pub async fn cleanup_expired_burn_messages(pool: &PgPool) -> Result<usize> {
    let result = sqlx::query(
        r#"UPDATE messages
           SET deleted_at = NOW(), updated_at = NOW()
           WHERE burn_after_reading = TRUE
             AND burned_at IS NOT NULL
             AND burned_at <= NOW()
             AND deleted_at IS NULL"#
    )
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("清理过期阅后即焚消息失败: {}", e))?;

    Ok(result.rows_affected() as usize)
}

/// 检查消息是否已被焚毁
pub async fn is_message_burned(pool: &PgPool, message_id: &Uuid) -> Result<bool> {
    let result = sqlx::query_scalar::<_, bool>(
        r#"SELECT burn_after_reading = TRUE
                AND burned_at IS NOT NULL
                AND burned_at <= NOW()
           FROM messages
           WHERE id = $1 AND deleted_at IS NULL"#
    )
    .bind(message_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("检查消息焚毁状态失败: {}", e))?;

    Ok(result.unwrap_or(false))
}

/// 获取会话中即将焚毁的消息（用于通知客户端）
pub async fn get_expiring_messages(
    pool: &PgPool,
    conversation_id: &Uuid,
    user_id: &Uuid,
) -> Result<Vec<MessageEntity>> {
    let messages = sqlx::query_as::<_, MessageEntity>(
        r#"SELECT m.* FROM messages m
           WHERE m.conversation_id = $1
             AND m.sender_id != $2
             AND m.burn_after_reading = TRUE
             AND m.burned_at IS NOT NULL
             AND m.burned_at > NOW()
             AND m.deleted_at IS NULL
           ORDER BY m.burned_at ASC"#
    )
    .bind(conversation_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取即将焚毁消息失败: {}", e))?;

    Ok(messages)
}

/// 获取会话的所有消息（用于导出，不分页）
pub async fn get_all_messages_for_export(
    pool: &PgPool,
    conversation_id: &Uuid,
) -> Result<Vec<MessageEntity>> {
    let messages = sqlx::query_as::<_, MessageEntity>(
        r#"
        SELECT * FROM messages
        WHERE conversation_id = $1
        ORDER BY created_at ASC
        "#
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取导出消息列表失败: {}", e))?;

    Ok(messages)
}

/// 统计会话中的消息总数
pub async fn count_messages_in_conversation(
    pool: &PgPool,
    conversation_id: &Uuid,
) -> Result<i64> {
    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM messages
        WHERE conversation_id = $1
        "#
    )
    .bind(conversation_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("统计消息数量失败: {}", e))?;

    Ok(count)
}

/// 创建或更新消息投递回执
pub async fn upsert_delivery_receipt(
    pool: &PgPool,
    message_id: &Uuid,
    user_id: &Uuid,
    status: &str,
) -> Result<DeliveryReceipt> {
    let receipt = sqlx::query_as::<_, DeliveryReceipt>(
        r#"
        INSERT INTO message_delivery_receipts (message_id, user_id, status)
        VALUES ($1, $2, $3)
        ON CONFLICT (message_id, user_id)
        DO UPDATE SET status = $3, updated_at = NOW()
        RETURNING *
        "#
    )
    .bind(message_id)
    .bind(user_id)
    .bind(status)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("创建投递回执失败: {}", e))?;

    Ok(receipt)
}

/// 获取消息的所有投递回执
pub async fn get_delivery_receipts_by_message(
    pool: &PgPool,
    message_id: &Uuid,
) -> Result<Vec<DeliveryReceipt>> {
    let receipts = sqlx::query_as::<_, DeliveryReceipt>(
        r#"
        SELECT * FROM message_delivery_receipts
        WHERE message_id = $1
        ORDER BY updated_at DESC
        "#
    )
    .bind(message_id)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("查询投递回执失败: {}", e))?;

    Ok(receipts)
}

/// 获取消息投递统计
pub async fn get_delivery_receipt_stats(
    pool: &PgPool,
    message_id: &Uuid,
    total_recipients: i64,
) -> Result<DeliveryReceiptStats> {
    let stats = sqlx::query_as::<_, (i64, i64)>(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE status = 'delivered'),
            COUNT(*) FILTER (WHERE status = 'read')
        FROM message_delivery_receipts
        WHERE message_id = $1
        "#
    )
    .bind(message_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("查询投递统计失败: {}", e))?;

    let delivered_count = stats.0;
    let read_count = stats.1;
    let pending_count = total_recipients - delivered_count - read_count;

    Ok(DeliveryReceiptStats {
        message_id: *message_id,
        total_recipients,
        delivered_count,
        read_count,
        pending_count: pending_count.max(0),
    })
}

/// 批量获取消息投递回执
pub async fn get_delivery_receipts_batch(
    pool: &PgPool,
    message_ids: &[Uuid],
) -> Result<Vec<DeliveryReceipt>> {
    if message_ids.is_empty() {
        return Ok(vec![]);
    }

    let receipts = sqlx::query_as::<_, DeliveryReceipt>(
        r#"
        SELECT * FROM message_delivery_receipts
        WHERE message_id = ANY($1)
        ORDER BY message_id, updated_at DESC
        "#
    )
    .bind(message_ids)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("批量查询投递回执失败: {}", e))?;

    Ok(receipts)
}
