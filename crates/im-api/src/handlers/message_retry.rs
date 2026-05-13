//! 消息重试队列处理器模块
//!
//! 提供消息发送失败重试相关的 API 端点：
//! - `POST /api/im/messages/:id/retry` - 手动重试失败消息
//! - `GET /api/im/messages/failed` - 获取用户失败消息列表
//! - `GET /api/im/messages/:id/retry-status` - 获取消息重试状态

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;
use serde::Deserialize;
use sqlx::PgPool;

use crate::models::auth::ApiResponse;
use crate::db::message_retry::{
    manual_retry_message, get_user_failed_messages, get_retry_by_message_id,
};
use crate::middleware::auth::AuthUser;

/// 失败消息查询参数
#[derive(Debug, Deserialize)]
pub struct FailedMessageQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_page() -> i64 { 1 }
fn default_limit() -> i64 { 20 }

/// 手动重试失败消息
///
/// POST /api/im/messages/:id/retry
pub async fn retry_message_handler(
    State(pool): State<PgPool>,
    AuthUser { user_id, .. }: AuthUser,
    Path(message_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let msg_uuid = match message_id.parse::<Uuid>() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的消息 ID")),
            );
        }
    };

    let _user_uuid = match user_id.parse::<Uuid>() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户 ID")),
            );
        }
    };

    // 检查消息是否存在且属于当前用户
    let message = match sqlx::query_as::<_, crate::models::message::MessageEntity>(
        "SELECT * FROM messages WHERE id = $1 AND sender_id = $2"
    )
    .bind(&msg_uuid)
    .bind(&_user_uuid)
    .fetch_optional(&pool)
    .await
    {
        Ok(Some(msg)) => msg,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "消息不存在或无权限操作")),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("QUERY_FAILED", format!("查询消息失败: {}", e))),
            );
        }
    };

    // 检查消息状态是否为失败
    let status: crate::models::message::MessageStatus = 
        message.status.parse().unwrap_or(crate::models::message::MessageStatus::Sent);
    
    if status != crate::models::message::MessageStatus::Failed {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("NOT_FAILED", "只能重试发送失败的消息")),
        );
    }

    // 执行手动重试
    match manual_retry_message(&pool, &msg_uuid).await {
        Ok(retry_entry) => {
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "messageId": retry_entry.message_id.to_string(),
                    "retryCount": retry_entry.retry_count,
                    "maxRetries": retry_entry.max_retries,
                    "status": retry_entry.status,
                    "nextRetryAt": retry_entry.next_retry_at.to_rfc3339(),
                    "message": "消息已加入重试队列"
                }))),
            )
        }
        Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("RETRY_FAILED", format!("重试失败: {}", e))),
            )
        }
    }
}

/// 获取用户失败消息列表
///
/// GET /api/im/messages/failed
pub async fn get_failed_messages_handler(
    State(pool): State<PgPool>,
    AuthUser { user_id, .. }: AuthUser,
    Query(query): Query<FailedMessageQuery>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match user_id.parse::<Uuid>() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户 ID")),
            );
        }
    };

    let limit = query.limit.clamp(1, 100);
    let page = query.page.max(1);

    match get_user_failed_messages(&pool, &user_uuid, page, limit).await {
        Ok(entries) => {
            let failed_list: Vec<serde_json::Value> = entries
                .iter()
                .map(|entry| {
                    serde_json::json!({
                        "messageId": entry.message_id.to_string(),
                        "conversationId": entry.conversation_id.to_string(),
                        "retryCount": entry.retry_count,
                        "maxRetries": entry.max_retries,
                        "lastError": entry.last_error,
                        "status": entry.status,
                        "createdAt": entry.created_at.to_rfc3339(),
                        "updatedAt": entry.updated_at.to_rfc3339(),
                    })
                })
                .collect();

            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "failedMessages": failed_list,
                    "page": page,
                    "limit": limit,
                    "total": failed_list.len(),
                }))),
            )
        }
        Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("QUERY_FAILED", format!("查询失败: {}", e))),
            )
        }
    }
}

/// 获取消息重试状态
///
/// GET /api/im/messages/:id/retry-status
pub async fn get_retry_status_handler(
    State(pool): State<PgPool>,
    Path(message_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let msg_uuid = match message_id.parse::<Uuid>() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的消息 ID")),
            );
        }
    };

    match get_retry_by_message_id(&pool, &msg_uuid).await {
        Ok(Some(entry)) => {
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "messageId": entry.message_id.to_string(),
                    "retryCount": entry.retry_count,
                    "maxRetries": entry.max_retries,
                    "nextRetryAt": entry.next_retry_at.to_rfc3339(),
                    "lastError": entry.last_error,
                    "status": entry.status,
                    "createdAt": entry.created_at.to_rfc3339(),
                    "updatedAt": entry.updated_at.to_rfc3339(),
                }))),
            )
        }
        Ok(None) => {
            (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "未找到该消息的重试记录")),
            )
        }
        Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("QUERY_FAILED", format!("查询失败: {}", e))),
            )
        }
    }
}
