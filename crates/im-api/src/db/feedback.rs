//! 用户反馈数据库操作

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::feedback::{CreateFeedbackRequest, UserFeedbackEntity};

/// 创建反馈
pub async fn create_feedback(
    pool: &PgPool,
    user_id: Uuid,
    req: CreateFeedbackRequest,
) -> Result<UserFeedbackEntity, sqlx::Error> {
    let feedback_type = req.feedback_type.to_lowercase();
    let priority = req.priority.unwrap_or_else(|| "medium".to_string());
    let now = chrono::Utc::now();

    let entity = sqlx::query_as::<_, UserFeedbackEntity>(
        r#"
        INSERT INTO user_feedbacks (id, user_id, feedback_type, content, contact_email, status, priority, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, 'pending', $6, $7, $7)
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(&feedback_type)
    .bind(&req.content)
    .bind(&req.contact_email)
    .bind(&priority)
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(entity)
}

/// 获取反馈详情
pub async fn get_feedback_by_id(
    pool: &PgPool,
    feedback_id: Uuid,
) -> Result<Option<UserFeedbackEntity>, sqlx::Error> {
    let entity = sqlx::query_as::<_, UserFeedbackEntity>(
        "SELECT * FROM user_feedbacks WHERE id = $1",
    )
    .bind(feedback_id)
    .fetch_optional(pool)
    .await?;

    Ok(entity)
}

/// 获取用户的反馈列表
pub async fn get_user_feedbacks(
    pool: &PgPool,
    user_id: Uuid,
    feedback_type: Option<&str>,
    status: Option<&str>,
    page: i64,
    page_size: i64,
) -> Result<Vec<UserFeedbackEntity>, sqlx::Error> {
    let offset = (page - 1) * page_size;
    let mut query = String::from("SELECT * FROM user_feedbacks WHERE user_id = $1");
    let mut bind_idx = 2;

    if feedback_type.is_some() {
        query.push_str(&format!(" AND feedback_type = ${}", bind_idx));
        bind_idx += 1;
    }
    if status.is_some() {
        query.push_str(&format!(" AND status = ${}", bind_idx));
        bind_idx += 1;
    }

    query.push_str(&format!(" ORDER BY created_at DESC LIMIT ${} OFFSET ${}", bind_idx, bind_idx + 1));

    let mut q = sqlx::query_as::<_, UserFeedbackEntity>(&query).bind(user_id);
    if let Some(ft) = feedback_type {
        q = q.bind(ft);
    }
    if let Some(st) = status {
        q = q.bind(st);
    }
    q = q.bind(page_size).bind(offset);

    q.fetch_all(pool).await
}

/// 获取所有反馈（管理员）
pub async fn get_all_feedbacks(
    pool: &PgPool,
    feedback_type: Option<&str>,
    status: Option<&str>,
    priority: Option<&str>,
    page: i64,
    page_size: i64,
) -> Result<Vec<UserFeedbackEntity>, sqlx::Error> {
    let offset = (page - 1) * page_size;
    let mut conditions = Vec::new();
    let mut bind_idx = 1;

    if feedback_type.is_some() {
        conditions.push(format!("feedback_type = ${}", bind_idx));
        bind_idx += 1;
    }
    if status.is_some() {
        conditions.push(format!("status = ${}", bind_idx));
        bind_idx += 1;
    }
    if priority.is_some() {
        conditions.push(format!("priority = ${}", bind_idx));
        bind_idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    let query = format!(
        "SELECT * FROM user_feedbacks{} ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
        where_clause, bind_idx, bind_idx + 1
    );

    let mut q = sqlx::query_as::<_, UserFeedbackEntity>(&query);
    if let Some(ft) = feedback_type {
        q = q.bind(ft);
    }
    if let Some(st) = status {
        q = q.bind(st);
    }
    if let Some(pr) = priority {
        q = q.bind(pr);
    }
    q = q.bind(page_size).bind(offset);

    q.fetch_all(pool).await
}

/// 更新反馈（管理员）
pub async fn update_feedback(
    pool: &PgPool,
    feedback_id: Uuid,
    status: Option<&str>,
    priority: Option<&str>,
    admin_reply: Option<&str>,
    replied_by: Option<Uuid>,
) -> Result<Option<UserFeedbackEntity>, sqlx::Error> {
    let now = chrono::Utc::now();
    let mut set_clauses = vec!["updated_at = $1".to_string()];
    let mut bind_idx = 2;

    if status.is_some() {
        set_clauses.push(format!("status = ${}", bind_idx));
        bind_idx += 1;
    }
    if priority.is_some() {
        set_clauses.push(format!("priority = ${}", bind_idx));
        bind_idx += 1;
    }
    if admin_reply.is_some() {
        set_clauses.push(format!("admin_reply = ${}", bind_idx));
        bind_idx += 1;
        set_clauses.push(format!("replied_by = ${}", bind_idx));
        bind_idx += 1;
        set_clauses.push(format!("replied_at = ${}", bind_idx));
        bind_idx += 1;
    }

    let query = format!(
        "UPDATE user_feedbacks SET {} WHERE id = ${} RETURNING *",
        set_clauses.join(", "),
        bind_idx
    );

    let mut q = sqlx::query_as::<_, UserFeedbackEntity>(&query).bind(now);
    if let Some(s) = status {
        q = q.bind(s);
    }
    if let Some(p) = priority {
        q = q.bind(p);
    }
    if let Some(ar) = admin_reply {
        q = q.bind(ar);
        q = q.bind(replied_by.unwrap());
        q = q.bind(now);
    }
    q = q.bind(feedback_id);

    q.fetch_optional(pool).await
}

/// 删除反馈
pub async fn delete_feedback(
    pool: &PgPool,
    feedback_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM user_feedbacks WHERE id = $1")
        .bind(feedback_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// 获取反馈统计
pub async fn get_feedback_stats(pool: &PgPool) -> Result<(i64, i64, i64, i64, i64, i64, i64, i64), sqlx::Error> {
    let row: (i64, i64, i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*) as total,
            COUNT(*) FILTER (WHERE status = 'pending') as pending,
            COUNT(*) FILTER (WHERE status = 'processing') as processing,
            COUNT(*) FILTER (WHERE status = 'resolved') as resolved,
            COUNT(*) FILTER (WHERE status = 'rejected') as rejected,
            COUNT(*) FILTER (WHERE feedback_type = 'bug') as bug,
            COUNT(*) FILTER (WHERE feedback_type = 'feature') as feature,
            COUNT(*) FILTER (WHERE feedback_type = 'other') as other
        FROM user_feedbacks
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok(row)
}
