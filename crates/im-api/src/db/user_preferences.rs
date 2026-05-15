//! 用户偏好设置数据库操作

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::user_preferences::{UserPreferenceEntity, PreferenceCategorySummary};

/// 获取用户的所有偏好设置
pub async fn get_all_preferences(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<UserPreferenceEntity>, sqlx::Error> {
    sqlx::query_as::<_, UserPreferenceEntity>(
        "SELECT id, user_id, category, key, value, created_at, updated_at 
         FROM user_preferences 
         WHERE user_id = $1 
         ORDER BY category, key"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// 按类别获取用户偏好
pub async fn get_preferences_by_category(
    pool: &PgPool,
    user_id: Uuid,
    category: &str,
) -> Result<Vec<UserPreferenceEntity>, sqlx::Error> {
    sqlx::query_as::<_, UserPreferenceEntity>(
        "SELECT id, user_id, category, key, value, created_at, updated_at 
         FROM user_preferences 
         WHERE user_id = $1 AND category = $2 
         ORDER BY key"
    )
    .bind(user_id)
    .bind(category)
    .fetch_all(pool)
    .await
}

/// 获取单个偏好设置
pub async fn get_preference(
    pool: &PgPool,
    user_id: Uuid,
    category: &str,
    key: &str,
) -> Result<Option<UserPreferenceEntity>, sqlx::Error> {
    sqlx::query_as::<_, UserPreferenceEntity>(
        "SELECT id, user_id, category, key, value, created_at, updated_at 
         FROM user_preferences 
         WHERE user_id = $1 AND category = $2 AND key = $3"
    )
    .bind(user_id)
    .bind(category)
    .bind(key)
    .fetch_optional(pool)
    .await
}

/// 设置偏好（upsert：存在则更新，不存在则插入）
pub async fn set_preference(
    pool: &PgPool,
    user_id: Uuid,
    category: &str,
    key: &str,
    value: &serde_json::Value,
) -> Result<UserPreferenceEntity, sqlx::Error> {
    sqlx::query_as::<_, UserPreferenceEntity>(
        "INSERT INTO user_preferences (id, user_id, category, key, value, created_at, updated_at)
         VALUES (gen_random_uuid(), $1, $2, $3, $4, NOW(), NOW())
         ON CONFLICT (user_id, category, key) 
         DO UPDATE SET value = $4, updated_at = NOW()
         RETURNING id, user_id, category, key, value, created_at, updated_at"
    )
    .bind(user_id)
    .bind(category)
    .bind(key)
    .bind(value)
    .fetch_one(pool)
    .await
}

/// 删除偏好设置
pub async fn delete_preference(
    pool: &PgPool,
    user_id: Uuid,
    category: &str,
    key: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM user_preferences WHERE user_id = $1 AND category = $2 AND key = $3"
    )
    .bind(user_id)
    .bind(category)
    .bind(key)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// 删除用户某类别下所有偏好
pub async fn delete_category_preferences(
    pool: &PgPool,
    user_id: Uuid,
    category: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM user_preferences WHERE user_id = $1 AND category = $2"
    )
    .bind(user_id)
    .bind(category)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// 获取偏好类别汇总
pub async fn get_category_summary(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<PreferenceCategorySummary>, sqlx::Error> {
    let rows: Vec<(String, i64, Vec<String>)> = sqlx::query_as(
        "SELECT category, COUNT(*) as count, ARRAY_AGG(key ORDER BY key) as keys 
         FROM user_preferences 
         WHERE user_id = $1 
         GROUP BY category 
         ORDER BY category"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(category, count, keys)| {
        PreferenceCategorySummary {
            category,
            count,
            keys,
        }
    }).collect())
}
