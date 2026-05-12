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
