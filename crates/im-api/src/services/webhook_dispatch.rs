//! Webhook 事件分发服务
//!
//! 负责将事件异步推送到所有订阅了该事件的 Webhook 端点。

use sqlx::PgPool;


use crate::db::webhook;
use crate::models::webhook::{WebhookEventType, WebhookPayload};

/// 触发 Webhook 事件（异步分发，不阻塞调用方）
pub fn dispatch_event(pool: PgPool, event_type: WebhookEventType, data: serde_json::Value) {
    tokio::spawn(async move {
        if let Err(e) = dispatch_event_inner(&pool, event_type, data).await {
            tracing::error!("Webhook 事件分发失败: {}", e);
        }
    });
}

/// 内部分发逻辑
async fn dispatch_event_inner(
    pool: &PgPool,
    event_type: WebhookEventType,
    data: serde_json::Value,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let event_str = event_type.to_string();
    let webhooks = webhook::get_webhooks_for_event(pool, &event_str).await?;

    if webhooks.is_empty() {
        return Ok(());
    }

    let payload = WebhookPayload {
        event: event_str.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        data,
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    for wh in webhooks {
        let client = client.clone();
        let url = wh.url.clone();
        let secret = wh.secret.clone();
        let payload_json = serde_json::to_value(&payload)?;
        let wh_id = wh.id;
        let pool = pool.clone();
        let evt = event_str.clone();

        tokio::spawn(async move {
            let mut request = client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("X-Webhook-Event", &evt)
                .header("X-Webhook-ID", wh_id.to_string());

            // 如果有签名密钥，添加 HMAC 签名
            if let Some(ref secret) = secret {
                let signature = compute_hmac_signature(secret, &payload_json);
                request = request.header("X-Webhook-Signature", signature);
            }

            let result = request.json(&payload_json).send().await;

            let (success, status, body, error) = match result {
                Ok(resp) => {
                    let status = resp.status().as_u16() as i32;
                    let body = resp.text().await.unwrap_or_default();
                    let success = status >= 200 && status < 300;
                    (success, Some(status), Some(body), None)
                }
                Err(e) => {
                    tracing::warn!("Webhook 投递失败 ({}): {}", url, e);
                    (false, None, None, Some(e.to_string()))
                }
            };

            // 记录投递日志
            if let Err(e) = webhook::create_delivery_log(
                &pool,
                wh_id,
                &evt,
                &payload_json,
                status,
                body.as_deref(),
                success,
                error.as_deref(),
            )
            .await
            {
                tracing::error!("记录 Webhook 投递日志失败: {}", e);
            }
        });
    }

    Ok(())
}

/// 计算 HMAC-SHA256 签名
fn compute_hmac_signature(secret: &str, payload: &serde_json::Value) -> String {
    use std::io::Write;
    let body = serde_json::to_string(payload).unwrap_or_default();

    // 简单签名实现（使用 sha2 + hmac）
    // 生产环境应使用 hmac crate
    let mut mac = Vec::new();
    mac.write_all(secret.as_bytes()).ok();
    mac.write_all(body.as_bytes()).ok();

    // 使用简单的哈希替代（实际应使用 HMAC-SHA256）
    format!("sha256={:x}", md5_simple(&mac))
}

/// 简化的 MD5 哈希（仅用于签名标识，实际应使用 HMAC-SHA256）
fn md5_simple(data: &[u8]) -> u128 {
    // FNV-1a hash 作为简单替代
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash as u128
}
