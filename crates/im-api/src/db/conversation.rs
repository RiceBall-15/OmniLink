use sqlx::PgPool;
use uuid::Uuid;
use anyhow::Result;
use chrono::Timelike;

use crate::models::conversation::{
    ConversationEntity, CreateConversationParams, ConversationType,
    ConversationNotificationPreference, UpdateNotificationPreferenceRequest,
    GlobalNotificationSettings, UpdateGlobalNotificationRequest,
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

        // 初始化每用户会话状态（未读计数 = 0）
        sqlx::query(
            r#"
            INSERT INTO conversation_user_state (conversation_id, user_id, last_read_at, unread_count, created_at, updated_at)
            VALUES ($1, $2, $3, 0, $3, $3)
            ON CONFLICT (conversation_id, user_id) DO NOTHING
            "#
        )
        .bind(id)
        .bind(participant_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| anyhow::anyhow!("初始化用户会话状态失败: {}", e))?;
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
pub async fn update_unread_count(pool: &PgPool, conversation_id: &Uuid, _user_id: &Uuid, count: i32) -> Result<()> {
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

    // 初始化每用户会话状态
    let _ = sqlx::query(
        r#"
        INSERT INTO conversation_user_state (conversation_id, user_id, last_read_at, unread_count, created_at, updated_at)
        VALUES ($1, $2, $3, 0, $3, $3)
        ON CONFLICT (conversation_id, user_id) DO NOTHING
        "#
    )
    .bind(conversation_id)
    .bind(user_id)
    .bind(now)
    .execute(pool)
    .await;

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

/// 切换会话归档状态
pub async fn toggle_archive_conversation(
    pool: &PgPool,
    conversation_id: &Uuid,
    is_archived: bool,
) -> Result<ConversationEntity> {
    let now = chrono::Utc::now();

    let conversation = sqlx::query_as::<_, ConversationEntity>(
        r#"
        UPDATE conversations
        SET is_archived = $1, updated_at = $2
        WHERE id = $3
        RETURNING *
        "#
    )
    .bind(is_archived)
    .bind(now)
    .bind(conversation_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("更新归档状态失败: {}", e))?;

    Ok(conversation)
}

/// 搜索用户的会话（按名称模糊匹配）
pub async fn search_conversations(
    pool: &PgPool,
    user_id: &Uuid,
    query: &str,
    include_archived: bool,
) -> Result<Vec<ConversationEntity>> {
    let search_pattern = format!("%{}%", query);

    let conversations = if include_archived {
        sqlx::query_as::<_, ConversationEntity>(
            r#"
            SELECT DISTINCT c.* FROM conversations c
            INNER JOIN conversation_participants cp ON c.id = cp.conversation_id
            WHERE cp.user_id = $1
            AND c.name ILIKE $2
            ORDER BY c.updated_at DESC
            "#
        )
        .bind(user_id)
        .bind(&search_pattern)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, ConversationEntity>(
            r#"
            SELECT DISTINCT c.* FROM conversations c
            INNER JOIN conversation_participants cp ON c.id = cp.conversation_id
            WHERE cp.user_id = $1
            AND c.name ILIKE $2
            AND c.is_archived = FALSE
            ORDER BY c.updated_at DESC
            "#
        )
        .bind(user_id)
        .bind(&search_pattern)
        .fetch_all(pool)
        .await
    };

    conversations.map_err(|e| anyhow::anyhow!("搜索会话失败: {}", e))
}

/// 获取用户的未读会话数量
pub async fn get_user_unread_count(
    pool: &PgPool,
    user_id: &Uuid,
) -> Result<i64> {
    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM conversation_user_state
        WHERE user_id = $1 AND unread_count > 0
        "#
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取未读会话数量失败: {}", e))?;

    Ok(count)
}

/// 获取用户的所有未读会话 ID 列表
pub async fn get_user_unread_conversation_ids(
    pool: &PgPool,
    user_id: &Uuid,
) -> Result<Vec<Uuid>> {
    let ids = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT conversation_id FROM conversation_user_state
        WHERE user_id = $1 AND unread_count > 0
        ORDER BY updated_at DESC
        "#
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取未读会话列表失败: {}", e))?;

    Ok(ids)
}

/// 批量获取用户在多个会话中的未读计数
pub async fn get_user_unread_counts_batch(
    pool: &PgPool,
    user_id: &Uuid,
    conversation_ids: &[Uuid],
) -> Result<HashMap<Uuid, i32>> {
    if conversation_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = sqlx::query_as::<_, (Uuid, i32)>(
        r#"
        SELECT conversation_id, unread_count
        FROM conversation_user_state
        WHERE user_id = $1 AND conversation_id = ANY($2)
        "#
    )
    .bind(user_id)
    .bind(conversation_ids)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("批量获取未读计数失败: {}", e))?;

    let mut map = HashMap::new();
    for (conv_id, count) in rows {
        map.insert(conv_id, count);
    }

    Ok(map)
}

/// 获取用户的会话列表（支持排序和标签过滤）
pub async fn get_conversations_by_user_sorted(
    pool: &PgPool,
    user_id: &Uuid,
    sort_by: &str,
    order: &str,
    tag_id: Option<&Uuid>,
    include_archived: bool,
) -> Result<Vec<ConversationEntity>> {
    // 验证排序字段防止 SQL 注入
    let valid_sort = match sort_by {
        "updated_at" | "created_at" | "name" | "unread_count" => sort_by,
        _ => "updated_at",
    };
    let valid_order = match order.to_uppercase().as_str() {
        "ASC" => "ASC",
        _ => "DESC",
    };

    let conversations = if let Some(tid) = tag_id {
        // 按标签过滤
        let query_str = format!(
            r#"
            SELECT DISTINCT c.* FROM conversations c
            INNER JOIN conversation_participants cp ON c.id = cp.conversation_id
            INNER JOIN conversation_tag_links ctl ON c.id = ctl.conversation_id
            WHERE cp.user_id = $1
            AND ctl.tag_id = $2
            {archived_filter}
            ORDER BY c.{sort} {ord}
            "#,
            sort = valid_sort,
            ord = valid_order,
            archived_filter = if include_archived { "" } else { "AND c.is_archived = FALSE" }
        );
        sqlx::query_as::<_, ConversationEntity>(&query_str)
            .bind(user_id)
            .bind(tid)
            .fetch_all(pool)
            .await
    } else {
        let query_str = format!(
            r#"
            SELECT DISTINCT c.* FROM conversations c
            INNER JOIN conversation_participants cp ON c.id = cp.conversation_id
            WHERE cp.user_id = $1
            {archived_filter}
            ORDER BY c.is_pinned DESC, c.{sort} {ord}
            "#,
            sort = valid_sort,
            ord = valid_order,
            archived_filter = if include_archived { "" } else { "AND c.is_archived = FALSE" }
        );
        sqlx::query_as::<_, ConversationEntity>(&query_str)
            .bind(user_id)
            .fetch_all(pool)
            .await
    };

    conversations.map_err(|e| anyhow::anyhow!("获取会话列表失败: {}", e))
}

// ==================== 标签相关操作 ====================

/// 创建标签
pub async fn create_tag(
    pool: &PgPool,
    user_id: &Uuid,
    name: &str,
    color: Option<&str>,
) -> Result<ConversationTag> {
    let id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let tag = sqlx::query_as::<_, ConversationTag>(
        r#"
        INSERT INTO conversation_tags (id, user_id, name, color, created_at)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#
    )
    .bind(id)
    .bind(user_id)
    .bind(name)
    .bind(color)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("创建标签失败: {}", e))?;

    Ok(tag)
}

/// 获取用户的所有标签
pub async fn get_user_tags(pool: &PgPool, user_id: &Uuid) -> Result<Vec<ConversationTag>> {
    let tags = sqlx::query_as::<_, ConversationTag>(
        r#"
        SELECT * FROM conversation_tags
        WHERE user_id = $1
        ORDER BY name ASC
        "#
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取标签列表失败: {}", e))?;

    Ok(tags)
}

/// 删除标签
pub async fn delete_tag(pool: &PgPool, user_id: &Uuid, tag_id: &Uuid) -> Result<()> {
    // 先删除关联
    sqlx::query(
        r#"
        DELETE FROM conversation_tag_links
        WHERE tag_id = $1
        "#
    )
    .bind(tag_id)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("删除标签关联失败: {}", e))?;

    // 删除标签本身（确保只删除自己的标签）
    let result = sqlx::query(
        r#"
        DELETE FROM conversation_tags
        WHERE id = $1 AND user_id = $2
        "#
    )
    .bind(tag_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("删除标签失败: {}", e))?;

    if result.rows_affected() == 0 {
        return Err(anyhow::anyhow!("标签不存在或无权限删除"));
    }

    Ok(())
}

/// 给会话添加标签
pub async fn add_tag_to_conversation(
    pool: &PgPool,
    conversation_id: &Uuid,
    tag_id: &Uuid,
) -> Result<()> {
    let now = chrono::Utc::now();

    // 检查是否已存在
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM conversation_tag_links
            WHERE conversation_id = $1 AND tag_id = $2
        )
        "#
    )
    .bind(conversation_id)
    .bind(tag_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("检查标签关联失败: {}", e))?;

    if exists {
        return Ok(());
    }

    sqlx::query(
        r#"
        INSERT INTO conversation_tag_links (conversation_id, tag_id, created_at)
        VALUES ($1, $2, $3)
        "#
    )
    .bind(conversation_id)
    .bind(tag_id)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("添加标签到会话失败: {}", e))?;

    Ok(())
}

/// 移除会话的标签
pub async fn remove_tag_from_conversation(
    pool: &PgPool,
    conversation_id: &Uuid,
    tag_id: &Uuid,
) -> Result<()> {
    sqlx::query(
        r#"
        DELETE FROM conversation_tag_links
        WHERE conversation_id = $1 AND tag_id = $2
        "#
    )
    .bind(conversation_id)
    .bind(tag_id)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("移除会话标签失败: {}", e))?;

    Ok(())
}

/// 获取会话的所有标签
pub async fn get_conversation_tags(
    pool: &PgPool,
    conversation_id: &Uuid,
) -> Result<Vec<ConversationTag>> {
    let tags = sqlx::query_as::<_, ConversationTag>(
        r#"
        SELECT ct.* FROM conversation_tags ct
        INNER JOIN conversation_tag_links ctl ON ct.id = ctl.tag_id
        WHERE ctl.conversation_id = $1
        ORDER BY ct.name ASC
        "#
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取会话标签失败: {}", e))?;

    Ok(tags)
}

use crate::models::conversation::ConversationTag;

use std::collections::HashMap;

/// 批量获取标签链接（内部辅助结构）
#[derive(Debug, sqlx::FromRow)]
struct TagLinkWithTag {
    conversation_id: Uuid,
    tag_id: Uuid,
    tag_name: String,
    tag_color: Option<String>,
    tag_user_id: Uuid,
    tag_created_at: chrono::DateTime<chrono::Utc>,
}

/// 批量获取多个会话的标签（避免 N+1 查询）
pub async fn get_conversation_tags_batch(
    pool: &PgPool,
    conversation_ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<ConversationTag>>> {
    if conversation_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let links = sqlx::query_as::<_, TagLinkWithTag>(
        r#"
        SELECT ctl.conversation_id,
               ct.id as tag_id,
               ct.name as tag_name,
               ct.color as tag_color,
               ct.user_id as tag_user_id,
               ct.created_at as tag_created_at
        FROM conversation_tags ct
        INNER JOIN conversation_tag_links ctl ON ct.id = ctl.tag_id
        WHERE ctl.conversation_id = ANY($1)
        ORDER BY ct.name ASC
        "#
    )
    .bind(conversation_ids)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("批量获取会话标签失败: {}", e))?;

    let mut map: HashMap<Uuid, Vec<ConversationTag>> = HashMap::new();
    for link in links {
        let tag = ConversationTag {
            id: link.tag_id,
            user_id: link.tag_user_id,
            name: link.tag_name,
            color: link.tag_color,
            created_at: link.tag_created_at,
        };
        map.entry(link.conversation_id).or_default().push(tag);
    }

    Ok(map)
}

// ===== 会话通知偏好设置 DB 函数 =====

/// 获取用户的会话通知偏好
pub async fn get_notification_preference(
    pool: &PgPool,
    user_id: &Uuid,
    conversation_id: &Uuid,
) -> Result<Option<ConversationNotificationPreference>> {
    let pref = sqlx::query_as::<_, ConversationNotificationPreference>(
        r#"
        SELECT * FROM conversation_notification_preferences
        WHERE user_id = $1 AND conversation_id = $2
        "#,
    )
    .bind(user_id)
    .bind(conversation_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取通知偏好失败: {}", e))?;

    Ok(pref)
}

/// 创建或更新会话通知偏好（UPSERT）
pub async fn upsert_notification_preference(
    pool: &PgPool,
    user_id: &Uuid,
    conversation_id: &Uuid,
    req: &UpdateNotificationPreferenceRequest,
) -> Result<ConversationNotificationPreference> {
    let now = chrono::Utc::now();

    // 先尝试获取现有记录
    let existing = get_notification_preference(pool, user_id, conversation_id).await?;

    if let Some(pref) = existing {
        // 更新现有记录
        let muted = req.muted.unwrap_or(pref.muted);
        let sound = req.sound.as_deref().unwrap_or(&pref.sound);
        let badge = req.badge.unwrap_or(pref.badge);
        let mention_only = req.mention_only.unwrap_or(pref.mention_only);

        let updated = sqlx::query_as::<_, ConversationNotificationPreference>(
            r#"
            UPDATE conversation_notification_preferences
            SET muted = $1, sound = $2, badge = $3, mention_only = $4, updated_at = $5
            WHERE user_id = $6 AND conversation_id = $7
            RETURNING *
            "#,
        )
        .bind(muted)
        .bind(sound)
        .bind(badge)
        .bind(mention_only)
        .bind(now)
        .bind(user_id)
        .bind(conversation_id)
        .fetch_one(pool)
        .await
        .map_err(|e| anyhow::anyhow!("更新通知偏好失败: {}", e))?;

        Ok(updated)
    } else {
        // 创建新记录
        let id = Uuid::new_v4();
        let muted = req.muted.unwrap_or(false);
        let sound = req.sound.as_deref().unwrap_or("default");
        let badge = req.badge.unwrap_or(true);
        let mention_only = req.mention_only.unwrap_or(false);

        let created = sqlx::query_as::<_, ConversationNotificationPreference>(
            r#"
            INSERT INTO conversation_notification_preferences
                (id, user_id, conversation_id, muted, sound, badge, mention_only, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(conversation_id)
        .bind(muted)
        .bind(sound)
        .bind(badge)
        .bind(mention_only)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| anyhow::anyhow!("创建通知偏好失败: {}", e))?;

        Ok(created)
    }
}

/// 删除会话通知偏好
pub async fn delete_notification_preference(
    pool: &PgPool,
    user_id: &Uuid,
    conversation_id: &Uuid,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        DELETE FROM conversation_notification_preferences
        WHERE user_id = $1 AND conversation_id = $2
        "#,
    )
    .bind(user_id)
    .bind(conversation_id)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("删除通知偏好失败: {}", e))?;

    Ok(result.rows_affected() > 0)
}

/// 检查会话是否对用户静音（考虑全局设置和会话偏好）
pub async fn is_conversation_muted_for_user(
    pool: &PgPool,
    user_id: &Uuid,
    conversation_id: &Uuid,
) -> Result<bool> {
    // 检查全局通知设置
    let global = get_global_notification_settings(pool, user_id).await?;
    if let Some(ref settings) = global {
        if !settings.enabled {
            return Ok(true); // 全局通知关闭，视为全部静音
        }
    }

    // 检查会话级通知偏好
    let pref = get_notification_preference(pool, user_id, conversation_id).await?;
    if let Some(pref) = pref {
        return Ok(pref.muted || pref.mention_only);
    }

    Ok(false) // 没有特殊设置，不静音
}

/// 检查是否在免打扰时段
pub async fn is_in_dnd_period(
    pool: &PgPool,
    user_id: &Uuid,
) -> Result<bool> {
    let global = get_global_notification_settings(pool, user_id).await?;
    if let Some(settings) = global {
        if let (Some(start), Some(end)) = (&settings.dnd_start, &settings.dnd_end) {
            let tz = settings.dnd_timezone.as_deref().unwrap_or("UTC");
            return Ok(check_dnd_time(start, end, tz));
        }
    }
    Ok(false)
}

/// 检查当前时间是否在免打扰时段内
fn check_dnd_time(start: &str, end: &str, _timezone: &str) -> bool {
    // 简化实现：使用 UTC 时间比较
    // 格式：HH:MM
    let now = chrono::Utc::now();
    let current_minutes = now.hour() * 60 + now.minute();

    let start_minutes = parse_time_to_minutes(start);
    let end_minutes = parse_time_to_minutes(end);

    if let (Some(s), Some(e)) = (start_minutes, end_minutes) {
        if s <= e {
            // 同一天内：如 22:00 - 23:59
            current_minutes >= s && current_minutes <= e
        } else {
            // 跨天：如 22:00 - 08:00
            current_minutes >= s || current_minutes <= e
        }
    } else {
        false
    }
}

/// 解析 HH:MM 格式时间为分钟数
fn parse_time_to_minutes(time: &str) -> Option<u32> {
    let parts: Vec<&str> = time.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let hours = parts[0].parse::<u32>().ok()?;
    let minutes = parts[1].parse::<u32>().ok()?;
    if hours > 23 || minutes > 59 {
        return None;
    }
    Some(hours * 60 + minutes)
}

// ===== 全局通知设置 DB 函数 =====

/// 获取用户的全局通知设置
pub async fn get_global_notification_settings(
    pool: &PgPool,
    user_id: &Uuid,
) -> Result<Option<GlobalNotificationSettings>> {
    let settings = sqlx::query_as::<_, GlobalNotificationSettings>(
        r#"
        SELECT * FROM global_notification_settings
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("获取全局通知设置失败: {}", e))?;

    Ok(settings)
}

/// 创建或更新全局通知设置（UPSERT）
pub async fn upsert_global_notification_settings(
    pool: &PgPool,
    user_id: &Uuid,
    req: &UpdateGlobalNotificationRequest,
) -> Result<GlobalNotificationSettings> {
    let now = chrono::Utc::now();

    // 先尝试获取现有记录
    let existing = get_global_notification_settings(pool, user_id).await?;

    if let Some(settings) = existing {
        // 更新现有记录
        let enabled = req.enabled.unwrap_or(settings.enabled);
        let sound = req.sound.as_deref().unwrap_or(&settings.sound);
        let badge = req.badge.unwrap_or(settings.badge);
        let preview = req.preview.unwrap_or(settings.preview);
        let dnd_start = req.dnd_start.as_deref().or(settings.dnd_start.as_deref());
        let dnd_end = req.dnd_end.as_deref().or(settings.dnd_end.as_deref());
        let dnd_timezone = req.dnd_timezone.as_deref().or(settings.dnd_timezone.as_deref());

        let updated = sqlx::query_as::<_, GlobalNotificationSettings>(
            r#"
            UPDATE global_notification_settings
            SET enabled = $1, sound = $2, badge = $3, preview = $4,
                dnd_start = $5, dnd_end = $6, dnd_timezone = $7, updated_at = $8
            WHERE user_id = $9
            RETURNING *
            "#,
        )
        .bind(enabled)
        .bind(sound)
        .bind(badge)
        .bind(preview)
        .bind(dnd_start)
        .bind(dnd_end)
        .bind(dnd_timezone)
        .bind(now)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| anyhow::anyhow!("更新全局通知设置失败: {}", e))?;

        Ok(updated)
    } else {
        // 创建新记录
        let id = Uuid::new_v4();
        let enabled = req.enabled.unwrap_or(true);
        let sound = req.sound.as_deref().unwrap_or("default");
        let badge = req.badge.unwrap_or(true);
        let preview = req.preview.unwrap_or(true);

        let created = sqlx::query_as::<_, GlobalNotificationSettings>(
            r#"
            INSERT INTO global_notification_settings
                (id, user_id, enabled, sound, badge, preview, dnd_start, dnd_end, dnd_timezone, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(enabled)
        .bind(sound)
        .bind(badge)
        .bind(preview)
        .bind(req.dnd_start.as_deref())
        .bind(req.dnd_end.as_deref())
        .bind(req.dnd_timezone.as_deref())
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| anyhow::anyhow!("创建全局通知设置失败: {}", e))?;

        Ok(created)
    }
}
