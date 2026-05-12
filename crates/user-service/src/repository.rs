use common::models::User;
use common::{AppError, Result};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use chrono::Utc;
use crate::models::DeviceInfo;

/// 用户仓库
///
/// 负责用户数据的数据库操作
pub struct UserRepository {
    pool: Pool<Postgres>,
}

impl UserRepository {
    /// 创建新的用户仓库实例
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// 根据ID查找用户
    pub async fn find_by_id(&self, user_id: Uuid) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// 根据邮箱查找用户
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE email = $1"
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// 根据用户名查找用户
    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE username = $1"
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// 创建用户
    ///
    /// 创建新用户并插入到数据库
    ///
    /// # Arguments
    /// * `user_id` - 用户 ID（UUID）
    /// * `username` - 用户名
    /// * `email` - 邮箱
    /// * `password_hash` - 密码哈希（bcrypt 加密）
    /// * `avatar_url` - 头像 URL（可选）
    pub async fn create(
        &self,
        user_id: Uuid,
        username: String,
        email: String,
        password_hash: String,
        avatar_url: Option<String>,
    ) -> Result<User> {
        let now = Utc::now();

        sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, username, email, password_hash, avatar_url, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#
        )
        .bind(user_id)
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .bind(avatar_url)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    /// 更新用户资料
    pub async fn update_profile(
        &self,
        user_id: Uuid,
        username: Option<String>,
        email: Option<String>,
        avatar_url: Option<String>,
    ) -> Result<User> {
        let now = Utc::now();

        // 动态构建更新查询
        let mut query = String::from("UPDATE users SET updated_at = $1");
        let mut param_count = 1;

        if username.is_some() {
            param_count += 1;
            query.push_str(&format!(", username = ${}", param_count));
        }
        if email.is_some() {
            param_count += 1;
            query.push_str(&format!(", email = ${}", param_count));
        }
        if avatar_url.is_some() {
            param_count += 1;
            query.push_str(&format!(", avatar_url = ${}", param_count));
        }

        param_count += 1;
        query.push_str(&format!(" WHERE id = ${} RETURNING *", param_count));

        let mut query_builder = sqlx::query_as::<_, User>(&query);
        query_builder = query_builder.bind(now);

        if let Some(username) = username {
            query_builder = query_builder.bind(username);
        }
        if let Some(email) = email {
            query_builder = query_builder.bind(email);
        }
        if let Some(avatar_url) = avatar_url {
            query_builder = query_builder.bind(avatar_url);
        }
        query_builder = query_builder.bind(user_id);

        query_builder
            .fetch_one(&self.pool)
            .await
            .map_err(AppError::Database)
    }

    /// 更新密码
    pub async fn update_password(&self, user_id: Uuid, new_password_hash: String) -> Result<()> {
        sqlx::query(
            "UPDATE users SET password_hash = $1, updated_at = $2 WHERE id = $3"
        )
        .bind(new_password_hash)
        .bind(Utc::now())
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    /// 更新最后登录时间
    pub async fn update_last_login(&self, user_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE users SET last_login_at = $1 WHERE id = $2"
        )
        .bind(Utc::now())
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    /// 删除用户
    pub async fn delete(&self, user_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

        Ok(())
    }

    /// 检查邮箱是否已存在
    pub async fn email_exists(&self, email: &str) -> Result<bool> {
        let exists: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)"
        )
        .bind(email)
        .fetch_one(&self.pool)
        .await?;

        Ok(exists.0)
    }

    /// 检查用户名是否已存在
    pub async fn username_exists(&self, username: &str) -> Result<bool> {
        let exists: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)"
        )
        .bind(username)
        .fetch_one(&self.pool)
        .await?;

        Ok(exists.0)
    }
}

/// 设备仓库
///
/// 负责用户设备数据的数据库操作
pub struct DeviceRepository {
    pool: Pool<Postgres>,
}

impl DeviceRepository {
    /// 创建新的设备仓库实例
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// 注册设备
    pub async fn register_device(
        &self,
        user_id: Uuid,
        device_id: String,
        device_name: String,
        device_type: String,
        platform: String,
    ) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO user_devices (user_id, device_id, device_name, device_type, platform, created_at, last_active)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (user_id, device_id)
            DO UPDATE SET
                device_name = EXCLUDED.device_name,
                device_type = EXCLUDED.device_type,
                platform = EXCLUDED.platform,
                last_active = EXCLUDED.last_active
            "#
        )
        .bind(user_id)
        .bind(&device_id)
        .bind(device_name)
        .bind(device_type)
        .bind(platform)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    /// 获取用户的所有设备
    pub async fn get_user_devices(&self, user_id: Uuid) -> Result<Vec<DeviceInfo>> {
        let devices = sqlx::query_as::<_, DeviceInfo>(
            r#"
            SELECT
                device_id as id,
                user_id,
                device_type,
                device_name,
                platform,
                last_active as last_active_at,
                created_at
            FROM user_devices
            WHERE user_id = $1
            ORDER BY last_active DESC
            "#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(devices)
    }

    /// 更新设备最后活跃时间
    pub async fn update_last_active(&self, user_id: Uuid, device_id: String) -> Result<()> {
        sqlx::query(
            "UPDATE user_devices SET last_active = $1 WHERE user_id = $2 AND device_id = $3"
        )
        .bind(Utc::now())
        .bind(user_id)
        .bind(device_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    /// 删除设备
    pub async fn delete_device(&self, user_id: Uuid, device_id: String) -> Result<()> {
        sqlx::query(
            "DELETE FROM user_devices WHERE user_id = $1 AND device_id = $2"
        )
        .bind(user_id)
        .bind(device_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    /// 删除用户的所有设备
    pub async fn delete_all_devices(&self, user_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM user_devices WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

        Ok(())
    }
}
