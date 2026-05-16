//! Gateway 请求处理器模块
//!
//! 提供 HTTP API 端点和 WebSocket 处理：
//! - `ws`: WebSocket 连接处理和消息路由
//! - HTTP API: 消息发送、历史查询、已读标记、会话管理等

pub mod ws;
pub mod file_upload;
pub mod user_management;

use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;

use crate::middleware::AuthUser;
use crate::models::{
    SendMessageRequest, MessageHistoryQuery, MarkReadRequest, EditMessageRequest, RecallMessageRequest,
    CreateConversationRequest, ConversationsQuery,
    BatchStatusQuery,
};
use crate::services::IMService;
use crate::conversation_service::ConversationService;
use common::ApiResponse;
use uuid::Uuid;

/// 发送消息
pub async fn send_message(
    State(im_service): State<Arc<IMService>>,
    auth: AuthUser,
    Json(request): Json<SendMessageRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    match im_service.send_message(request, auth.0.sub).await {
        Ok(response) => Ok((
            StatusCode::CREATED,
            Json(ApiResponse::success(response)),
        )),
        Err(e) => {
            tracing::error!("Failed to send message: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取消息历史
pub async fn get_message_history(
    State(im_service): State<Arc<IMService>>,
    _auth: AuthUser,
    Query(query): Query<MessageHistoryQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    match im_service
        .get_message_history(query.conversation_id, query.limit.unwrap_or(50), query.before_message_id)
        .await
    {
        Ok(response) => Ok(Json(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("Failed to get message history: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 标记消息已读
pub async fn mark_read(
    State(im_service): State<Arc<IMService>>,
    auth: AuthUser,
    Json(request): Json<MarkReadRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    match im_service
        .mark_as_read(request.conversation_id, request.message_id, auth.0.sub)
        .await
    {
        Ok(_) => Ok(Json(ApiResponse::success(serde_json::json!({
            "status": "read"
        })))),
        Err(e) => {
            tracing::error!("Failed to mark message as read: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 编辑消息
pub async fn edit_message(
    State(im_service): State<Arc<IMService>>,
    auth: AuthUser,
    Json(request): Json<EditMessageRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    match im_service
        .edit_message(request.conversation_id, request.message_id, auth.0.sub, request.content)
        .await
    {
        Ok(_) => Ok(Json(ApiResponse::success(serde_json::json!({
            "status": "edited"
        })))),
        Err(e) => {
            tracing::error!("Failed to edit message: {}", e);
            match e {
                common::AppError::Authorization(_) => Err(StatusCode::FORBIDDEN),
                common::AppError::NotFound(_) => Err(StatusCode::NOT_FOUND),
                _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    }
}

/// 撤回消息
pub async fn recall_message(
    State(im_service): State<Arc<IMService>>,
    auth: AuthUser,
    Json(request): Json<RecallMessageRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    match im_service
        .recall_message(request.conversation_id, request.message_id, auth.0.sub)
        .await
    {
        Ok(_) => Ok(Json(ApiResponse::success(serde_json::json!({
            "status": "recalled"
        })))),
        Err(e) => {
            tracing::error!("Failed to recall message: {}", e);
            match e {
                common::AppError::Authorization(_) => Err(StatusCode::FORBIDDEN),
                common::AppError::NotFound(_) => Err(StatusCode::NOT_FOUND),
                _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    }
}

/// 获取在线用户
pub async fn get_online_users(
    State(im_service): State<Arc<IMService>>,
) -> Result<impl IntoResponse, StatusCode> {
    match im_service.get_online_users().await {
        Ok(response) => Ok(Json(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("Failed to get online users: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 批量查询用户在线状态
pub async fn batch_status_query(
    State(im_service): State<Arc<IMService>>,
    Json(request): Json<BatchStatusQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    match im_service.get_batch_status(&request.user_ids).await {
        Ok(response) => Ok(Json(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("Failed to batch query user status: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 创建对话
pub async fn create_conversation(
    State(conv_service): State<Arc<ConversationService>>,
    auth: AuthUser,
    Json(request): Json<CreateConversationRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    match conv_service.create_conversation(request, auth.0.sub).await {
        Ok(response) => Ok((
            StatusCode::CREATED,
            Json(ApiResponse::success(response)),
        )),
        Err(e) => {
            tracing::error!("Failed to create conversation: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取对话列表
pub async fn list_conversations(
    State(conv_service): State<Arc<ConversationService>>,
    auth: AuthUser,
    Query(query): Query<ConversationsQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let _ = query; // 参数暂不使用
    match conv_service.list_conversations(auth.0.sub).await {
        Ok(response) => Ok(Json(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("Failed to list conversations: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取对话详情
pub async fn get_conversation(
    State(conv_service): State<Arc<ConversationService>>,
    _auth: AuthUser,
    Path(conversation_id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    match conv_service.get_conversation(conversation_id).await {
        Ok(response) => Ok(Json(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("Failed to get conversation: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
