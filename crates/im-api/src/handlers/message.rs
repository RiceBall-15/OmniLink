use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;
use serde::Deserialize;
use sqlx::PgPool;

use crate::models::auth::ApiResponse;
use crate::models::message::{Message, SendMessageRequest, EditMessageRequest, CreateMessageParams, MessageType};
use crate::db::message::{create_message, get_messages_by_conversation, get_message_by_id, update_message_content, recall_message, mark_conversation_as_read, can_edit_message, can_recall_message, get_last_message, count_unread_messages};

/// 获取会话的消息列表（分页）
#[derive(Debug, Deserialize)]
pub struct GetMessagesQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_page() -> i64 { 1 }
fn default_limit() -> i64 { 50 }

pub async fn get_messages(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    Path(conversation_id): Path<String>,
    Query(query): Query<GetMessagesQuery>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    // 解析会话 ID
    let conv_uuid = match conversation_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的会话 ID")),
            );
        }
    };

    // 解析用户 ID
    let user_uuid = match user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户 ID")),
            );
        }
    };

    // 检查用户是否是会话参与者
    let is_participant = match crate::db::conversation::is_conversation_participant(&pool, &conv_uuid, &user_uuid).await {
        Ok(result) => result,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("CHECK_PARTICIPANT_FAILED", format!("检查参与者失败: {}", e))),
            );
        }
    };

    if !is_participant {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "您不是此会话的参与者")),
        );
    }

    // 获取消息列表
    match get_messages_by_conversation(&pool, &conv_uuid, query.page, query.limit).await {
        Ok(messages) => {
            let message_list: Vec<Message> = messages.iter().map(|m| m.to_message()).collect();
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::to_value(message_list).unwrap())),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("GET_MESSAGES_FAILED", format!("获取消息失败: {}", e))),
        ),
    }
}

/// 发送消息
pub async fn send_message(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    Path(conversation_id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    // 解析会话 ID
    let conv_uuid = match conversation_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的会话 ID")),
            );
        }
    };

    // 解析用户 ID
    let sender_uuid = match user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户 ID")),
            );
        }
    };

    // 检查用户是否是会话参与者
    let is_participant = match crate::db::conversation::is_conversation_participant(&pool, &conv_uuid, &sender_uuid).await {
        Ok(result) => result,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("CHECK_PARTICIPANT_FAILED", format!("检查参与者失败: {}", e))),
            );
        }
    };

    if !is_participant {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "您不是此会话的参与者")),
        );
    }

    // 验证消息内容
    if req.content.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("EMPTY_CONTENT", "消息内容不能为空")),
        );
    }

    // 创建消息参数
    let params = CreateMessageParams {
        conversation_id: conv_uuid,
        sender_id: sender_uuid,
        content: req.content,
        type_: req.type_,
        reply_to: None,
        metadata: None,
    };

    // 创建消息
    match create_message(&pool, params).await {
        Ok(message_entity) => {
            let message = message_entity.to_message();

            // 更新会话的更新时间
            let _ = sqlx::query(
                r#"
                UPDATE conversations
                SET updated_at = NOW()
                WHERE id = $1
                "#
            )
            .bind(conv_uuid)
            .execute(&pool)
            .await;

            (
                StatusCode::CREATED,
                Json(ApiResponse::success(serde_json::to_value(message).unwrap())),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SEND_MESSAGE_FAILED", format!("发送消息失败: {}", e))),
        ),
    }
}

/// 编辑消息
pub async fn edit_message(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    Path((conversation_id, message_id)): Path<(String, String)>,
    Json(req): Json<EditMessageRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    // 解析 ID
    let msg_uuid = match message_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的消息 ID")),
            );
        }
    };

    let user_uuid = match user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户 ID")),
            );
        }
    };

    // 验证消息内容
    if req.content.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("EMPTY_CONTENT", "消息内容不能为空")),
        );
    }

    // 获取消息
    let message_entity = match get_message_by_id(&pool, &msg_uuid).await {
        Ok(Some(msg)) => msg,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("MESSAGE_NOT_FOUND", "消息不存在")),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("GET_MESSAGE_FAILED", format!("获取消息失败: {}", e))),
            );
        }
    };

    // 检查是否可以编辑
    if !can_edit_message(&message_entity, &user_uuid) {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("CANNOT_EDIT", "无法编辑此消息（只能编辑自己的消息，且在发送后2分钟内）")),
        );
    }

    // 更新消息
    match update_message_content(&pool, &msg_uuid, &req.content).await {
        Ok(updated) => {
            let message = updated.to_message();
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::to_value(message).unwrap())),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("EDIT_MESSAGE_FAILED", format!("编辑消息失败: {}", e))),
        ),
    }
}

/// 撤回消息
pub async fn recall_message_handler(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    Path((conversation_id, message_id)): Path<(String, String)>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    // 解析 ID
    let msg_uuid = match message_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的消息 ID")),
            );
        }
    };

    let user_uuid = match user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户 ID")),
            );
        }
    };

    // 获取消息
    let message_entity = match get_message_by_id(&pool, &msg_uuid).await {
        Ok(Some(msg)) => msg,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("MESSAGE_NOT_FOUND", "消息不存在")),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("GET_MESSAGE_FAILED", format!("获取消息失败: {}", e))),
            );
        }
    };

    // 检查是否可以撤回
    if !can_recall_message(&message_entity, &user_uuid) {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("CANNOT_RECALL", "无法撤回此消息（只能撤回自己的消息，且在发送后2分钟内）")),
        );
    }

    // 撤回消息
    match recall_message(&pool, &msg_uuid).await {
        Ok(recalled) => {
            let message = recalled.to_message();
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::to_value(message).unwrap())),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("RECALL_MESSAGE_FAILED", format!("撤回消息失败: {}", e))),
        ),
    }
}

/// 标记会话消息为已读
pub async fn mark_as_read_handler(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    Path(conversation_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    // 解析 ID
    let conv_uuid = match conversation_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的会话 ID")),
            );
        }
    };

    let user_uuid = match user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户 ID")),
            );
        }
    };

    // 检查用户是否是会话参与者
    let is_participant = match crate::db::conversation::is_conversation_participant(&pool, &conv_uuid, &user_uuid).await {
        Ok(result) => result,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("CHECK_PARTICIPANT_FAILED", format!("检查参与者失败: {}", e))),
            );
        }
    };

    if !is_participant {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "您不是此会话的参与者")),
        );
    }

    // 标记已读
    match mark_conversation_as_read(&pool, &conv_uuid, &user_uuid).await {
        Ok(()) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({ "success": true }))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("MARK_READ_FAILED", format!("标记已读失败: {}", e))),
        ),
    }
}
