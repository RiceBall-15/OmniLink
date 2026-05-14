//! 定时消息后台处理任务
//!
//! 定期检查并发送到期的定时消息

use sqlx::PgPool;
use tracing::{info, warn, error};
use chrono::Utc;

use crate::db::message::{
    get_pending_scheduled_messages,
    mark_scheduled_message_sent,
    mark_scheduled_message_failed,
    create_message,
    cleanup_expired_burn_messages,
};
use crate::models::message::{CreateMessageParams, MessageType};

/// 启动定时消息处理后台任务
///
/// 每 30 秒检查一次到期的定时消息，将其发送为真实消息
pub fn start_scheduled_message_processor(pool: PgPool) {
    tokio::spawn(async move {
        info!("定时消息处理任务已启动（每30秒检查一次）");

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;

            if let Err(e) = process_pending_scheduled_messages(&pool).await {
                error!("处理定时消息出错: {}", e);
            }
        }
    });
}

/// 处理所有到期的定时消息
async fn process_pending_scheduled_messages(pool: &PgPool) -> anyhow::Result<()> {
    // 获取到期的定时消息（最多一次处理 50 条）
    let pending_messages = get_pending_scheduled_messages(pool, 50).await?;

    if pending_messages.is_empty() {
        return Ok(());
    }

    info!("发现 {} 条到期的定时消息", pending_messages.len());

    for scheduled in &pending_messages {
        // 解析消息类型
        let message_type = match scheduled.message_type.as_str() {
            "text" => MessageType::Text,
            "image" => MessageType::Image,
            "file" => MessageType::File,
            "system" => MessageType::System,
            _ => MessageType::Text,
        };

        // 创建真实消息
        let params = CreateMessageParams {
            conversation_id: scheduled.conversation_id,
            sender_id: scheduled.sender_id,
            content: scheduled.content.clone(),
            type_: message_type,
            reply_to: scheduled.reply_to,
            metadata: scheduled.metadata.clone(),
            burn_after_reading: false,
            burn_after_seconds: None,
        };

        match create_message(pool, params).await {
            Ok(message) => {
                // 标记定时消息为已发送
                match mark_scheduled_message_sent(pool, &scheduled.id).await {
                    Ok(_) => {
                        info!(
                            "定时消息 {} 已成功发送为消息 {}",
                            scheduled.id, message.id
                        );
                    }
                    Err(e) => {
                        error!(
                            "标记定时消息 {} 为已发送失败: {}",
                            scheduled.id, e
                        );
                    }
                }
            }
            Err(e) => {
                // 标记定时消息为发送失败
                let error_msg = format!("发送失败: {}", e);
                warn!("定时消息 {} {}", scheduled.id, error_msg);

                if let Err(mark_err) = mark_scheduled_message_failed(
                    pool,
                    &scheduled.id,
                    &error_msg,
                )
                .await
                {
                    error!(
                        "标记定时消息 {} 为失败状态出错: {}",
                        scheduled.id, mark_err
                    );
                }
            }
        }
    }

    Ok(())
}

/// 获取定时消息处理状态
pub async fn get_scheduled_task_status(pool: &PgPool) -> serde_json::Value {
    // 查询各类定时消息的数量
    let pending_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM scheduled_messages WHERE status = 'pending'"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let sent_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM scheduled_messages WHERE status = 'sent'"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let failed_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM scheduled_messages WHERE status = 'failed'"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let next_pending: Option<chrono::DateTime<Utc>> = sqlx::query_scalar(
        "SELECT MIN(scheduled_at) FROM scheduled_messages WHERE status = 'pending'"
    )
    .fetch_one(pool)
    .await
    .ok()
    .flatten();

    serde_json::json!({
        "pending_count": pending_count,
        "sent_count": sent_count,
        "failed_count": failed_count,
        "next_scheduled_at": next_pending,
        "check_interval_seconds": 30,
    })
}

/// 启动阅后即焚消息清理后台任务
///
/// 每 10 秒检查一次过期的阅后即焚消息，自动删除并记录清理数量
pub fn start_burn_message_cleanup(pool: PgPool) {
    tokio::spawn(async move {
        info!("阅后即焚消息清理任务已启动（每10秒检查一次）");

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;

            match cleanup_expired_burn_messages(&pool).await {
                Ok(count) => {
                    if count > 0 {
                        info!("已清理 {} 条过期的阅后即焚消息", count);
                    }
                }
                Err(e) => {
                    error!("清理阅后即焚消息出错: {}", e);
                }
            }
        }
    });
}
