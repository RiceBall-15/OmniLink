//! 聊天记录导出后台处理任务
//!
//! 定期检查并处理待导出的任务，将消息格式化为指定格式并保存到文件

use sqlx::PgPool;
use tracing::{info, warn, error};
use chrono::Utc;
use std::path::PathBuf;

use crate::db::chat_export::{get_pending_export_jobs, update_export_job_status};
use crate::db::message::get_all_messages_for_export;
use crate::models::chat_export::{ExportFormat, ExportStatus};

/// 导出文件存储目录
const EXPORT_DIR: &str = "/tmp/omnilink_exports";

/// 启动导出任务处理后台任务
///
/// 每 15 秒检查一次待处理的导出任务
pub fn start_export_worker(pool: PgPool) {
    tokio::spawn(async move {
        info!("聊天记录导出任务已启动（每15秒检查一次）");

        // 确保导出目录存在
        if let Err(e) = tokio::fs::create_dir_all(EXPORT_DIR).await {
            error!("创建导出目录失败: {}", e);
        }

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;

            if let Err(e) = process_pending_export_jobs(&pool).await {
                error!("处理导出任务出错: {}", e);
            }
        }
    });
}

/// 处理所有待处理的导出任务
async fn process_pending_export_jobs(pool: &PgPool) -> anyhow::Result<()> {
    let pending_jobs = get_pending_export_jobs(pool, 5).await?;

    if pending_jobs.is_empty() {
        return Ok(());
    }

    info!("发现 {} 个待处理的导出任务", pending_jobs.len());

    for job in &pending_jobs {
        // 标记为处理中
        if let Err(e) = update_export_job_status(
            pool,
            job.id,
            ExportStatus::Processing,
            None, None, None, None,
        ).await {
            error!("更新导出任务 {} 状态为处理中失败: {}", job.id, e);
            continue;
        }

        info!("开始处理导出任务 {} (会话: {}, 格式: {})", job.id, job.conversation_id, job.format.to_string());

        // 执行导出
        match export_conversation_messages(pool, &job.conversation_id, &job.format).await {
            Ok((file_path, file_size, message_count)) => {
                info!(
                    "导出任务 {} 完成: 文件={}, 大小={}字节, 消息数={}",
                    job.id, file_path, file_size, message_count
                );

                if let Err(e) = update_export_job_status(
                    pool,
                    job.id,
                    ExportStatus::Completed,
                    Some(&file_path),
                    Some(file_size),
                    Some(message_count),
                    None,
                ).await {
                    error!("更新导出任务 {} 为完成状态失败: {}", job.id, e);
                }
            }
            Err(e) => {
                let error_msg = format!("导出失败: {}", e);
                warn!("导出任务 {} {}", job.id, error_msg);

                if let Err(mark_err) = update_export_job_status(
                    pool,
                    job.id,
                    ExportStatus::Failed,
                    None, None, None,
                    Some(&error_msg),
                ).await {
                    error!("更新导出任务 {} 为失败状态出错: {}", job.id, mark_err);
                }
            }
        }
    }

    Ok(())
}

/// 导出会话消息为指定格式
async fn export_conversation_messages(
    pool: &PgPool,
    conversation_id: &uuid::Uuid,
    format: &ExportFormat,
) -> anyhow::Result<(String, i64, i32)> {
    // 获取所有消息
    let messages = get_all_messages_for_export(pool, conversation_id).await?;
    let message_count = messages.len() as i32;

    if message_count == 0 {
        anyhow::bail!("会话中没有消息可导出");
    }

    // 格式化消息
    let content = match format {
        ExportFormat::Json => format_as_json(&messages)?,
        ExportFormat::Csv => format_as_csv(&messages)?,
        ExportFormat::Txt => format_as_txt(&messages)?,
    };

    // 生成文件名
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let file_name = format!("export_{}_{}.{}", conversation_id, timestamp, format.file_extension());
    let file_path = PathBuf::from(EXPORT_DIR).join(&file_name);
    let file_path_str = file_path.to_string_lossy().to_string();

    // 写入文件
    tokio::fs::write(&file_path, &content).await
        .map_err(|e| anyhow::anyhow!("写入导出文件失败: {}", e))?;

    // 获取文件大小
    let file_size = tokio::fs::metadata(&file_path).await
        .map(|m| m.len() as i64)
        .unwrap_or(0);

    Ok((file_path_str, file_size, message_count))
}

/// 格式化为 JSON
fn format_as_json(messages: &[crate::models::message::MessageEntity]) -> anyhow::Result<String> {
    let export_data: Vec<serde_json::Value> = messages.iter().map(|m| {
        serde_json::json!({
            "id": m.id.to_string(),
            "conversation_id": m.conversation_id.to_string(),
            "sender_id": m.sender_id.to_string(),
            "content": m.content,
            "type": m.type_,
            "status": m.status,
            "reply_to": m.reply_to.map(|r| r.to_string()),
            "metadata": m.metadata,
            "created_at": m.created_at.to_rfc3339(),
            "updated_at": m.updated_at.to_rfc3339(),
            "read_at": m.read_at.map(|r| r.to_rfc3339()),
        })
    }).collect();

    let json = serde_json::json!({
        "export_time": Utc::now().to_rfc3339(),
        "message_count": messages.len(),
        "messages": export_data,
    });

    serde_json::to_string_pretty(&json)
        .map_err(|e| anyhow::anyhow!("JSON 序列化失败: {}", e))
}

/// 格式化为 CSV
fn format_as_csv(messages: &[crate::models::message::MessageEntity]) -> anyhow::Result<String> {
    let mut wtr = csv::Writer::from_writer(vec![]);

    // 写入表头
    wtr.write_record(&["id", "conversation_id", "sender_id", "content", "type", "status", "reply_to", "created_at", "updated_at", "read_at"])?;

    for m in messages {
        wtr.write_record(&[
            m.id.to_string(),
            m.conversation_id.to_string(),
            m.sender_id.to_string(),
            m.content.clone(),
            m.type_.clone(),
            m.status.clone(),
            m.reply_to.map(|r| r.to_string()).unwrap_or_default(),
            m.created_at.to_rfc3339(),
            m.updated_at.to_rfc3339(),
            m.read_at.map(|r| r.to_rfc3339()).unwrap_or_default(),
        ])?;
    }

    let data = wtr.into_inner()
        .map_err(|e| anyhow::anyhow!("CSV 写入失败: {}", e))?;

    String::from_utf8(data)
        .map_err(|e| anyhow::anyhow!("CSV 编码失败: {}", e))
}

/// 格式化为纯文本
fn format_as_txt(messages: &[crate::models::message::MessageEntity]) -> anyhow::Result<String> {
    let mut output = String::new();
    output.push_str(&format!("OmniLink 聊天记录导出\n"));
    output.push_str(&format!("导出时间: {}\n", Utc::now().format("%Y-%m-%d %H:%M:%S")));
    output.push_str(&format!("消息数量: {}\n", messages.len()));
    output.push_str("========================================\n\n");

    for m in messages {
        let time = m.created_at.format("%Y-%m-%d %H:%M:%S");
        let sender = &m.sender_id.to_string()[..8]; // 取前8位作为简短标识
        let msg_type = match m.type_.as_str() {
            "text" => "",
            "image" => "[图片]",
            "file" => "[文件]",
            "system" => "[系统]",
            _ => "[未知]",
        };

        output.push_str(&format!("[{}] 用户{}: {}{}\n", time, sender, msg_type, m.content));

        if let Some(reply_to) = m.reply_to {
            output.push_str(&format!("  ↳ 回复消息: {}\n", reply_to));
        }

        output.push('\n');
    }

    Ok(output)
}

/// 获取导出任务处理状态
pub async fn get_export_worker_status(pool: &PgPool) -> serde_json::Value {
    let pending_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM export_jobs WHERE status = 'pending'"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let processing_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM export_jobs WHERE status = 'processing'"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let completed_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM export_jobs WHERE status = 'completed'"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let failed_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM export_jobs WHERE status = 'failed'"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    serde_json::json!({
        "pending_count": pending_count,
        "processing_count": processing_count,
        "completed_count": completed_count,
        "failed_count": failed_count,
        "check_interval_seconds": 15,
        "export_dir": EXPORT_DIR,
    })
}
