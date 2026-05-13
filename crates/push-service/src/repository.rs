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
            "#,
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
                "#,
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
                "#,
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
            "#,
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
            "#,
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
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(template.id)
        .bind(&template.name)
        .bind(&template.title_template)
        .bind(&template.body_template)
        .bind(&template.data_template)
        .bind(&template.sound)
        .bind(template.badge)
        .bind(template.created_at)
        .bind(template.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(new_template)
    }

    /// 获取推送模板
    pub async fn get_template(&self, name: &str) -> Result<Option<PushTemplate>> {
        let template = sqlx::query_as::<_, PushTemplate>(
            r#"
            SELECT * FROM push_templates WHERE name = $1
            "#,
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
            "#,
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
            "#,
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
        // 总发送数和失败数
        let (total_sent, total_failed): (i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE status = 'sent') as total_sent,
                COUNT(*) FILTER (WHERE status = 'failed') as total_failed
            FROM push_messages
            WHERE ($1::timestamptz IS NULL OR created_at >= $1)
              AND ($2::timestamptz IS NULL OR created_at <= $2)
            "#,
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
            "#,
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
            "#,
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
            "#,
        )
        .bind(cutoff_date)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    // ==================== 设备管理 ====================

    /// 创建设备记录
    pub async fn create_device(&self, device: DeviceInfo) -> Result<DeviceInfo> {
        let new_device = sqlx::query_as::<_, DeviceInfo>(
            r#"
            INSERT INTO push_devices (id, user_id, device_type, device_token, device_name, app_version, os_version, is_active, last_active_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(device.id)
        .bind(device.user_id)
        .bind(&device.device_type)
        .bind(&device.device_token)
        .bind(&device.device_name)
        .bind(&device.app_version)
        .bind(&device.os_version)
        .bind(device.is_active)
        .bind(device.last_active_at)
        .bind(device.created_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(new_device)
    }

    /// 获取用户设备列表
    pub async fn get_user_devices(&self, user_id: Uuid) -> Result<Vec<DeviceInfo>> {
        let devices = sqlx::query_as::<_, DeviceInfo>(
            r#"
            SELECT * FROM push_devices
            WHERE user_id = $1
            ORDER BY last_active_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(devices)
    }

    /// 注销设备
    pub async fn delete_device(&self, device_id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM push_devices WHERE id = $1
            "#,
        )
        .bind(device_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 更新设备活跃时间
    pub async fn update_device_active(&self, device_id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE push_devices
            SET last_active_at = $1
            WHERE id = $2
            "#,
        )
        .bind(chrono::Utc::now())
        .bind(device_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ==================== 通知偏好 ====================

    /// 获取用户通知偏好
    pub async fn get_notification_preferences(&self, user_id: Uuid) -> Result<NotificationPreferences> {
        let prefs = sqlx::query_as::<_, NotificationPreferences>(
            r#"
            SELECT * FROM notification_preferences WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(prefs.unwrap_or_else(|| NotificationPreferences {
            user_id,
            enable_notifications: true,
            enable_message_notifications: true,
            enable_system_notifications: true,
            enable_promotional_notifications: true,
            enable_reminder_notifications: true,
            quiet_hours_start: None,
            quiet_hours_end: None,
            updated_at: chrono::Utc::now(),
        }))
    }

    /// 创建或更新通知偏好
    pub async fn upsert_notification_preferences(&self, prefs: NotificationPreferences) -> Result<NotificationPreferences> {
        let updated = sqlx::query_as::<_, NotificationPreferences>(
            r#"
            INSERT INTO notification_preferences (user_id, enable_notifications, enable_message_notifications,
                enable_system_notifications, enable_promotional_notifications, enable_reminder_notifications,
                quiet_hours_start, quiet_hours_end, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (user_id) DO UPDATE SET
                enable_notifications = EXCLUDED.enable_notifications,
                enable_message_notifications = EXCLUDED.enable_message_notifications,
                enable_system_notifications = EXCLUDED.enable_system_notifications,
                enable_promotional_notifications = EXCLUDED.enable_promotional_notifications,
                enable_reminder_notifications = EXCLUDED.enable_reminder_notifications,
                quiet_hours_start = EXCLUDED.quiet_hours_start,
                quiet_hours_end = EXCLUDED.quiet_hours_end,
                updated_at = EXCLUDED.updated_at
            RETURNING *
            "#,
        )
        .bind(prefs.user_id)
        .bind(prefs.enable_notifications)
        .bind(prefs.enable_message_notifications)
        .bind(prefs.enable_system_notifications)
        .bind(prefs.enable_promotional_notifications)
        .bind(prefs.enable_reminder_notifications)
        .bind(&prefs.quiet_hours_start)
        .bind(&prefs.quiet_hours_end)
        .bind(prefs.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(updated)
    }

    // ==================== 推送配置 ====================

    /// 获取所有推送配置
    pub async fn get_push_configs(&self) -> Result<Vec<PushConfigItem>> {
        let configs = sqlx::query_as::<_, PushConfigItem>(
            r#"
            SELECT * FROM push_configs
            ORDER BY config_key
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(configs)
    }

    /// 创建或更新推送配置
    pub async fn upsert_push_config(&self, config: PushConfigItem) -> Result<PushConfigItem> {
        let updated = sqlx::query_as::<_, PushConfigItem>(
            r#"
            INSERT INTO push_configs (id, config_key, config_value, description, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (config_key) DO UPDATE SET
                config_value = EXCLUDED.config_value,
                description = EXCLUDED.description,
                updated_at = EXCLUDED.updated_at
            RETURNING *
            "#,
        )
        .bind(config.id)
        .bind(&config.config_key)
        .bind(&config.config_value)
        .bind(&config.description)
        .bind(config.created_at)
        .bind(config.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(updated)
    }

    /// 删除推送配置
    pub async fn delete_push_config(&self, key: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM push_configs WHERE config_key = $1
            "#,
        )
        .bind(key)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ==================== 健康监控 ====================

    /// 获取推送健康状态
    pub async fn get_push_health(&self) -> Result<PushHealthStatus> {
        // 获取设备统计
        let (total_devices, active_devices): (i64, i64) = sqlx::query_as(
            r#"
            SELECT COUNT(*) as total, COUNT(*) FILTER (WHERE is_active = true) as active
            FROM push_devices
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        // 按设备类型统计
        let devices_by_type = sqlx::query_as::<_, DeviceTypeCount>(
            r#"
            SELECT device_type, COUNT(*) as count
            FROM push_devices
            GROUP BY device_type
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        // 获取最近失败数（24小时内）
        let recent_failures: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM push_messages
            WHERE status = 'failed' AND failed_at > NOW() - INTERVAL '24 hours'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        // 计算成功率
        let (total, failed): (i64, i64) = sqlx::query_as(
            r#"
            SELECT COUNT(*) as total, COUNT(*) FILTER (WHERE status = 'failed') as failed
            FROM push_messages
            WHERE created_at > NOW() - INTERVAL '24 hours'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        let success_rate = if total > 0 {
            ((total - failed) as f64 / total as f64) * 100.0
        } else {
            100.0
        };

        Ok(PushHealthStatus {
            total_devices,
            active_devices,
            devices_by_type,
            recent_failures,
            success_rate,
        })
    }
}
