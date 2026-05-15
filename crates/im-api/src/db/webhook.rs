//! Webhook 数据库操作

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::webhook::{WebhookDeliveryEntity, WebhookEntity};

/// 创建 Webhook
pub async fn create_webhook(
    pool: &PgPool,
    user_id: Uuid,
    url: &str,
    secret: Option<&str>,
    events: &[String],
    description: Option<&str>,
) -> Result<WebhookEntity, sqlx::Error> {
    sqlx::query_as::<_, WebhookEntity>(
        "INSERT INTO webhooks (id, user_id, url, secret, events, description, is_active, created_at, updated_at)
         VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, true, NOW(), NOW())
         RETURNING id, user_id, url, secret, events, description, is_active, created_at, updated_at"
    )
    .bind(user_id)
    .bind(url)
    .bind(secret)
    .bind(events)
    .bind(description)
    .fetch_one(pool)
    .await
}

/// 获取用户的所有 Webhook
pub async fn get_user_webhooks(
    pool: &PgPool,
    user_id: Uuid,
    is_active: Option<bool>,
) -> Result<Vec<WebhookEntity>, sqlx::Error> {
    if let Some(active) = is_active {
        sqlx::query_as::<_, WebhookEntity>(
            "SELECT id, user_id, url, secret, events, description, is_active, created_at, updated_at 
             FROM webhooks WHERE user_id = $1 AND is_active = $2 ORDER BY created_at DESC"
        )
        .bind(user_id)
        .bind(active)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, WebhookEntity>(
            "SELECT id, user_id, url, secret, events, description, is_active, created_at, updated_at 
             FROM webhooks WHERE user_id = $1 ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }
}

/// 获取单个 Webhook
pub async fn get_webhook(
    pool: &PgPool,
    webhook_id: Uuid,
    user_id: Uuid,
) -> Result<Option<WebhookEntity>, sqlx::Error> {
    sqlx::query_as::<_, WebhookEntity>(
        "SELECT id, user_id, url, secret, events, description, is_active, created_at, updated_at 
         FROM webhooks WHERE id = $1 AND user_id = $2"
    )
    .bind(webhook_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

/// 更新 Webhook
pub async fn update_webhook(
    pool: &PgPool,
    webhook_id: Uuid,
    user_id: Uuid,
    url: Option<&str>,
    secret: Option<&str>,
    events: Option<&[String]>,
    _description: Option<&str>,
    is_active: Option<bool>,
) -> Result<Option<WebhookEntity>, sqlx::Error> {
    // 构建动态更新
    let existing = get_webhook(pool, webhook_id, user_id).await?;
    let existing = match existing {
        Some(e) => e,
        None => return Ok(None),
    };

    let new_url = url.unwrap_or(&existing.url);
    let new_events = events.unwrap_or(&existing.events);
    let new_active = is_active.unwrap_or(existing.is_active);

    let result = sqlx::query_as::<_, WebhookEntity>(
        "UPDATE webhooks SET 
            url = $3, 
            events = $4, 
            is_active = $5,
            updated_at = NOW()
         WHERE id = $1 AND user_id = $2
         RETURNING id, user_id, url, secret, events, description, is_active, created_at, updated_at"
    )
    .bind(webhook_id)
    .bind(user_id)
    .bind(new_url)
    .bind(new_events)
    .bind(new_active)
    .fetch_optional(pool)
    .await?;

    Ok(result)
}

/// 删除 Webhook
pub async fn delete_webhook(
    pool: &PgPool,
    webhook_id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM webhooks WHERE id = $1 AND user_id = $2")
        .bind(webhook_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// 获取所有订阅了特定事件的活跃 Webhook
pub async fn get_webhooks_for_event(
    pool: &PgPool,
    event_type: &str,
) -> Result<Vec<WebhookEntity>, sqlx::Error> {
    sqlx::query_as::<_, WebhookEntity>(
        "SELECT id, user_id, url, secret, events, description, is_active, created_at, updated_at 
         FROM webhooks WHERE is_active = true AND $1 = ANY(events)"
    )
    .bind(event_type)
    .fetch_all(pool)
    .await
}

/// 记录投递日志
pub async fn create_delivery_log(
    pool: &PgPool,
    webhook_id: Uuid,
    event_type: &str,
    payload: &serde_json::Value,
    response_status: Option<i32>,
    response_body: Option<&str>,
    success: bool,
    error_message: Option<&str>,
) -> Result<WebhookDeliveryEntity, sqlx::Error> {
    sqlx::query_as::<_, WebhookDeliveryEntity>(
        "INSERT INTO webhook_deliveries (id, webhook_id, event_type, payload, response_status, response_body, success, error_message, delivered_at)
         VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, NOW())
         RETURNING id, webhook_id, event_type, payload, response_status, response_body, success, error_message, delivered_at"
    )
    .bind(webhook_id)
    .bind(event_type)
    .bind(payload)
    .bind(response_status)
    .bind(response_body)
    .bind(success)
    .bind(error_message)
    .fetch_one(pool)
    .await
}

/// 获取 Webhook 的投递日志
pub async fn get_delivery_logs(
    pool: &PgPool,
    webhook_id: Uuid,
    event_type: Option<&str>,
    success: Option<bool>,
    page: i64,
    page_size: i64,
) -> Result<Vec<WebhookDeliveryEntity>, sqlx::Error> {
    let offset = (page - 1) * page_size;
    
    // 基础查询 + 动态条件
    match (event_type, success) {
        (Some(et), Some(s)) => {
            sqlx::query_as::<_, WebhookDeliveryEntity>(
                "SELECT id, webhook_id, event_type, payload, response_status, response_body, success, error_message, delivered_at 
                 FROM webhook_deliveries WHERE webhook_id = $1 AND event_type = $2 AND success = $3 
                 ORDER BY delivered_at DESC LIMIT $4 OFFSET $5"
            )
            .bind(webhook_id).bind(et).bind(s).bind(page_size).bind(offset)
            .fetch_all(pool).await
        }
        (Some(et), None) => {
            sqlx::query_as::<_, WebhookDeliveryEntity>(
                "SELECT id, webhook_id, event_type, payload, response_status, response_body, success, error_message, delivered_at 
                 FROM webhook_deliveries WHERE webhook_id = $1 AND event_type = $2 
                 ORDER BY delivered_at DESC LIMIT $3 OFFSET $4"
            )
            .bind(webhook_id).bind(et).bind(page_size).bind(offset)
            .fetch_all(pool).await
        }
        (None, Some(s)) => {
            sqlx::query_as::<_, WebhookDeliveryEntity>(
                "SELECT id, webhook_id, event_type, payload, response_status, response_body, success, error_message, delivered_at 
                 FROM webhook_deliveries WHERE webhook_id = $1 AND success = $2 
                 ORDER BY delivered_at DESC LIMIT $3 OFFSET $4"
            )
            .bind(webhook_id).bind(s).bind(page_size).bind(offset)
            .fetch_all(pool).await
        }
        (None, None) => {
            sqlx::query_as::<_, WebhookDeliveryEntity>(
                "SELECT id, webhook_id, event_type, payload, response_status, response_body, success, error_message, delivered_at 
                 FROM webhook_deliveries WHERE webhook_id = $1 
                 ORDER BY delivered_at DESC LIMIT $2 OFFSET $3"
            )
            .bind(webhook_id).bind(page_size).bind(offset)
            .fetch_all(pool).await
        }
    }
}
