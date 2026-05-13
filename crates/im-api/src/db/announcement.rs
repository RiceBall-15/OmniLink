//! 系统公告数据库操作

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::announcement::{AnnouncementEntity, AnnouncementRead, AnnouncementWithReadStatus, CreateAnnouncementRequest};

/// 创建系统公告
pub async fn create_announcement(
    pool: &PgPool,
    req: CreateAnnouncementRequest,
    created_by: Uuid,
) -> Result<AnnouncementEntity> {
    let now = Utc::now();
    let type_str = req.type_.unwrap_or_else(|| "info".to_string());
    let priority = req.priority.unwrap_or(0);
    let expires_at = req
        .expires_at
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let entity = sqlx::query_as::<_, AnnouncementEntity>(
        r#"
        INSERT INTO system_announcements (title, content, type_, priority, created_by, is_active, expires_at, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, true, $6, $7, $7)
        RETURNING *
        "#,
    )
    .bind(&req.title)
    .bind(&req.content)
    .bind(&type_str)
    .bind(priority)
    .bind(created_by)
    .bind(expires_at)
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(entity)
}

/// 获取公告列表（管理员视角，包含所有状态）
pub async fn get_all_announcements(
    pool: &PgPool,
    page: i64,
    page_size: i64,
) -> Result<Vec<AnnouncementEntity>> {
    let offset = (page - 1) * page_size;

    let announcements = sqlx::query_as::<_, AnnouncementEntity>(
        r#"
        SELECT * FROM system_announcements
        ORDER BY priority DESC, created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(page_size)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(announcements)
}

/// 获取活跃公告列表（用户视角，未过期的）
/// 获取活跃公告列表（用户视角，未过期的）
pub async fn get_active_announcements(
    pool: &PgPool,
    user_id: Uuid,
    page: i64,
    page_size: i64,
) -> Result<Vec<AnnouncementWithReadStatus>> {
    let offset = (page - 1) * page_size;
    let now = Utc::now();

    let rows = sqlx::query_as::<_, AnnouncementWithReadStatus>(
        r#"
        SELECT sa.id, sa.title, sa.content, sa.type_, sa.priority, sa.created_by,
               sa.is_active, sa.expires_at, sa.created_at, sa.updated_at,
               CASE WHEN ar.id IS NOT NULL THEN true ELSE false END as is_read,
               ar.read_at
        FROM system_announcements sa
        LEFT JOIN announcement_reads ar ON ar.announcement_id = sa.id AND ar.user_id = $1
        WHERE sa.is_active = true AND (sa.expires_at IS NULL OR sa.expires_at > $2)
        ORDER BY sa.priority DESC, sa.created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(user_id)
    .bind(now)
    .bind(page_size)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// 获取单个公告
pub async fn get_announcement_by_id(
    pool: &PgPool,
    announcement_id: Uuid,
) -> Result<Option<AnnouncementEntity>> {
    let announcement = sqlx::query_as::<_, AnnouncementEntity>(
        r#"
        SELECT * FROM system_announcements WHERE id = $1
        "#,
    )
    .bind(announcement_id)
    .fetch_optional(pool)
    .await?;

    Ok(announcement)
}

/// 标记公告为已读
pub async fn mark_announcement_read(
    pool: &PgPool,
    announcement_id: Uuid,
    user_id: Uuid,
) -> Result<AnnouncementRead> {
    let now = Utc::now();

    let read_record = sqlx::query_as::<_, AnnouncementRead>(
        r#"
        INSERT INTO announcement_reads (announcement_id, user_id, read_at)
        VALUES ($1, $2, $3)
        ON CONFLICT (announcement_id, user_id) DO UPDATE SET read_at = $3
        RETURNING *
        "#,
    )
    .bind(announcement_id)
    .bind(user_id)
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(read_record)
}

/// 检查用户是否已读公告
pub async fn is_announcement_read(
    pool: &PgPool,
    announcement_id: Uuid,
    user_id: Uuid,
) -> Result<bool> {
    let count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM announcement_reads
        WHERE announcement_id = $1 AND user_id = $2
        "#,
    )
    .bind(announcement_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(count.0 > 0)
}

/// 更新公告
pub async fn update_announcement(
    pool: &PgPool,
    announcement_id: Uuid,
    title: Option<String>,
    content: Option<String>,
    type_: Option<String>,
    priority: Option<i32>,
    is_active: Option<bool>,
    expires_at: Option<Option<String>>,
) -> Result<AnnouncementEntity> {
    let now = Utc::now();

    // 先获取现有记录
    let existing = get_announcement_by_id(pool, announcement_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Announcement not found"))?;

    let new_title = title.unwrap_or(existing.title);
    let new_content = content.unwrap_or(existing.content);
    let new_type = type_.unwrap_or(existing.type_);
    let new_priority = priority.unwrap_or(existing.priority);
    let new_is_active = is_active.unwrap_or(existing.is_active);
    let new_expires_at = match expires_at {
        Some(Some(s)) => chrono::DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|dt| dt.with_timezone(&Utc)),
        Some(None) => None,
        None => existing.expires_at,
    };

    let updated = sqlx::query_as::<_, AnnouncementEntity>(
        r#"
        UPDATE system_announcements
        SET title = $1, content = $2, type_ = $3, priority = $4, is_active = $5, expires_at = $6, updated_at = $7
        WHERE id = $8
        RETURNING *
        "#,
    )
    .bind(&new_title)
    .bind(&new_content)
    .bind(&new_type)
    .bind(new_priority)
    .bind(new_is_active)
    .bind(new_expires_at)
    .bind(now)
    .bind(announcement_id)
    .fetch_one(pool)
    .await?;

    Ok(updated)
}

/// 删除公告
pub async fn delete_announcement(pool: &PgPool, announcement_id: Uuid) -> Result<bool> {
    let result = sqlx::query(
        r#"
        DELETE FROM system_announcements WHERE id = $1
        "#,
    )
    .bind(announcement_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// 获取用户未读公告数量
pub async fn get_unread_announcement_count(pool: &PgPool, user_id: Uuid) -> Result<i64> {
    let now = Utc::now();

    let count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM system_announcements sa
        WHERE sa.is_active = true
          AND (sa.expires_at IS NULL OR sa.expires_at > $1)
          AND NOT EXISTS (
              SELECT 1 FROM announcement_reads ar
              WHERE ar.announcement_id = sa.id AND ar.user_id = $2
          )
        "#,
    )
    .bind(now)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(count.0)
}

/// 清理过期公告（自动标记为非活跃）
pub async fn cleanup_expired_announcements(pool: &PgPool) -> Result<u64> {
    let now = Utc::now();

    let result = sqlx::query(
        r#"
        UPDATE system_announcements
        SET is_active = false, updated_at = $1
        WHERE is_active = true AND expires_at IS NOT NULL AND expires_at <= $1
        "#,
    )
    .bind(now)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
