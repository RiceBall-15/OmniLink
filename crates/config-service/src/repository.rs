use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use super::models::*;

pub struct ConfigRepository {
    pool: PgPool,
}

impl ConfigRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 获取配置项
    pub async fn get_config(&self, key: &str) -> Result<Option<ConfigItem>> {
        let config = sqlx::query_as::<_, ConfigItem>(
            r#"
            SELECT * FROM config
            WHERE key = $1
            "#
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(config)
    }

    /// 创建或更新配置项
    pub async fn upsert_config(
        &self,
        key: &str,
        value: &str,
        updated_by: Option<Uuid>,
    ) -> Result<ConfigItem> {
        // 先尝试更新
        let result = sqlx::query_as::<_, ConfigItem>(
            r#"
            UPDATE config
            SET value = $2,
                version = version + 1,
                updated_at = NOW(),
                updated_by = $3
            WHERE key = $1
            RETURNING *
            "#
        )
        .bind(key)
        .bind(value)
        .bind(updated_by)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(config) = result {
            // 同时记录历史
            self._insert_config_history(key, value, config.version - 1, updated_by, None)
                .await?;
            Ok(config)
        } else {
            // 不存在则插入
            let config = sqlx::query_as::<_, ConfigItem>(
                r#"
                INSERT INTO config (key, value, version, created_at, updated_at, updated_by)
                VALUES ($1, $2, 1, NOW(), NOW(), $3)
                RETURNING *
                "#
            )
            .bind(key)
            .bind(value)
            .bind(updated_by)
            .fetch_one(&self.pool)
            .await?;

            Ok(config)
        }
    }

    /// 删除配置项
    pub async fn delete_config(&self, key: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM config WHERE key = $1
            "#
        )
        .bind(key)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 获取所有配置
    pub async fn list_configs(&self) -> Result<Vec<ConfigItem>> {
        let configs = sqlx::query_as::<_, ConfigItem>(
            r#"
            SELECT * FROM config
            ORDER BY key
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(configs)
    }

    /// 批量获取配置
    pub async fn batch_get_configs(&self, keys: &[String]) -> Result<Vec<ConfigItem>> {
        let configs = sqlx::query_as::<_, ConfigItem>(
            r#"
            SELECT * FROM config
            WHERE key = ANY($1)
            ORDER BY key
            "#
        )
        .bind(keys)
        .fetch_all(&self.pool)
        .await?;

        Ok(configs)
    }

    /// 获取配置历史
    pub async fn get_config_history(&self, key: &str, limit: i64) -> Result<Vec<ConfigHistory>> {
        let history = sqlx::query_as::<_, ConfigHistory>(
            r#"
            SELECT * FROM config_history
            WHERE key = $1
            ORDER BY version DESC
            LIMIT $2
            "#
        )
        .bind(key)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(history)
    }

    /// 恢复配置到指定版本
    pub async fn restore_config_version(
        &self,
        key: &str,
        version: i32,
        updated_by: Option<Uuid>,
    ) -> Result<Option<ConfigItem>> {
        // 获取历史版本
        let history = sqlx::query_as::<_, ConfigHistory>(
            r#"
            SELECT * FROM config_history
            WHERE key = $1 AND version = $2
            "#
        )
        .bind(key)
        .bind(version)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(h) = history {
            // 恢复到当前版本
            let config = self.upsert_config(key, &h.value, updated_by).await?;
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    /// 内部方法：插入配置历史
    async fn _insert_config_history(
        &self,
        key: &str,
        value: &str,
        version: i32,
        updated_by: Option<Uuid>,
        reason: Option<String>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO config_history (key, value, version, updated_by, change_reason)
            VALUES ($1, $2, $3, $4, $5)
            "#
        )
        .bind(key)
        .bind(value)
        .bind(version)
        .bind(updated_by)
        .bind(reason)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 添加配置订阅
    pub async fn add_subscription(&self, sub: &ConfigSubscription) -> Result<ConfigSubscription> {
        let subscription = sqlx::query_as::<_, ConfigSubscription>(
            r#"
            INSERT INTO config_subscription (id, key, subscriber, callback_url, created_at, last_notified_at)
            VALUES ($1, $2, $3, $4, NOW(), NULL)
            RETURNING *
            "#
        )
        .bind(sub.id)
        .bind(&sub.key)
        .bind(&sub.subscriber)
        .bind(&sub.callback_url)
        .fetch_one(&self.pool)
        .await?;

        Ok(subscription)
    }

    /// 获取配置订阅
    pub async fn get_subscriptions(&self, key: &str) -> Result<Vec<ConfigSubscription>> {
        let subs = sqlx::query_as::<_, ConfigSubscription>(
            r#"
            SELECT * FROM config_subscription
            WHERE key = $1
            ORDER BY created_at
            "#
        )
        .bind(key)
        .fetch_all(&self.pool)
        .await?;

        Ok(subs)
    }

    /// 删除订阅
    pub async fn remove_subscription(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM config_subscription WHERE id = $1
            "#
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}