use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::chat_export::{ExportFormat, ExportJobEntity, ExportStatus};

/// 创建导出任务
pub async fn create_export_job(
    pool: &PgPool,
    user_id: Uuid,
    conversation_id: Uuid,
    format: ExportFormat,
) -> Result<ExportJobEntity, sqlx::Error> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    let format_str = format.to_string();
    let status_str = ExportStatus::Pending.to_string();

    sqlx::query_as::<_, ExportJobEntity>(
        r#"
        INSERT INTO export_jobs (id, user_id, conversation_id, format, status, message_count, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, 0, $6, $7)
        RETURNING id, user_id, conversation_id, format, status, message_count, file_path, file_size, error_message, created_at, updated_at, completed_at
        "#
    )
    .bind(id)
    .bind(user_id)
    .bind(conversation_id)
    .bind(format_str)
    .bind(status_str)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
}

/// 获取导出任务
pub async fn get_export_job(
    pool: &PgPool,
    job_id: Uuid,
    user_id: Uuid,
) -> Result<Option<ExportJobEntity>, sqlx::Error> {
    sqlx::query_as::<_, ExportJobEntity>(
        r#"
        SELECT id, user_id, conversation_id, format, status, message_count, file_path, file_size, error_message, created_at, updated_at, completed_at
        FROM export_jobs
        WHERE id = $1 AND user_id = $2
        "#
    )
    .bind(job_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

/// 更新导出任务状态
pub async fn update_export_job_status(
    pool: &PgPool,
    job_id: Uuid,
    status: ExportStatus,
    file_path: Option<&str>,
    file_size: Option<i64>,
    message_count: Option<i32>,
    error_message: Option<&str>,
) -> Result<ExportJobEntity, sqlx::Error> {
    let now = Utc::now();
    let status_str = status.to_string();
    let completed_at = if status == ExportStatus::Completed || status == ExportStatus::Failed {
        Some(now)
    } else {
        None
    };

    sqlx::query_as::<_, ExportJobEntity>(
        r#"
        UPDATE export_jobs
        SET status = $2, file_path = COALESCE($3, file_path), file_size = COALESCE($4, file_size),
            message_count = COALESCE($5, message_count), error_message = COALESCE($6, error_message),
            updated_at = $7, completed_at = COALESCE($8, completed_at)
        WHERE id = $1
        RETURNING id, user_id, conversation_id, format, status, message_count, file_path, file_size, error_message, created_at, updated_at, completed_at
        "#
    )
    .bind(job_id)
    .bind(status_str)
    .bind(file_path)
    .bind(file_size)
    .bind(message_count)
    .bind(error_message)
    .bind(now)
    .bind(completed_at)
    .fetch_one(pool)
    .await
}

/// 获取用户的导出任务列表
pub async fn get_user_export_jobs(
    pool: &PgPool,
    user_id: Uuid,
    page: i64,
    page_size: i64,
) -> Result<Vec<ExportJobEntity>, sqlx::Error> {
    let offset = (page - 1) * page_size;

    sqlx::query_as::<_, ExportJobEntity>(
        r#"
        SELECT id, user_id, conversation_id, format, status, message_count, file_path, file_size, error_message, created_at, updated_at, completed_at
        FROM export_jobs
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#
    )
    .bind(user_id)
    .bind(page_size)
    .bind(offset)
    .fetch_all(pool)
    .await
}

/// 获取待处理的导出任务
pub async fn get_pending_export_jobs(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<ExportJobEntity>, sqlx::Error> {
    sqlx::query_as::<_, ExportJobEntity>(
        r#"
        SELECT id, user_id, conversation_id, format, status, message_count, file_path, file_size, error_message, created_at, updated_at, completed_at
        FROM export_jobs
        WHERE status = 'pending'
        ORDER BY created_at ASC
        LIMIT $1
        "#
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}
