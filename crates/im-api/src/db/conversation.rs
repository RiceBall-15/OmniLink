use sqlx::PgPool;
use uuid::Uuid;
use anyhow::Result;

use crate::models::conversation::{
    ConversationEntity, Conversation, CreateConversationParams, ConversationType,
};

/// 创建会话
pub async fn create_conversation(pool: &PgPool, params: CreateConversationParams) -> Result<ConversationEntity> {
    let now = chrono::Utc::now();
    let type_str = params.type_.to_string();
    let id = Uuid::new_v4();

    // 开始事务
    let mut tx = pool.begin().await
        .map_err(|e| anyhow::anyhow!("开始事务失败: {}", e))?;

    // 插入会话
    let conversation = sqlx::query_as::<_, ConversationEntity>(
        r#"
        INSERT INTO conversations (id, type, name, avatar, created_by, unread_count, is_pinned, is_muted, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, 0, FALSE, FALSE, $6, $7)
        RETURNING *
        "#
    )
    .bind(id)
    .bind(&type_str)
    .bind(&params.name)
    .bind(&params.avatar)
    .bind(params.created_by)
    .bind(now)
    .bind(now)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| anyhow::anyhow!("创建会话失败: {}", e))?;

    // 添加参与者
    for participant_id in params.participant_ids {
        sqlx::query(
            r#"
            INSERT INTO conversation_participants (conversation_id, user_id, joined_at)
            VALUES ($1, $2, $3)
            "#
        )
        .bind(id)
        .bind(participant_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| anyhow::anyhow!("添加参与者失败: {}", e))?;
    }

    // 提交事务
    tx.commit().await
        .map_err(|e| anyhow::anyhow!("提交事务失败: {}", e))?;

    Ok(conversation)
}

/// 根据用户 ID 获取会话列表
pub async fn get_conversations_by_user(pool: &PgPool, user_id: &Uuid) -> Result<Vec<ConversationEntity>> {
    let conversations = sqlx::query_as::<_, ConversationEntity>(
        r#"
        SELECT DISTINCT c.* FROM conversations c
        INNER JOIN conversation_participants cp ON c.id = cp.conversation_id
        WHERE cp.user_id = $1
        ORDER BY c.updated_at DESC
        "#
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取会话列表失败: {}", e))?;

    Ok(conversations)
}

/// 根据 ID 获取会话
pub async fn get_conversation_by_id(pool: &PgPool, conversation_id: &Uuid) -> Result<Option<ConversationEntity>> {
    let conversation = sqlx::query_as::<_, ConversationEntity>(
        r#"
        SELECT * FROM conversations
        WHERE id = $1
        "#
    )
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取会话失败: {}", e))?;

    Ok(conversation)
}

/// 检查用户是否是会话参与者
pub async fn is_conversation_participant(pool: &PgPool, conversation_id: &Uuid, user_id: &Uuid) -> Result<bool> {
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM conversation_participants
            WHERE conversation_id = $1 AND user_id = $2
        )
        "#
    )
    .bind(conversation_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("检查参与者失败: {}", e))?;

    Ok(exists)
}

/// 更新会话未读计数
pub async fn update_unread_count(pool: &PgPool, conversation_id: &Uuid, user_id: &Uuid, count: i32) -> Result<()> {
    // 这里简化处理，直接更新会话的 unread_count
    // 实际应用中可能需要为每个用户维护未读计数
    sqlx::query(
        r#"
        UPDATE conversations
        SET unread_count = $1
        WHERE id = $2
        "#
    )
    .bind(count)
    .bind(conversation_id)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("更新未读计数失败: {}", e))?;

    Ok(())
}

/// 更新会话信息
pub async fn update_conversation(
    pool: &PgPool,
    conversation_id: &Uuid,
    name: Option<String>,
    avatar: Option<String>,
) -> Result<ConversationEntity> {
    let now = chrono::Utc::now();

    let conversation = sqlx::query_as::<_, ConversationEntity>(
        r#"
        UPDATE conversations
        SET name = COALESCE($1, name),
            avatar = COALESCE($2, avatar),
            updated_at = $3
        WHERE id = $4
        RETURNING *
        "#
    )
    .bind(name)
    .bind(avatar)
    .bind(now)
    .bind(conversation_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("更新会话失败: {}", e))?;

    Ok(conversation)
}

/// 切换会话置顶状态
pub async fn toggle_pin_conversation(
    pool: &PgPool,
    conversation_id: &Uuid,
    is_pinned: bool,
) -> Result<ConversationEntity> {
    let now = chrono::Utc::now();

    let conversation = sqlx::query_as::<_, ConversationEntity>(
        r#"
        UPDATE conversations
        SET is_pinned = $1, updated_at = $2
        WHERE id = $3
        RETURNING *
        "#
    )
    .bind(is_pinned)
    .bind(now)
    .bind(conversation_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("更新置顶状态失败: {}", e))?;

    Ok(conversation)
}

/// 切换会话静音状态
pub async fn toggle_mute_conversation(
    pool: &PgPool,
    conversation_id: &Uuid,
    is_muted: bool,
) -> Result<ConversationEntity> {
    let now = chrono::Utc::now();

    let conversation = sqlx::query_as::<_, ConversationEntity>(
        r#"
        UPDATE conversations
        SET is_muted = $1, updated_at = $2
        WHERE id = $3
        RETURNING *
        "#
    )
    .bind(is_muted)
    .bind(now)
    .bind(conversation_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("更新静音状态失败: {}", e))?;

    Ok(conversation)
}

/// 查找或创建直接会话
pub async fn find_or_create_direct_conversation(
    pool: &PgPool,
    user_id: &Uuid,
    other_user_id: &Uuid,
) -> Result<ConversationEntity> {
    // 首先查找是否已存在两个用户的直接会话
    let existing = sqlx::query_as::<_, ConversationEntity>(
        r#"
        SELECT DISTINCT c.* FROM conversations c
        WHERE c.type = 'direct'
        AND EXISTS (
            SELECT 1 FROM conversation_participants cp
            WHERE cp.conversation_id = c.id AND cp.user_id = $1
        )
        AND EXISTS (
            SELECT 1 FROM conversation_participants cp
            WHERE cp.conversation_id = c.id AND cp.user_id = $2
        )
        LIMIT 1
        "#
    )
    .bind(user_id)
    .bind(other_user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("查找直接会话失败: {}", e))?;

    if let Some(conv) = existing {
        return Ok(conv);
    }

    // 创建新的直接会话
    let params = CreateConversationParams {
        type_: ConversationType::Direct,
        name: None,
        avatar: None,
        created_by: *user_id,
        participant_ids: vec![*user_id, *other_user_id],
    };

    create_conversation(pool, params).await
}

/// 获取会话参与者列表
pub async fn get_conversation_participants(
    pool: &PgPool,
    conversation_id: &Uuid,
) -> Result<Vec<Uuid>> {
    let participants = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT user_id FROM conversation_participants
        WHERE conversation_id = $1
        ORDER BY joined_at ASC
        "#
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取参与者列表失败: {}", e))?;

    Ok(participants)
}

/// 添加参与者到会话
pub async fn add_participant(
    pool: &PgPool,
    conversation_id: &Uuid,
    user_id: &Uuid,
) -> Result<()> {
    let now = chrono::Utc::now();

    // 检查是否已经是参与者
    let exists = is_conversation_participant(pool, conversation_id, user_id).await?;
    if exists {
        return Ok(());
    }

    sqlx::query(
        r#"
        INSERT INTO conversation_participants (conversation_id, user_id, joined_at)
        VALUES ($1, $2, $3)
        "#
    )
    .bind(conversation_id)
    .bind(user_id)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("添加参与者失败: {}", e))?;

    Ok(())
}

/// 从会话中移除参与者
pub async fn remove_participant(
    pool: &PgPool,
    conversation_id: &Uuid,
    user_id: &Uuid,
) -> Result<()> {
    sqlx::query(
        r#"
        DELETE FROM conversation_participants
        WHERE conversation_id = $1 AND user_id = $2
        "#
    )
    .bind(conversation_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("移除参与者失败: {}", e))?;

    Ok(())
}

/// 批量添加参与者
pub async fn add_participants(
    pool: &PgPool,
    conversation_id: &Uuid,
    user_ids: &[Uuid],
) -> Result<()> {
    let now = chrono::Utc::now();

    for user_id in user_ids {
        // 检查是否已经是参与者
        let exists = is_conversation_participant(pool, conversation_id, user_id).await?;
        if !exists {
            sqlx::query(
                r#"
                INSERT INTO conversation_participants (conversation_id, user_id, joined_at)
                VALUES ($1, $2, $3)
                "#
            )
            .bind(conversation_id)
            .bind(user_id)
            .bind(now)
            .execute(pool)
            .await
            .map_err(|e| anyhow::anyhow!("添加参与者失败: {}", e))?;
        }
    }

    Ok(())
}

/// 获取参与者数量
pub async fn get_participant_count(
    pool: &PgPool,
    conversation_id: &Uuid,
) -> Result<i64> {
    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM conversation_participants
        WHERE conversation_id = $1
        "#
    )
    .bind(conversation_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取参与者数量失败: {}", e))?;

    Ok(count)
}

/// 更新群公告
pub async fn update_group_announcement(
    pool: &PgPool,
    conversation_id: &Uuid,
    announcement: &str,
) -> Result<ConversationEntity> {
    let now = chrono::Utc::now();

    // 将公告存储在 conversations 表的 metadata 字段中
    let conversation = sqlx::query_as::<_, ConversationEntity>(
        r#"
        UPDATE conversations
        SET metadata = jsonb_set(
            COALESCE(metadata, '{}'),
            '{announcement}',
            $1::jsonb
        ),
        updated_at = $2
        WHERE id = $3
        RETURNING *
        "#
    )
    .bind(serde_json::json!(announcement))
    .bind(now)
    .bind(conversation_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("更新群公告失败: {}", e))?;

    Ok(conversation)
}

/// 获取群公告
pub async fn get_group_announcement(
    pool: &PgPool,
    conversation_id: &Uuid,
) -> Result<Option<String>> {
    let result = sqlx::query_scalar::<_, Option<serde_json::Value>>(
        r#"
        SELECT metadata->'announcement' FROM conversations
        WHERE id = $1
        "#
    )
    .bind(conversation_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取群公告失败: {}", e))?;

    match result {
        Some(value) => {
            if let Some(s) = value.as_str() {
                Ok(Some(s.to_string()))
            } else {
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

/// 检查用户是否是群主或管理员（通过 created_by 字段判断）
pub async fn is_group_owner(
    pool: &PgPool,
    conversation_id: &Uuid,
    user_id: &Uuid,
) -> Result<bool> {
    let conversation = get_conversation_by_id(pool, conversation_id).await?;
    
    match conversation {
        Some(conv) => Ok(conv.created_by == Some(*user_id)),
        None => Ok(false),
    }
}
