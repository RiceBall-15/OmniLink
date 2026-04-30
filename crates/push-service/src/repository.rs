use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use super::models::*;

pub struct PushRepository {
    pool: PgPool,
}

impl PushRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 创建推送消息记录
    pub async fn create_push_message(&self, msg: PushMessage) -> Result<PushMessage> {
        let message = sqlx::query_as::<_, PushMessage>(
            r#"
            INSERT INTO push_messages (id, user_id, device_type, device_token, title, body, data,
                                     badge, sound, priority, ttl, created_at, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, 'pending')
            RETURNING *
            "#
        )
        .bind(msg.id)
        .bind(msg.user_id)
        .bind(&msg.device_type)
        .bind(&msg.device_token)
        .bind(&msg.title)
        .bind(&msg.body)
        .bind(&msg.data)
        .bind(msg.badge)
        .bind(&msg.sound)
        .bind(msg.priority)
        .bind(msg.ttl)
        .bind(msg.created_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(message)
    }

    /// 更新推送状态
    pub async fn update_push_status(
        &self,
        message_id: Uuid,
        status: &str,
        error: Option<&str>,
    ) -> Result<bool> {
        let now = chrono::Utc::now();

        let result = if status == "sent" {
            sqlx::query(
                r#"
                UPDATE push_messages
                SET status = $1, sent_at = $2
                WHERE id = $3
                "#
            )
            .bind(status)
            .bind(now)
            .bind(message_id)
            .execute(&self.pool)
            .await?
        } else {
            sqlx::query(
                r#"
                UPDATE push_messages
                SET status = $1, failed_at = $2, error = $3
                WHERE id = $4
                "#
            )
            .bind(status)
            .bind(now)
            .bind(error)
            .bind(message_id)
            .execute(&self.pool)
            .await?
        };

        Ok(result.rows_affected() > 0)
    }

    /// 获取推送消息
    pub async fn get_push_message(&self, message_id: Uuid) -> Result<Option<PushMessage>> {
        let message = sqlx::query_as::<_, PushMessage>(
            r#"
            SELECT * FROM push_messages WHERE id = $1
            "#
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(message)
    }

    /// 获取用户推送记录
    pub async fn get_user_push_messages(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PushMessage>> {
        let messages = sqlx::query_as::<_, PushMessage>(
            r#"
            SELECT * FROM push_messages
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(messages)
    }

    /// 创建推送模板
    pub async fn create_template(&self, template: PushTemplate) -> Result<PushTemplate> {
        let new_template = sqlx::query_as::<_, PushTemplate>(
            r#"
            INSERT INTO push_templates (id, name, title_template, body_template, data_template,
                                       sound, badge, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
            RETURNING *
            "#
        )
        .bind(template.id)
        .bind(&template.name)
        .bind(&template.title_template)
        .bind(&template.body_template)
        .bind(&template.data_template)
        .bind(&template.sound)
        .bind(template.badge)
        .fetch_one(&self.pool)
        .await?;

        Ok(new_template)
    }

    /// 获取推送模板
    pub async fn get_template(&self, name: &str) -> Result<Option<PushTemplate>> {
        let template = sqlx::query_as::<_, PushTemplate>(
            r#"
            SELECT * FROM push_templates WHERE name = $1
            "#
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(template)
    }

    /// 获取所有模板
    pub async fn list_templates(&self) -> Result<Vec<PushTemplate>> {
        let templates = sqlx::query_as::<_, PushTemplate>(
            r#"
            SELECT * FROM push_templates
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(templates)
    }

    /// 删除模板
    pub async fn delete_template(&self, name: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM push_templates WHERE name = $1
            "#
        )
        .bind(name)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 获取推送统计
    pub async fn get_push_stats(
        &self,
        start_date: Option<chrono::DateTime<chrono::Utc>>,
        end_date: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<PushStats> {
        // 总发送数
        let (total_sent, total_failed): (i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE status = 'sent') as total_sent,
                COUNT(*) FILTER (WHERE status = 'failed') as total_failed
            FROM push_messages
            WHERE ($1::timestamptz IS NULL OR created_at >= $1)
              AND ($2::timestamptz IS NULL OR created_at <= $2)
            "#
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_one(&self.pool)
        .await?;

        // 按设备类型统计
        let by_device_type = sqlx::query_as::<_, DeviceTypeStats>(
            r#"
            SELECT
                device_type,
                COUNT(*) FILTER (WHERE status = 'sent') as sent,
                COUNT(*) FILTER (WHERE status = 'failed') as failed
            FROM push_messages
            WHERE ($1::timestamptz IS NULL OR created_at >= $1)
              AND ($2::timestamptz IS NULL OR created_at <= $2)
            GROUP BY device_type
            "#
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await?;

        // 按日期统计
        let by_date = sqlx::query_as::<_, DateStats>(
            r#"
            SELECT
                DATE(created_at) as date,
                COUNT(*) FILTER (WHERE status = 'sent') as sent,
                COUNT(*) FILTER (WHERE status = 'failed') as failed
            FROM push_messages
            WHERE ($1::timestamptz IS NULL OR created_at >= $1)
              AND ($2::timestamptz IS NULL OR created_at <= $2)
            GROUP BY DATE(created_at)
            ORDER BY date DESC
            LIMIT 30
            "#
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await?;

        Ok(PushStats {
            total_sent,
            total_failed,
            by_device_type,
            by_date,
        })
    }

    /// 清理过期推送记录
    pub async fn cleanup_old_messages(&self, days: i64) -> Result<u64> {
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(days);

        let result = sqlx::query(
            r#"
            DELETE FROM push_messages
            WHERE created_at < $1
            "#
        )
        .bind(cutoff_date)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}