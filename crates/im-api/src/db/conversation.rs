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
