use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;
use sqlx::PgPool;

use crate::models::auth::ApiResponse;
use crate::models::conversation::{
    Conversation, CreateConversationRequest, CreateConversationParams, ConversationType,
};
use crate::db::conversation::{
    create_conversation, get_conversations_by_user, get_conversation_by_id,
};
use crate::db::message::get_last_message;

/// 获取用户的会话列表
pub async fn get_conversations(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
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

    // 获取会话列表
    match get_conversations_by_user(&pool, &user_uuid).await {
        Ok(conversation_entities) => {
            let mut conversations: Vec<Conversation> = Vec::new();

            for conv_entity in conversation_entities {
                let mut conversation = conv_entity.to_conversation();

                // 获取最后一条消息
                if let Ok(Some(last_msg_entity)) = get_last_message(&pool, &conv_entity.id).await {
                    conversation.last_message = Some(last_msg_entity.to_message());
                }

                conversations.push(conversation);
            }

            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::to_value(conversations).unwrap())),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("GET_CONVERSATIONS_FAILED", format!("获取会话列表失败: {}", e))),
        ),
    }
}

/// 创建新会话
pub async fn create_conversation_handler(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    Json(req): Json<CreateConversationRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
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

    // 验证参与者 ID
    let participant_ids = match req.participant_ids {
        Some(ids) => {
            let mut parsed_ids = Vec::new();
            for id_str in ids {
                match id_str.parse::<Uuid>() {
                    Ok(uuid) => parsed_ids.push(uuid),
                    Err(_) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(ApiResponse::error("INVALID_PARTICIPANT_ID", format!("无效的参与者 ID: {}", id_str))),
                        );
                    }
                }
            }
            parsed_ids
        }
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("MISSING_PARTICIPANTS", "必须指定参与者")),
            );
        }
    };

    // 确保创建者也是参与者
    if !participant_ids.contains(&user_uuid) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("CREATOR_NOT_PARTICIPANT", "创建者必须是参与者")),
        );
    }

    // 对于直接会话，检查是否已存在
    if req.type_ == ConversationType::Direct && participant_ids.len() == 2 {
        let other_user_id = participant_ids.iter().find(|&&id| id != user_uuid).unwrap();
        match crate::db::conversation::find_or_create_direct_conversation(&pool, &user_uuid, other_user_id).await {
            Ok(existing_conv) => {
                let conversation = existing_conv.to_conversation();
                return (
                    StatusCode::OK,
                    Json(ApiResponse::success(serde_json::to_value(conversation).unwrap())),
                );
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("FIND_CONVERSATION_FAILED", format!("查找会话失败: {}", e))),
                );
            }
        }
    }

    // 验证会话类型和参与者数量
    match req.type_ {
        ConversationType::Direct => {
            if participant_ids.len() != 2 {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error("INVALID_PARTICIPANT_COUNT", "直接会话必须恰好有2个参与者")),
                );
            }
        }
        ConversationType::Group => {
            if participant_ids.len() < 3 {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error("INVALID_PARTICIPANT_COUNT", "群组会话至少需要3个参与者")),
                );
            }
        }
        ConversationType::Ai => {
            // AI 会话的特殊处理
            if !participant_ids.contains(&user_uuid) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error("CREATOR_NOT_PARTICIPANT", "创建者必须是参与者")),
                );
            }
        }
    }

    // 创建会话参数
    let params = CreateConversationParams {
        type_: req.type_,
        name: req.name,
        avatar: None,
        created_by: user_uuid,
        participant_ids,
    };

    // 创建会话
    match create_conversation(&pool, params).await {
        Ok(conv_entity) => {
            let conversation = conv_entity.to_conversation();
            (
                StatusCode::CREATED,
                Json(ApiResponse::success(serde_json::to_value(conversation).unwrap())),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("CREATE_CONVERSATION_FAILED", format!("创建会话失败: {}", e))),
        ),
    }
}
