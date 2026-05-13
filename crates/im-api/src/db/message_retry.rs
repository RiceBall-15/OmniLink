//! 消息重试队列数据库操作模块
//!
//! 提供消息发送失败后的重试队列管理：
//! - 创建重试记录
//! - 查询待重试消息
//! - 更新重试状态
//! - 指数退避计算

use sqlx::PgPool;
use uuid::Uuid;
use chrono::{Utc, DateTime, Duration};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// 重试队列状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RetryStatus {
    Pending,
    Retrying,
    Succeeded,
    Failed,
}

impl std::fmt::Display for RetryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RetryStatus::Pending => write!(f, "pending"),
            RetryStatus::Retrying => write!(f, "retrying"),
            RetryStatus::Succeeded => write!(f, "succeeded"),
            RetryStatus::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for RetryStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(RetryStatus::Pending),
            "retrying" => Ok(RetryStatus::Retrying),
            "succeeded" => Ok(RetryStatus::Succeeded),
            "failed" => Ok(RetryStatus::Failed),
            _ => Err(format!("Unknown retry status: {}", s)),
        }
    }
}

/// 重试队列实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RetryQueueEntity {
    pub id: Uuid,
    pub message_id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub retry_count: i32,
    pub max_retries: i32,
    pub next_retry_at: DateTime<chrono::Utc>,
    pub last_error: Option<String>,
    pub status: String,
    pub created_at: DateTime<chrono::Utc>,
    pub updated_at: DateTime<chrono::Utc>,
}

/// 计算指数退避时间（秒）
/// 第1次重试：2秒
/// 第2次重试：4秒
/// 第3次重试：8秒
/// 最大60秒
fn calculate_backoff(retry_count: i32) -> i64 {
    let base = 2_i64;
    let backoff = base.pow(retry_count as u32 + 1); // 2^(count+1): 2, 4, 8
    backoff.min(60) // 最大60秒
}

/// 创建重试记录
pub async fn create_retry_entry(
    pool: &PgPool,
    message_id: &Uuid,
    conversation_id: &Uuid,
    sender_id: &Uuid,
    max_retries: i32,
) -> Result<RetryQueueEntity> {
    let now = Utc::now();
    let backoff_seconds = calculate_backoff(0);
    let next_retry_at = now + Duration::seconds(backoff_seconds);

    let entry = sqlx::query_as::<_, RetryQueueEntity>(
        r#"
        INSERT INTO message_retry_queue 
            (id, message_id, conversation_id, sender_id, retry_count, max_retries, next_retry_at, status, created_at, updated_at)
        VALUES ($1, $2, $3, $4, 0, $5, $6, 'pending', $7, $7)
        RETURNING id, message_id, conversation_id, sender_id, retry_count, max_retries, next_retry_at, last_error, status, created_at, updated_at
        "#
    )
    .bind(Uuid::new_v4())
    .bind(message_id)
    .bind(conversation_id)
    .bind(sender_id)
    .bind(max_retries)
    .bind(next_retry_at)
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(entry)
}

/// 获取待重试的消息列表
pub async fn get_pending_retries(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<RetryQueueEntity>> {
    let now = Utc::now();
    let entries = sqlx::query_as::<_, RetryQueueEntity>(
        r#"
        SELECT id, message_id, conversation_id, sender_id, retry_count, max_retries, 
               next_retry_at, last_error, status, created_at, updated_at
        FROM message_retry_queue
        WHERE status = 'pending' AND next_retry_at <= $1
        ORDER BY next_retry_at ASC
        LIMIT $2
        "#
    )
    .bind(now)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(entries)
}

/// 更新重试状态
pub async fn update_retry_status(
    pool: &PgPool,
    retry_id: &Uuid,
    status: RetryStatus,
    error_message: Option<String>,
) -> Result<()> {
    let now = Utc::now();
    sqlx::query(
        r#"
        UPDATE message_retry_queue
        SET status = $1, last_error = $2, updated_at = $3
        WHERE id = $4
        "#
    )
    .bind(status.to_string())
    .bind(error_message)
    .bind(now)
    .bind(retry_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// 增加重试计数并计算下次重试时间
pub async fn increment_retry_count(
    pool: &PgPool,
    retry_id: &Uuid,
    error_message: Option<String>,
) -> Result<RetryQueueEntity> {
    let now = Utc::now();

    // 先获取当前记录
    let current = sqlx::query_as::<_, RetryQueueEntity>(
        "SELECT id, message_id, conversation_id, sender_id, retry_count, max_retries, next_retry_at, last_error, status, created_at, updated_at FROM message_retry_queue WHERE id = $1"
    )
    .bind(retry_id)
    .fetch_one(pool)
    .await?;

    let new_count = current.retry_count + 1;

    // 检查是否超过最大重试次数
    if new_count >= current.max_retries {
        // 更新为最终失败状态
        sqlx::query(
            r#"
            UPDATE message_retry_queue
            SET retry_count = $1, status = 'failed', last_error = $2, updated_at = $3
            WHERE id = $4
            "#
        )
        .bind(new_count)
        .bind(error_message.clone().unwrap_or_else(|| "达到最大重试次数".to_string()))
        .bind(now)
        .bind(retry_id)
        .execute(pool)
        .await?;

        // 更新消息状态为失败
        sqlx::query(
            "UPDATE messages SET status = 'failed', updated_at = $1 WHERE id = $2"
        )
        .bind(now)
        .bind(current.message_id)
        .execute(pool)
        .await?;

        return Ok(RetryQueueEntity {
            id: current.id,
            message_id: current.message_id,
            conversation_id: current.conversation_id,
            sender_id: current.sender_id,
            retry_count: new_count,
            max_retries: current.max_retries,
            next_retry_at: current.next_retry_at,
            last_error: error_message,
            status: "failed".to_string(),
            created_at: current.created_at,
            updated_at: now,
        });
    }

    // 计算下次重试时间
    let backoff_seconds = calculate_backoff(new_count);
    let next_retry_at = now + Duration::seconds(backoff_seconds);

    let updated = sqlx::query_as::<_, RetryQueueEntity>(
        r#"
        UPDATE message_retry_queue
        SET retry_count = $1, next_retry_at = $2, status = 'pending', last_error = $3, updated_at = $4
        WHERE id = $5
        RETURNING id, message_id, conversation_id, sender_id, retry_count, max_retries, next_retry_at, last_error, status, created_at, updated_at
        "#
    )
    .bind(new_count)
    .bind(next_retry_at)
    .bind(error_message)
    .bind(now)
    .bind(retry_id)
    .fetch_one(pool)
    .await?;

    Ok(updated)
}

/// 获取消息的重试记录
pub async fn get_retry_by_message_id(
    pool: &PgPool,
    message_id: &Uuid,
) -> Result<Option<RetryQueueEntity>> {
    let entry = sqlx::query_as::<_, RetryQueueEntity>(
        r#"
        SELECT id, message_id, conversation_id, sender_id, retry_count, max_retries, 
               next_retry_at, last_error, status, created_at, updated_at
        FROM message_retry_queue
        WHERE message_id = $1
        ORDER BY created_at DESC
        LIMIT 1
        "#
    )
    .bind(message_id)
    .fetch_optional(pool)
    .await?;

    Ok(entry)
}

/// 手动重试消息（重置重试队列状态）
pub async fn manual_retry_message(
    pool: &PgPool,
    message_id: &Uuid,
) -> Result<RetryQueueEntity> {
    let now = Utc::now();
    let backoff_seconds = calculate_backoff(0);
    let next_retry_at = now + Duration::seconds(backoff_seconds);

    // 更新或创建重试记录
    let entry = sqlx::query_as::<_, RetryQueueEntity>(
        r#"
        INSERT INTO message_retry_queue 
            (id, message_id, conversation_id, sender_id, retry_count, max_retries, next_retry_at, status, created_at, updated_at)
        SELECT 
            gen_random_uuid(), $1, conversation_id, sender_id, 0, 3, $2, 'pending', $3, $3
        FROM messages WHERE id = $1
        ON CONFLICT (message_id) DO UPDATE SET
            retry_count = 0,
            next_retry_at = $2,
            status = 'pending',
            last_error = NULL,
            updated_at = $3
        RETURNING id, message_id, conversation_id, sender_id, retry_count, max_retries, next_retry_at, last_error, status, created_at, updated_at
        "#
    )
    .bind(message_id)
    .bind(next_retry_at)
    .bind(now)
    .fetch_one(pool)
    .await?;

    // 更新消息状态为 sending
    sqlx::query(
        "UPDATE messages SET status = 'sending', updated_at = $1 WHERE id = $2"
    )
    .bind(now)
    .bind(message_id)
    .execute(pool)
    .await?;

    Ok(entry)
}

/// 获取用户的重试队列（失败消息）
pub async fn get_user_failed_messages(
    pool: &PgPool,
    user_id: &Uuid,
    page: i64,
    limit: i64,
) -> Result<Vec<RetryQueueEntity>> {
    let offset = (page - 1) * limit;
    let entries = sqlx::query_as::<_, RetryQueueEntity>(
        r#"
        SELECT id, message_id, conversation_id, sender_id, retry_count, max_retries, 
               next_retry_at, last_error, status, created_at, updated_at
        FROM message_retry_queue
        WHERE sender_id = $1 AND status = 'failed'
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_backoff() {
        // 重试0次: 2秒
        assert_eq!(calculate_backoff(0), 2);
        // 重试1次: 4秒
        assert_eq!(calculate_backoff(1), 4);
        // 重试2次: 8秒
        assert_eq!(calculate_backoff(2), 8);
        // 重试3次: 16秒
        assert_eq!(calculate_backoff(3), 16);
        // 重试4次: 32秒
        assert_eq!(calculate_backoff(4), 32);
        // 重试5次: 60秒（上限）
        assert_eq!(calculate_backoff(5), 60);
        // 重试10次: 60秒（上限）
        assert_eq!(calculate_backoff(10), 60);
    }

    #[test]
    fn test_retry_status_display() {
        assert_eq!(RetryStatus::Pending.to_string(), "pending");
        assert_eq!(RetryStatus::Retrying.to_string(), "retrying");
        assert_eq!(RetryStatus::Succeeded.to_string(), "succeeded");
        assert_eq!(RetryStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_retry_status_from_str() {
        assert_eq!("pending".parse::<RetryStatus>().unwrap(), RetryStatus::Pending);
        assert_eq!("retrying".parse::<RetryStatus>().unwrap(), RetryStatus::Retrying);
        assert_eq!("succeeded".parse::<RetryStatus>().unwrap(), RetryStatus::Succeeded);
        assert_eq!("failed".parse::<RetryStatus>().unwrap(), RetryStatus::Failed);
        assert!("unknown".parse::<RetryStatus>().is_err());
    }
}
