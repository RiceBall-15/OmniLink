//! 管理员用户管理数据库操作

use sqlx::PgPool;
use uuid::Uuid;

/// 用户管理查询结果
#[derive(Debug, sqlx::FromRow)]
pub struct AdminUserRow {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub status: Option<String>,
    pub online_status: Option<String>,
    pub last_active_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 获取用户列表（管理员 - 分页、搜索、筛选）
pub async fn get_users(
    pool: &PgPool,
    page: i64,
    limit: i64,
    search: Option<&str>,
    status: Option<&str>,
    sort_by: Option<&str>,
    sort_order: Option<&str>,
) -> Result<(Vec<AdminUserRow>, i64), String> {
    let offset = (page - 1) * limit;

    let mut where_clauses: Vec<String> = Vec::new();
    let mut bind_idx = 1;

    if search.is_some() {
        where_clauses.push(format!(
            "(username ILIKE ${} OR email ILIKE ${} OR nickname ILIKE ${})",
            bind_idx, bind_idx, bind_idx
        ));
        bind_idx += 1;
    }

    if status.is_some() {
        where_clauses.push(format!("COALESCE(status, 'active') = ${}", bind_idx));
        bind_idx += 1;
    }

    let where_str = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let sort_column = match sort_by.unwrap_or("created_at") {
        "username" => "username",
        "email" => "email",
        "last_active" => "last_active_at",
        "created_at" => "created_at",
        _ => "created_at",
    };

    let order = match sort_order.unwrap_or("desc") {
        "asc" => "ASC",
        _ => "DESC",
    };

    // Count total
    let count_query = format!("SELECT COUNT(*) FROM users {}", where_str);
    let mut count_sql = sqlx::query_scalar::<_, i64>(&count_query);
    if let Some(s) = search {
        let pattern = format!("%{}%", s);
        count_sql = count_sql.bind(pattern);
    }
    if let Some(st) = status {
        count_sql = count_sql.bind(st);
    }
    let total = count_sql
        .fetch_one(pool)
        .await
        .map_err(|e| format!("查询用户总数失败: {}", e))?;

    // Fetch users
    let data_query = format!(
        r#"SELECT id, username, email, nickname, avatar,
                  COALESCE(status, 'active') as status,
                  COALESCE(online_status, 'offline') as online_status,
                  last_active_at, created_at, updated_at
           FROM users {} ORDER BY {} {} LIMIT ${} OFFSET ${}"#,
        where_str, sort_column, order, bind_idx, bind_idx + 1
    );

    let mut data_sql = sqlx::query_as::<_, AdminUserRow>(&data_query);
    if let Some(s) = search {
        let pattern = format!("%{}%", s);
        data_sql = data_sql.bind(pattern);
    }
    if let Some(st) = status {
        data_sql = data_sql.bind(st);
    }
    data_sql = data_sql.bind(limit).bind(offset);

    let users = data_sql
        .fetch_all(pool)
        .await
        .map_err(|e| format!("查询用户列表失败: {}", e))?;

    Ok((users, total))
}

/// 更新用户状态（封禁/解封）
pub async fn update_user_status(
    pool: &PgPool,
    user_id: &Uuid,
    status: &str,
) -> Result<bool, String> {
    let result = sqlx::query(
        "UPDATE users SET status = $1, updated_at = NOW() WHERE id = $2"
    )
    .bind(status)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| format!("更新用户状态失败: {}", e))?;

    Ok(result.rows_affected() > 0)
}

/// 获取用户详情（管理员视图）
pub async fn get_user_detail(
    pool: &PgPool,
    user_id: &Uuid,
) -> Result<Option<AdminUserRow>, String> {
    let user = sqlx::query_as::<_, AdminUserRow>(
        r#"SELECT id, username, email, nickname, avatar,
                  COALESCE(status, 'active') as status,
                  COALESCE(online_status, 'offline') as online_status,
                  last_active_at, created_at, updated_at
           FROM users WHERE id = $1"#
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("查询用户详情失败: {}", e))?;

    Ok(user)
}

/// 获取用户消息统计（管理员用）
pub async fn get_user_message_stats(
    pool: &PgPool,
    user_id: &Uuid,
) -> Result<(i64, i64, i64, i64, i64), String> {
    let row = sqlx::query_as::<_, (i64, i64, i64, i64, i64)>(
        r#"SELECT
            COUNT(*) as total_messages,
            COUNT(*) FILTER (WHERE created_at >= NOW() - INTERVAL '1 day') as messages_today,
            COUNT(*) FILTER (WHERE created_at >= NOW() - INTERVAL '7 days') as messages_this_week,
            COUNT(*) FILTER (WHERE created_at >= NOW() - INTERVAL '30 days') as messages_this_month,
            COUNT(DISTINCT conversation_id) as active_conversations
           FROM messages WHERE sender_id = $1"#
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("查询用户消息统计失败: {}", e))?;

    Ok(row)
}

/// 获取用户高峰活动时段
pub async fn get_user_peak_hours(
    pool: &PgPool,
    user_id: &Uuid,
) -> Result<Vec<(i32, i64)>, String> {
    let rows = sqlx::query_as::<_, (i32, i64)>(
        r#"SELECT EXTRACT(HOUR FROM created_at)::integer as hour, COUNT(*) as cnt
           FROM messages WHERE sender_id = $1
           GROUP BY hour ORDER BY cnt DESC LIMIT 5"#
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("查询用户高峰时段失败: {}", e))?;

    Ok(rows)
}

/// 确保 users 表有 status 和 last_active_at 列
pub async fn ensure_user_columns(pool: &PgPool) -> Result<(), String> {
    sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS status VARCHAR(20) DEFAULT 'active'")
        .execute(pool)
        .await
        .map_err(|e| format!("添加 status 列失败: {}", e))?;

    sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS last_active_at TIMESTAMP WITH TIME ZONE")
        .execute(pool)
        .await
        .map_err(|e| format!("添加 last_active_at 列失败: {}", e))?;

    Ok(())
}
