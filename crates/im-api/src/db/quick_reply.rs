//! 快捷回复数据库操作

use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::quick_reply::{CreateQuickReplyRequest, QuickReplyEntity, UpdateQuickReplyRequest};

/// 创建快捷回复
pub async fn create_quick_reply(
    pool: &PgPool,
    user_id: Uuid,
    req: CreateQuickReplyRequest,
) -> Result<QuickReplyEntity> {
    let now = Utc::now();
    let category = req.category.unwrap_or_else(|| "general".to_string());
    let sort_order = req.sort_order.unwrap_or(0);

    let entity = sqlx::query_as::<_, QuickReplyEntity>(
        r#"
        INSERT INTO quick_replies (user_id, title, content, category, sort_order, is_global, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, false, $6, $6)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(&req.title)
    .bind(&req.content)
    .bind(&category)
    .bind(sort_order)
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(entity)
}

/// 获取用户的快捷回复列表（含全局模板）
pub async fn get_user_quick_replies(
    pool: &PgPool,
    user_id: Uuid,
    category: Option<&str>,
) -> Result<Vec<QuickReplyEntity>> {
    let entities = if let Some(cat) = category {
        sqlx::query_as::<_, QuickReplyEntity>(
            r#"
            SELECT * FROM quick_replies
            WHERE (user_id = $1 OR is_global = true)
              AND category = $2
            ORDER BY sort_order ASC, created_at DESC
            "#,
        )
        .bind(user_id)
        .bind(cat)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, QuickReplyEntity>(
            r#"
            SELECT * FROM quick_replies
            WHERE user_id = $1 OR is_global = true
            ORDER BY sort_order ASC, created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?
    };

    Ok(entities)
}

/// 获取单个快捷回复
pub async fn get_quick_reply_by_id(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<QuickReplyEntity>> {
    let entity = sqlx::query_as::<_, QuickReplyEntity>(
        r#"
        SELECT * FROM quick_replies WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(entity)
}

/// 更新快捷回复（仅限拥有者）
pub async fn update_quick_reply(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
    req: UpdateQuickReplyRequest,
) -> Result<Option<QuickReplyEntity>> {
    let now = Utc::now();

    // 先获取现有记录
    let existing = get_quick_reply_by_id(pool, id).await?;
    let existing = match existing {
        Some(e) => e,
        None => return Ok(None),
    };

    // 验证所有权（全局模板仅管理员可编辑）
    if existing.user_id != user_id {
        return Ok(None);
    }

    let title = req.title.unwrap_or(existing.title);
    let content = req.content.unwrap_or(existing.content);
    let category = req.category.unwrap_or(existing.category);
    let sort_order = req.sort_order.unwrap_or(existing.sort_order);

    let entity = sqlx::query_as::<_, QuickReplyEntity>(
        r#"
        UPDATE quick_replies
        SET title = $2, content = $3, category = $4, sort_order = $5, updated_at = $6
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&title)
    .bind(&content)
    .bind(&category)
    .bind(sort_order)
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(Some(entity))
}

/// 删除快捷回复（仅限拥有者）
pub async fn delete_quick_reply(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        DELETE FROM quick_replies WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// 创建全局快捷回复（管理员）
pub async fn create_global_quick_reply(
    pool: &PgPool,
    admin_id: Uuid,
    req: CreateQuickReplyRequest,
) -> Result<QuickReplyEntity> {
    let now = Utc::now();
    let category = req.category.unwrap_or_else(|| "general".to_string());
    let sort_order = req.sort_order.unwrap_or(0);

    let entity = sqlx::query_as::<_, QuickReplyEntity>(
        r#"
        INSERT INTO quick_replies (user_id, title, content, category, sort_order, is_global, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, true, $6, $6)
        RETURNING *
        "#,
    )
    .bind(admin_id)
    .bind(&req.title)
    .bind(&req.content)
    .bind(&category)
    .bind(sort_order)
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(entity)
}
