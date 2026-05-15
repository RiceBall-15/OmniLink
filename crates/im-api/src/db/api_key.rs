use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::models::api_key::{ApiKeyEntity, CreateApiKeyRequest, generate_api_key, verify_api_key};

/// 创建 API Key
pub async fn create_api_key(
    pool: &PgPool,
    owner_id: Uuid,
    req: &CreateApiKeyRequest,
) -> Result<(ApiKeyEntity, String), sqlx::Error> {
    let (raw_key, key_prefix, key_hash) = generate_api_key();

    let permissions = req.permissions.clone().unwrap_or_else(|| "read".to_string());
    let expires_at: Option<DateTime<Utc>> = req.expires_at.as_ref().and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc))
    });

    let row = sqlx::query_as::<_, ApiKeyEntity>(
        r#"
        INSERT INTO api_keys (key_prefix, key_hash, name, permissions, rate_limit, owner_id, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, key_prefix, key_hash, name, permissions, rate_limit,
                  owner_id, is_active, last_used_at, expires_at, created_at, updated_at
        "#,
    )
    .bind(key_prefix)
    .bind(key_hash)
    .bind(&req.name)
    .bind(permissions)
    .bind(req.rate_limit)
    .bind(owner_id)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;

    Ok((row, raw_key))
}

/// 获取用户的所有 API Keys
pub async fn get_api_keys_by_owner(
    pool: &PgPool,
    owner_id: Uuid,
) -> Result<Vec<ApiKeyEntity>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ApiKeyEntity>(
        r#"
        SELECT id, key_prefix, key_hash, name, permissions,
               rate_limit, owner_id, is_active,
               last_used_at, expires_at, created_at, updated_at
        FROM api_keys
        WHERE owner_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// 通过 key_hash 查找 API Key
pub async fn find_api_key_by_hash(
    pool: &PgPool,
    key_hash: &str,
) -> Result<Option<ApiKeyEntity>, sqlx::Error> {
    let row = sqlx::query_as::<_, ApiKeyEntity>(
        r#"
        SELECT id, key_prefix, key_hash, name, permissions,
               rate_limit, owner_id, is_active,
               last_used_at, expires_at, created_at, updated_at
        FROM api_keys
        WHERE key_hash = $1 AND is_active = true
        "#,
    )
    .bind(key_hash)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// 验证 API Key 并返回实体
pub async fn validate_api_key(
    pool: &PgPool,
    raw_key: &str,
) -> Result<Option<ApiKeyEntity>, sqlx::Error> {
    // 从 raw key 计算 hash
    let (_, key_prefix, _) = generate_api_key_from_raw(raw_key);

    // 先通过 prefix 查找候选 key
    let candidates = sqlx::query_as::<_, ApiKeyEntity>(
        r#"
        SELECT id, key_prefix, key_hash, name, permissions,
               rate_limit, owner_id, is_active,
               last_used_at, expires_at, created_at, updated_at
        FROM api_keys
        WHERE key_prefix = $1 AND is_active = true
        "#,
    )
    .bind(key_prefix)
    .fetch_all(pool)
    .await?;

    // 验证 hash 匹配
    for candidate in candidates {
        if verify_api_key(raw_key, &candidate.key_hash) {
            // 检查是否过期
            if let Some(expires_at) = candidate.expires_at {
                if expires_at < Utc::now() {
                    return Ok(None); // 已过期
                }
            }
            return Ok(Some(candidate));
        }
    }

    Ok(None)
}

/// 更新 API Key 最后使用时间
pub async fn update_last_used(
    pool: &PgPool,
    key_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE api_keys SET last_used_at = NOW() WHERE id = $1",
    )
    .bind(key_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// 停用 API Key（软删除）
pub async fn deactivate_api_key(
    pool: &PgPool,
    key_id: Uuid,
    owner_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE api_keys SET is_active = false WHERE id = $1 AND owner_id = $2",
    )
    .bind(key_id)
    .bind(owner_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// 删除 API Key（硬删除）
pub async fn delete_api_key(
    pool: &PgPool,
    key_id: Uuid,
    owner_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM api_keys WHERE id = $1 AND owner_id = $2",
    )
    .bind(key_id)
    .bind(owner_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// 从原始 key 计算前缀和 hash（辅助函数）
fn generate_api_key_from_raw(raw_key: &str) -> (String, String, String) {
    let key_prefix = raw_key.chars().take(8).collect::<String>();
    let key_hash = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        raw_key.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    };
    (raw_key.to_string(), key_prefix, key_hash)
}
