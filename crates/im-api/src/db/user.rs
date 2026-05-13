use sqlx::{PgPool, Row};
use uuid::Uuid;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::Utc;
use crate::models::auth::{UserEntity, CreateUserParams};

/// 创建用户
pub async fn create_user(
    pool: &PgPool,
    params: CreateUserParams,
) -> Result<UserEntity, String> {
    let password_hash = hash(&params.password_hash, DEFAULT_COST)
        .map_err(|e| format!("密码加密失败: {}", e))?;

    let user_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO users (id, username, email, password_hash, avatar, nickname, bio, status_message, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id, username, email, password_hash, avatar, nickname, bio, status_message, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(&params.username)
    .bind(&params.email)
    .bind(&password_hash)
    .bind::<Option<String>>(None) // avatar
    .bind::<Option<String>>(None) // nickname
    .bind::<Option<String>>(None) // bio
    .bind::<Option<String>>(None) // status_message
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map(|row| UserEntity {
        id: row.get("id"),
        username: row.get("username"),
        email: row.get("email"),
        password_hash: row.get("password_hash"),
        avatar: row.get("avatar"),
        nickname: row.get("nickname"),
        bio: row.get("bio"),
        status_message: row.get("status_message"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
    .map_err(|e| {
        if e.to_string().contains("users_email_key") {
            "该邮箱已被注册".to_string()
        } else if e.to_string().contains("users_username_key") {
            "该用户名已被使用".to_string()
        } else {
            format!("创建用户失败: {}", e)
        }
    })
}

/// 根据邮箱查找用户
pub async fn find_user_by_email(
    pool: &PgPool,
    email: &str,
) -> Result<Option<UserEntity>, String> {
    sqlx::query(
        r#"
        SELECT id, username, email, password_hash, avatar, nickname, bio, status_message, created_at, updated_at
        FROM users
        WHERE email = $1
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("查询用户失败: {}", e))
    .map(|opt_row| {
        opt_row.map(|row| UserEntity {
            id: row.get("id"),
            username: row.get("username"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            avatar: row.get("avatar"),
            nickname: row.get("nickname"),
            bio: row.get("bio"),
            status_message: row.get("status_message"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    })
}

/// 根据 ID 查找用户
pub async fn find_user_by_id(
    pool: &PgPool,
    user_id: &str,
) -> Result<Option<UserEntity>, String> {
    let uuid = Uuid::parse_str(user_id)
        .map_err(|_| "无效的用户 ID 格式".to_string())?;

    sqlx::query(
        r#"
        SELECT id, username, email, password_hash, avatar, nickname, bio, status_message, created_at, updated_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(uuid)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("查询用户失败: {}", e))
    .map(|opt_row| {
        opt_row.map(|row| UserEntity {
            id: row.get("id"),
            username: row.get("username"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            avatar: row.get("avatar"),
            nickname: row.get("nickname"),
            bio: row.get("bio"),
            status_message: row.get("status_message"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    })
}

/// 更新用户信息
pub async fn update_user(
    pool: &PgPool,
    user_id: &str,
    username: Option<String>,
    email: Option<String>,
    avatar: Option<String>,
) -> Result<UserEntity, String> {
    let uuid = Uuid::parse_str(user_id)
        .map_err(|_| "无效的用户 ID 格式".to_string())?;

    let now = Utc::now();

    // 动态构建更新语句
    let mut query = String::from("UPDATE users SET updated_at = $1");
    let mut param_count = 2;

    if username.is_some() {
        query.push_str(&format!(", username = ${}", param_count));
        param_count += 1;
    }

    if email.is_some() {
        query.push_str(&format!(", email = ${}", param_count));
        param_count += 1;
    }

    if avatar.is_some() {
        query.push_str(&format!(", avatar = ${}", param_count));
        param_count += 1;
    }

    query.push_str(&format!(" WHERE id = ${} RETURNING id, username, email, password_hash, avatar, nickname, bio, status_message, created_at, updated_at", param_count));

    let mut sqlx_query = sqlx::query(&query).bind(now);

    if let Some(username) = username {
        sqlx_query = sqlx_query.bind(username);
    }

    if let Some(email) = email {
        sqlx_query = sqlx_query.bind(email);
    }

    if let Some(avatar) = avatar {
        sqlx_query = sqlx_query.bind(avatar);
    }

    sqlx_query = sqlx_query.bind(uuid);

    sqlx_query
        .fetch_one(pool)
        .await
        .map(|row| UserEntity {
            id: row.get("id"),
            username: row.get("username"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            avatar: row.get("avatar"),
            nickname: row.get("nickname"),
            bio: row.get("bio"),
            status_message: row.get("status_message"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .map_err(|e| {
            if e.to_string().contains("users_email_key") {
                "该邮箱已被注册".to_string()
            } else if e.to_string().contains("users_username_key") {
                "该用户名已被使用".to_string()
            } else {
                format!("更新用户失败: {}", e)
            }
        })
}

/// 验证密码
pub fn verify_password(password: &str, hash: &str) -> Result<bool, String> {
    verify(password, hash).map_err(|e| format!("密码验证失败: {}", e))
}

/// 更新用户资料（扩展字段：nickname, bio, status_message, avatar）
pub async fn update_user_profile(
    pool: &PgPool,
    user_id: &str,
    nickname: Option<String>,
    bio: Option<String>,
    status_message: Option<String>,
    avatar: Option<String>,
) -> Result<UserEntity, String> {
    let uuid = Uuid::parse_str(user_id)
        .map_err(|_| "无效的用户 ID 格式".to_string())?;

    let now = Utc::now();

    // 动态构建更新语句
    let mut query = String::from("UPDATE users SET updated_at = $1");
    let mut param_count = 2;

    if nickname.is_some() {
        query.push_str(&format!(", nickname = ${}", param_count));
        param_count += 1;
    }

    if bio.is_some() {
        query.push_str(&format!(", bio = ${}", param_count));
        param_count += 1;
    }

    if status_message.is_some() {
        query.push_str(&format!(", status_message = ${}", param_count));
        param_count += 1;
    }

    if avatar.is_some() {
        query.push_str(&format!(", avatar = ${}", param_count));
        param_count += 1;
    }

    query.push_str(&format!(" WHERE id = ${} RETURNING id, username, email, password_hash, avatar, nickname, bio, status_message, created_at, updated_at", param_count));

    let mut sqlx_query = sqlx::query(&query).bind(now);

    if let Some(nickname) = nickname {
        sqlx_query = sqlx_query.bind(nickname);
    }

    if let Some(bio) = bio {
        sqlx_query = sqlx_query.bind(bio);
    }

    if let Some(status_message) = status_message {
        sqlx_query = sqlx_query.bind(status_message);
    }

    if let Some(avatar) = avatar {
        sqlx_query = sqlx_query.bind(avatar);
    }

    sqlx_query = sqlx_query.bind(uuid);

    sqlx_query
        .fetch_one(pool)
        .await
        .map(|row| UserEntity {
            id: row.get("id"),
            username: row.get("username"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            avatar: row.get("avatar"),
            nickname: row.get("nickname"),
            bio: row.get("bio"),
            status_message: row.get("status_message"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .map_err(|e| format!("更新用户资料失败: {}", e))
}

// ==================== 用户屏蔽 ====================

/// 屏蔽用户
pub async fn block_user(
    pool: &PgPool,
    blocker_id: &str,
    blocked_id: &str,
) -> Result<(), String> {
    // 不能屏蔽自己
    if blocker_id == blocked_id {
        return Err("不能屏蔽自己".to_string());
    }

    // 检查是否已屏蔽
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM user_blocks WHERE blocker_id = $1 AND blocked_id = $2)"
    )
    .bind(blocker_id)
    .bind(blocked_id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("查询屏蔽状态失败: {}", e))?;

    if exists {
        return Err("已经屏蔽了该用户".to_string());
    }

    let block_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    sqlx::query(
        r#"INSERT INTO user_blocks (id, blocker_id, blocked_id, created_at)
        VALUES ($1, $2, $3, $4)"#
    )
    .bind(&block_id)
    .bind(blocker_id)
    .bind(blocked_id)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| format!("屏蔽用户失败: {}", e))?;

    Ok(())
}

/// 取消屏蔽用户
pub async fn unblock_user(
    pool: &PgPool,
    blocker_id: &str,
    blocked_id: &str,
) -> Result<(), String> {
    let result = sqlx::query(
        "DELETE FROM user_blocks WHERE blocker_id = $1 AND blocked_id = $2"
    )
    .bind(blocker_id)
    .bind(blocked_id)
    .execute(pool)
    .await
    .map_err(|e| format!("取消屏蔽失败: {}", e))?;

    if result.rows_affected() == 0 {
        return Err("未找到屏蔽记录".to_string());
    }

    Ok(())
}

/// 获取用户的屏蔽列表
pub async fn get_blocked_users(
    pool: &PgPool,
    blocker_id: &str,
) -> Result<Vec<crate::models::auth::BlockRecord>, String> {
    let rows = sqlx::query(
        r#"SELECT ub.id, ub.blocker_id, ub.blocked_id, ub.created_at,
                  u.username as blocked_username, u.avatar as blocked_avatar
        FROM user_blocks ub
        LEFT JOIN users u ON ub.blocked_id = u.id::text
        WHERE ub.blocker_id = $1
        ORDER BY ub.created_at DESC"#
    )
    .bind(blocker_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("获取屏蔽列表失败: {}", e))?;

    let blocks = rows.iter().map(|row| {
        let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
        crate::models::auth::BlockRecord {
            id: row.get("id"),
            blocker_id: row.get("blocker_id"),
            blocked_id: row.get("blocked_id"),
            blocked_username: row.get("blocked_username"),
            blocked_avatar: row.get("blocked_avatar"),
            created_at: created_at.to_rfc3339(),
        }
    }).collect();

    Ok(blocks)
}

/// 检查用户是否被屏蔽
pub async fn is_user_blocked(
    pool: &PgPool,
    user_id: &str,
    other_user_id: &str,
) -> Result<bool, String> {
    let blocked = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM user_blocks WHERE blocker_id = $1 AND blocked_id = $2)"
    )
    .bind(other_user_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("查询屏蔽状态失败: {}", e))?;

    Ok(blocked)
}

/// 获取用户已屏蔽的所有用户ID列表
pub async fn get_blocked_user_ids(
    pool: &PgPool,
    blocker_id: &str,
) -> Result<Vec<String>, String> {
    let rows = sqlx::query_scalar::<_, String>(
        "SELECT blocked_id FROM user_blocks WHERE blocker_id = $1"
    )
    .bind(blocker_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("获取屏蔽ID列表失败: {}", e))?;

    Ok(rows)
}

/// 用户在线状态详情
#[derive(Debug, serde::Serialize)]
pub struct UserStatusInfo {
    pub user_id: String,
    pub online_status: String,
    pub status_message: Option<String>,
    pub last_active_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// 更新用户在线状态
pub async fn update_user_online_status(
    pool: &PgPool,
    user_id: &str,
    online_status: &str,
    status_message: Option<&str>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(user_id)
        .map_err(|_| "无效的用户 ID 格式".to_string())?;

    let now = Utc::now();

    // 确保 online_status 列存在
    sqlx::query(
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS online_status VARCHAR(20) DEFAULT 'offline'"
    )
    .execute(pool)
    .await
    .map_err(|e| format!("添加 online_status 列失败: {}", e))?;

    sqlx::query(
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS last_active_at TIMESTAMP WITH TIME ZONE"
    )
    .execute(pool)
    .await
    .map_err(|e| format!("添加 last_active_at 列失败: {}", e))?;

    // 更新状态
    let mut query = String::from("UPDATE users SET online_status = $1, last_active_at = $2");
    let mut param_count = 3;

    if status_message.is_some() {
        query.push_str(&format!(", status_message = ${}", param_count));
        param_count += 1;
    }

    query.push_str(&format!(" WHERE id = ${}", param_count));

    let mut sqlx_query = sqlx::query(&query)
        .bind(online_status)
        .bind(now);

    if let Some(msg) = status_message {
        sqlx_query = sqlx_query.bind(msg);
    }

    sqlx_query = sqlx_query.bind(uuid);

    sqlx_query
        .execute(pool)
        .await
        .map_err(|e| format!("更新在线状态失败: {}", e))?;

    Ok(())
}

/// 获取用户在线状态详情
pub async fn get_user_status(
    pool: &PgPool,
    user_id: &str,
) -> Result<UserStatusInfo, String> {
    let uuid = Uuid::parse_str(user_id)
        .map_err(|_| "无效的用户 ID 格式".to_string())?;

    let row = sqlx::query(
        r#"SELECT id, COALESCE(online_status, 'offline') as online_status,
                  status_message, last_active_at
           FROM users WHERE id = $1"#
    )
    .bind(uuid)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("查询用户状态失败: {}", e))?
    .ok_or_else(|| "用户不存在".to_string())?;

    Ok(UserStatusInfo {
        user_id: row.get::<Uuid, _>("id").to_string(),
        online_status: row.get("online_status"),
        status_message: row.get("status_message"),
        last_active_at: row.get("last_active_at"),
    })
}
