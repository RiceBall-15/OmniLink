use std::collections::HashMap;
use common::models::User;
use common::{AppError, Result};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

/// 用户仓库
pub struct UserRepository {
    pool: Pool<Postgres>,
}

impl UserRepository {
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
        .await
        .map_err(AppError::Database)?;

        Ok(user)
    }

    /// 批量获取用户信息
    pub async fn find_by_ids(&self, user_ids: &[Uuid]) -> Result<Vec<User>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }

        let users = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE id = ANY($1)"
        )
        .bind(user_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(users)
    }

    /// 根据邮箱查找用户
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE email = $1"
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(user)
    }

    /// 根据用户名查找用户
    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE username = $1"
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(user)
    }

    /// 检查用户是否被屏蔽
    pub async fn is_user_blocked(&self, blocker_id: Uuid, blocked_id: Uuid) -> Result<bool> {
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM user_blocks WHERE blocker_id = $1 AND blocked_id = $2)"
        )
        .bind(blocker_id)
        .bind(blocked_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(result)
    }

    /// 获取用户屏蔽列表中的所有用户ID
    pub async fn get_blocked_user_ids(&self, user_id: Uuid) -> Result<Vec<Uuid>> {
        let rows = sqlx::query_scalar::<_, Uuid>(
            "SELECT blocked_id FROM user_blocks WHERE blocker_id = $1"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows)
    }

    /// 批量获取多个用户的屏蔽列表
    pub async fn get_blocked_user_ids_batch(&self, user_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<Uuid>>> {
        if user_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let rows = sqlx::query_as::<_, (Uuid, Uuid)>(
            "SELECT blocker_id, blocked_id FROM user_blocks WHERE blocker_id = ANY($1)"
        )
        .bind(user_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let mut result: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for (blocker_id, blocked_id) in rows {
            result.entry(blocker_id).or_default().push(blocked_id);
        }

        Ok(result)
    }

    /// 查询哪些用户屏蔽了指定用户（反向查询）
    ///
    /// 返回屏蔽了 blocked_id 的所有用户ID
    pub async fn get_blocked_by_user_ids(&self, blocked_id: Uuid) -> Result<Vec<Uuid>> {
        let rows = sqlx::query_scalar::<_, Uuid>(
            "SELECT blocker_id FROM user_blocks WHERE blocked_id = $1"
        )
        .bind(blocked_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows)
    }

    /// 搜索用户（按用户名或邮箱模糊匹配）
    pub async fn search(&self, query: &str, limit: i64, offset: i64) -> Result<Vec<User>> {
        let pattern = format!("%{}%", query);
        let users = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE username ILIKE $1 OR email ILIKE $1 ORDER BY username LIMIT $2 OFFSET $3"
        )
        .bind(&pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(users)
    }

    /// 获取用户总数
    pub async fn count(&self) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await
            .map_err(AppError::Database)?;
        Ok(count)
    }
}
