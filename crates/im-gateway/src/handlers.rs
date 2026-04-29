use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    Json as JsonResponse,
};
use common::ApiResponse;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::*;
use crate::services::{IMService, ConversationService};
use crate::middleware::Auth;

/// 发送消息
pub async fn send_message(
    State(service): State<Arc<IMService>>,
    Auth(claims): Auth,
    Json(request): Json<SendMessageRequest>,
) -> Result<JsonResponse<ApiResponse<SendMessageResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match service.send_message(request, claims.user_id).await {
        Ok(response) => Ok(JsonResponse(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("Send message error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(ApiResponse::error(e.to_string())),
            ))
        }
    }
}

/// 创建对话
pub async fn create_conversation(
    State(service): State<Arc<ConversationService>>,
    Auth(claims): Auth,
    Json(request): Json<CreateConversationRequest>,
) -> Result<JsonResponse<ApiResponse<CreateConversationResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match service.create_conversation(request, claims.user_id).await {
        Ok(response) => Ok(JsonResponse(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("Create conversation error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(ApiResponse::error(e.to_string())),
            ))
        }
    }
}

/// 获取对话列表
pub async fn list_conversations(
    State(service): State<Arc<ConversationService>>,
    Auth(claims): Auth,
) -> Result<JsonResponse<ApiResponse<ConversationsListResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match service.list_conversations(claims.user_id).await {
        Ok(response) => Ok(JsonResponse(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("List conversations error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(ApiResponse::error(e.to_string())),
            ))
        }
    }
}

/// 获取对话详情
pub async fn get_conversation(
    State(service): State<Arc<ConversationService>>,
    Auth(_claims): Auth,
    Path(conversation_id): Path<Uuid>,
) -> Result<JsonResponse<ApiResponse<ConversationInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    match service.get_conversation(conversation_id).await {
        Ok(response) => Ok(JsonResponse(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("Get conversation error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(ApiResponse::error(e.to_string())),
            ))
        }
    }
}

/// 获取消息历史
pub async fn get_message_history(
    State(service): State<Arc<IMService>>,
    Auth(_claims): Auth,
    Path(conversation_id): Path<Uuid>,
    Query(query): Query<MessageHistoryRequest>,
) -> Result<JsonResponse<ApiResponse<MessageHistoryResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let limit = query.limit.unwrap_or(50);
    let before_message_id = query.before_message_id;

    match service.get_message_history(conversation_id, limit, before_message_id).await {
        Ok(response) => Ok(JsonResponse(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("Get message history error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(ApiResponse::error(e.to_string())),
            ))
        }
    }
}

/// 标记已读
pub async fn mark_read(
    State(service): State<Arc<IMService>>,
    Auth(claims): Auth,
    Json(request): Json<MarkReadRequest>,
) -> Result<JsonResponse<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match service.mark_as_read(request.conversation_id, request.message_id, claims.user_id).await {
        Ok(_) => Ok(JsonResponse(ApiResponse::success(()))),
        Err(e) => {
            tracing::error!("Mark read error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(ApiResponse::error(e.to_string())),
            ))
        }
    }
}

/// 获取在线用户
pub async fn get_online_users(
    State(service): State<Arc<IMService>>,
    Auth(_claims): Auth,
) -> Result<JsonResponse<ApiResponse<OnlineUsersResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match service.get_online_users().await {
        Ok(response) => Ok(JsonResponse(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("Get online users error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(ApiResponse::error(e.to_string())),
            ))
        }
    }
}