//! 联系人数据库操作模块

use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;
use anyhow::Result;
use crate::models::auth::{ContactEntity, UserEntity};

/// 添加联系人
pub async fn add_contact(
    pool: &PgPool,
    user_id: &Uuid,
    contact_id: &Uuid,
    nickname: Option<String>,
) -> Result<ContactEntity> {
    // 检查不能添加自己为联系人
    if user_id == contact_id {
        return Err(anyhow::anyhow!("不能添加自己为联系人"));
    }

    // 检查联系人是否存在
    let user_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)"
    )
    .bind(contact_id)
    .fetch_one(pool)
    .await?;

    if !user_exists {
        return Err(anyhow::anyhow!("用户不存在"));
    }

    // 检查是否已经是联系人
    let already_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM contacts WHERE user_id = $1 AND contact_id = $2)"
    )
    .bind(user_id)
    .bind(contact_id)
    .fetch_one(pool)
    .await?;

    if already_exists {
        return Err(anyhow::anyhow!("该用户已经是您的联系人"));
    }

    let now = Utc::now();
    let contact = sqlx::query_as::<_, ContactEntity>(
        r#"
        INSERT INTO contacts (id, user_id, contact_id, nickname, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, user_id, contact_id, nickname, created_at, updated_at
        "#
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(contact_id)
    .bind(&nickname)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(contact)
}

/// 删除联系人
pub async fn remove_contact(
    pool: &PgPool,
    user_id: &Uuid,
    contact_id: &Uuid,
) -> Result<bool> {
    let result = sqlx::query(
        "DELETE FROM contacts WHERE user_id = $1 AND contact_id = $2"
    )
    .bind(user_id)
    .bind(contact_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// 获取联系人列表
pub async fn get_contacts(
    pool: &PgPool,
    user_id: &Uuid,
    page: i64,
    limit: i64,
) -> Result<Vec<ContactEntity>> {
    let offset = (page - 1) * limit;

    let contacts = sqlx::query_as::<_, ContactEntity>(
        r#"
        SELECT id, user_id, contact_id, nickname, created_at, updated_at
        FROM contacts
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(contacts)
}

/// 获取联系人总数
pub async fn count_contacts(
    pool: &PgPool,
    user_id: &Uuid,
) -> Result<i64> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM contacts WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(count)
}

/// 更新联系人备注名
pub async fn update_contact_nickname(
    pool: &PgPool,
    user_id: &Uuid,
    contact_id: &Uuid,
    nickname: &str,
) -> Result<ContactEntity> {
    let contact = sqlx::query_as::<_, ContactEntity>(
        r#"
        UPDATE contacts 
        SET nickname = $1, updated_at = $2
        WHERE user_id = $3 AND contact_id = $4
        RETURNING id, user_id, contact_id, nickname, created_at, updated_at
        "#
    )
    .bind(nickname)
    .bind(Utc::now())
    .bind(user_id)
    .bind(contact_id)
    .fetch_one(pool)
    .await?;

    Ok(contact)
}

/// 检查是否是联系人
pub async fn is_contact(
    pool: &PgPool,
    user_id: &Uuid,
    contact_id: &Uuid,
) -> Result<bool> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM contacts WHERE user_id = $1 AND contact_id = $2)"
    )
    .bind(user_id)
    .bind(contact_id)
    .fetch_one(pool)
    .await?;

    Ok(exists)
}

/// 搜索用户（按用户名或昵称）- 使用原生SQL查询
pub async fn search_users(
    pool: &PgPool,
    keyword: &str,
    current_user_id: &Uuid,
    limit: i64,
) -> Result<Vec<(UserEntity, bool)>> {
    let pattern = format!("%{}%", keyword);
    
    let rows = sqlx::query_as::<_, UserEntity>(
        r#"
        SELECT id, username, email, password_hash, avatar, nickname, bio, status_message, created_at, updated_at
        FROM users
        WHERE (username ILIKE $1 OR nickname ILIKE $1) AND id != $2
        ORDER BY 
            CASE WHEN username ILIKE $1 THEN 0 ELSE 1 END,
            username
        LIMIT $3
        "#
    )
    .bind(&pattern)
    .bind(current_user_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    // 批量检查是否为联系人
    let mut results = Vec::new();
    for user in rows {
        let is_contact_flag = is_contact(pool, current_user_id, &user.id).await.unwrap_or(false);
        results.push((user, is_contact_flag));
    }

    Ok(results)
}

/// 根据联系人ID获取联系人记录
pub async fn get_contact_by_id(
    pool: &PgPool,
    user_id: &Uuid,
    contact_id: &Uuid,
) -> Result<Option<ContactEntity>> {
    let contact = sqlx::query_as::<_, ContactEntity>(
        r#"
        SELECT id, user_id, contact_id, nickname, created_at, updated_at
        FROM contacts
        WHERE user_id = $1 AND contact_id = $2
        "#
    )
    .bind(user_id)
    .bind(contact_id)
    .fetch_optional(pool)
    .await?;

    Ok(contact)
}
