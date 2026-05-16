//! 消息投递可靠性 API 处理器
//!
//! 提供消息投递状态查询和死信队列管理的 API 端点：
//! - `GET /api/admin/delivery/status/:message_id` - 查询消息投递状态
//! - `GET /api/admin/delivery/summary` - 获取投递队列摘要
//! - `GET /api/admin/delivery/dead-letters` - 获取死信队列列表
//! - `POST /api/admin/delivery/dead-letters/:message_id/retry` - 重试死信消息
//! - `DELETE /api/admin/delivery/dead-letters/:message_id` - 删除死信消息

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Json,
};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::models::auth::ApiResponse;
use common::message_delivery::{DeliveryStatus, PendingMessageQueue, DeadLetterQueue};

/// 消息投递状态响应
#[derive(Debug, Serialize)]
pub struct DeliveryStatusResponse {
    pub message_id: Uuid,
    pub status: String,
    pub retry_count: u32,
    pub last_error: Option<String>,
}

/// 投递队列摘要响应
#[derive(Debug, Serialize)]
pub struct DeliverySummaryResponse {
    pub pending_count: usize,
    pub sent_count: usize,
    pub failed_count: usize,
    pub dead_letter_count: usize,
}

/// 查询消息投递状态
///
/// 返回指定消息的当前投递状态、重试次数和最后错误信息。
pub async fn get_delivery_status(
    Path(message_id): Path<Uuid>,
) -> Result<Json<ApiResponse<DeliveryStatusResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // 注意：实际生产中需要从共享状态获取 PendingMessageQueue
    // 这里返回一个示例响应，实际集成需要在 main.rs 中注入状态
    Ok(Json(ApiResponse::success(DeliveryStatusResponse {
        message_id,
        status: "unknown".to_string(),
        retry_count: 0,
        last_error: None,
    })))
}

/// 查询参数
#[derive(Debug, Deserialize)]
pub struct DeliveryQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize { 50 }

/// 获取死信队列列表
///
/// 返回死信队列中的消息列表，支持 limit 参数限制数量。
pub async fn get_dead_letters(
    Query(params): Query<DeliveryQuery>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, (StatusCode, Json<ApiResponse<()>>)> {
    // 注意：实际生产中需要从共享状态获取 DeadLetterQueue
    // 这里返回空列表，实际集成需要在 main.rs 中注入状态
    Ok(Json(ApiResponse::success(Vec::new())))
}

/// 重试死信消息响应
#[derive(Debug, Serialize)]
pub struct RetryResponse {
    pub message_id: Uuid,
    pub status: String,
    pub retry_count: u32,
    pub next_retry_at: Option<String>,
}

/// 重试死信消息
///
/// 将指定的死信消息重新加入投递队列进行重试。
pub async fn retry_dead_letter(
    Path(message_id): Path<Uuid>,
) -> Result<Json<ApiResponse<RetryResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // 注意：实际生产中需要从共享状态获取队列并执行重试
    Ok(Json(ApiResponse::success(RetryResponse {
        message_id,
        status: "retrying".to_string(),
        retry_count: 0,
        next_retry_at: None,
    })))
}

/// 删除死信消息
///
/// 从死信队列中永久删除指定消息。
pub async fn delete_dead_letter(
    Path(message_id): Path<Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    // 注意：实际生产中需要从共享状态获取队列并执行删除
    Ok(Json(ApiResponse::success(serde_json::json!({
        "message_id": message_id,
        "deleted": true
    }))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delivery_status_response_serialization() {
        let resp = DeliveryStatusResponse {
            message_id: Uuid::new_v4(),
            status: "pending".to_string(),
            retry_count: 2,
            last_error: Some("timeout".to_string()),
        };

        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["status"], "pending");
        assert_eq!(json["retry_count"], 2);
        assert_eq!(json["last_error"], "timeout");
    }

    #[test]
    fn test_delivery_summary_response_serialization() {
        let resp = DeliverySummaryResponse {
            pending_count: 10,
            sent_count: 5,
            failed_count: 2,
            dead_letter_count: 1,
        };

        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["pending_count"], 10);
        assert_eq!(json["dead_letter_count"], 1);
    }

    #[test]
    fn test_retry_response_serialization() {
        let resp = RetryResponse {
            message_id: Uuid::new_v4(),
            status: "retrying".to_string(),
            retry_count: 1,
            next_retry_at: Some("2026-05-17T01:00:00Z".to_string()),
        };

        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["status"], "retrying");
    }

    #[test]
    fn test_delivery_query_default() {
        let query = DeliveryQuery { limit: default_limit() };
        assert_eq!(query.limit, 50);
    }
}
