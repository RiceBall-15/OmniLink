//! 数据保留策略数据库操作

use sqlx::PgPool;
use uuid::Uuid;

/// 创建保留策略
pub async fn create_policy(
    pool: &PgPool,
    name: &str,
    description: Option<&str>,
    retention_days: i32,
    target_table: &str,
) -> Result<Uuid, sqlx::Error> {
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO data_retention_policies (id, name, description, retention_days, target_table, is_enabled, created_at, updated_at)
         VALUES (gen_random_uuid(), $1, $2, $3, $4, true, NOW(), NOW())
         RETURNING id"
    )
    .bind(name)
    .bind(description)
    .bind(retention_days)
    .bind(target_table)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// 获取所有保留策略
pub async fn get_all_policies(pool: &PgPool) -> Result<Vec<(Uuid, String, Option<String>, i32, String, bool, Option<chrono::DateTime<chrono::Utc>>, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, name, description, retention_days, target_table, is_enabled, last_run_at, created_at, updated_at
         FROM data_retention_policies ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await
}

/// 获取单个策略
pub async fn get_policy(pool: &PgPool, policy_id: Uuid) -> Result<Option<(Uuid, String, Option<String>, i32, String, bool, Option<chrono::DateTime<chrono::Utc>>, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, name, description, retention_days, target_table, is_enabled, last_run_at, created_at, updated_at
         FROM data_retention_policies WHERE id = $1"
    )
    .bind(policy_id)
    .fetch_optional(pool)
    .await
}

/// 更新策略
pub async fn update_policy(
    pool: &PgPool,
    policy_id: Uuid,
    name: Option<&str>,
    description: Option<&str>,
    retention_days: Option<i32>,
    is_enabled: Option<bool>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE data_retention_policies SET
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            retention_days = COALESCE($4, retention_days),
            is_enabled = COALESCE($5, is_enabled),
            updated_at = NOW()
         WHERE id = $1"
    )
    .bind(policy_id)
    .bind(name)
    .bind(description)
    .bind(retention_days)
    .bind(is_enabled)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// 删除策略
pub async fn delete_policy(pool: &PgPool, policy_id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM data_retention_policies WHERE id = $1")
        .bind(policy_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// 更新策略最后运行时间
pub async fn update_last_run(pool: &PgPool, policy_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE data_retention_policies SET last_run_at = NOW() WHERE id = $1")
        .bind(policy_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 获取所有启用的策略
pub async fn get_enabled_policies(pool: &PgPool) -> Result<Vec<(Uuid, String, i32, String)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, name, retention_days, target_table
         FROM data_retention_policies WHERE is_enabled = true"
    )
    .fetch_all(pool)
    .await
}
