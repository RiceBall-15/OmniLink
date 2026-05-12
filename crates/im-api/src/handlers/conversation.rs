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
    SearchConversationsQuery,
};
use crate::db::conversation::{
    create_conversation, get_conversations_by_user, get_conversation_by_id,
    toggle_pin_conversation, toggle_mute_conversation, toggle_archive_conversation,
    search_conversations,
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

/// 获取群组成员列表
pub async fn get_group_members(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    axum::extract::Path(conversation_id): axum::extract::Path<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户 ID")),
            );
        }
    };

    let conv_uuid = match conversation_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_CONVERSATION_ID", "无效的会话 ID")),
            );
        }
    };

    // 检查用户是否是会话参与者
    match crate::db::conversation::is_conversation_participant(&pool, &conv_uuid, &user_uuid).await {
        Ok(true) => {}
        Ok(false) => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("NOT_PARTICIPANT", "您不是该会话的参与者")),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("CHECK_PARTICIPANT_FAILED", format!("检查参与者失败: {}", e))),
            );
        }
    }

    // 获取参与者列表
    match crate::db::conversation::get_conversation_participants(&pool, &conv_uuid).await {
        Ok(participants) => {
            let participant_ids: Vec<String> = participants.iter().map(|id| id.to_string()).collect();
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "conversation_id": conversation_id,
                    "members": participant_ids,
                    "count": participants.len()
                }))),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("GET_MEMBERS_FAILED", format!("获取成员列表失败: {}", e))),
        ),
    }
}

/// 添加群组成员
pub async fn add_group_members(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    axum::extract::Path(conversation_id): axum::extract::Path<String>,
    Json(req): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户 ID")),
            );
        }
    };

    let conv_uuid = match conversation_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_CONVERSATION_ID", "无效的会话 ID")),
            );
        }
    };

    // 检查用户是否是群主
    match crate::db::conversation::is_group_owner(&pool, &conv_uuid, &user_uuid).await {
        Ok(true) => {}
        Ok(false) => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("NOT_GROUP_OWNER", "只有群主可以添加成员")),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("CHECK_OWNER_FAILED", format!("检查群主失败: {}", e))),
            );
        }
    }

    // 解析要添加的用户 ID 列表
    let user_ids: Vec<Uuid> = match req.get("user_ids").and_then(|v| v.as_array()) {
        Some(ids) => {
            let mut parsed_ids = Vec::new();
            for id_value in ids {
                if let Some(id_str) = id_value.as_str() {
                    match id_str.parse::<Uuid>() {
                        Ok(uuid) => parsed_ids.push(uuid),
                        Err(_) => {
                            return (
                                StatusCode::BAD_REQUEST,
                                Json(ApiResponse::error("INVALID_USER_ID", format!("无效的用户 ID: {}", id_str))),
                            );
                        }
                    }
                }
            }
            parsed_ids
        }
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("MISSING_USER_IDS", "必须指定要添加的用户 ID 列表")),
            );
        }
    };

    if user_ids.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("EMPTY_USER_IDS", "用户 ID 列表不能为空")),
        );
    }

    // 批量添加成员
    match crate::db::conversation::add_participants(&pool, &conv_uuid, &user_ids).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "message": "成员添加成功",
                "added_count": user_ids.len()
            }))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("ADD_MEMBERS_FAILED", format!("添加成员失败: {}", e))),
        ),
    }
}

/// 移除群组成员
pub async fn remove_group_member(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    axum::extract::Path((conversation_id, member_id)): axum::extract::Path<(String, String)>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户 ID")),
            );
        }
    };

    let conv_uuid = match conversation_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_CONVERSATION_ID", "无效的会话 ID")),
            );
        }
    };

    let member_uuid = match member_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_MEMBER_ID", "无效的成员 ID")),
            );
        }
    };

    // 检查用户是否是群主
    match crate::db::conversation::is_group_owner(&pool, &conv_uuid, &user_uuid).await {
        Ok(true) => {}
        Ok(false) => {
            // 群主可以移除任何人，普通成员只能移除自己
            if user_uuid != member_uuid {
                return (
                    StatusCode::FORBIDDEN,
                    Json(ApiResponse::error("NOT_GROUP_OWNER", "只有群主可以移除其他成员")),
                );
            }
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("CHECK_OWNER_FAILED", format!("检查群主失败: {}", e))),
            );
        }
    }

    // 移除成员
    match crate::db::conversation::remove_participant(&pool, &conv_uuid, &member_uuid).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "message": "成员移除成功"
            }))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("REMOVE_MEMBER_FAILED", format!("移除成员失败: {}", e))),
        ),
    }
}

/// 更新群组信息
pub async fn update_group_info(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    axum::extract::Path(conversation_id): axum::extract::Path<String>,
    Json(req): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户 ID")),
            );
        }
    };

    let conv_uuid = match conversation_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_CONVERSATION_ID", "无效的会话 ID")),
            );
        }
    };

    // 检查用户是否是群主
    match crate::db::conversation::is_group_owner(&pool, &conv_uuid, &user_uuid).await {
        Ok(true) => {}
        Ok(false) => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("NOT_GROUP_OWNER", "只有群主可以更新群组信息")),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("CHECK_OWNER_FAILED", format!("检查群主失败: {}", e))),
            );
        }
    }

    let name = req.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
    let avatar = req.get("avatar").and_then(|v| v.as_str()).map(|s| s.to_string());
    let announcement = req.get("announcement").and_then(|v| v.as_str()).map(|s| s.to_string());

    // 更新基本信息
    match crate::db::conversation::update_conversation(&pool, &conv_uuid, name, avatar).await {
        Ok(_) => {}
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("UPDATE_GROUP_FAILED", format!("更新群组信息失败: {}", e))),
            );
        }
    }

    // 更新公告（如果有）
    if let Some(announcement_text) = announcement {
        match crate::db::conversation::update_group_announcement(&pool, &conv_uuid, &announcement_text).await {
            Ok(_) => {}
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("UPDATE_ANNOUNCEMENT_FAILED", format!("更新群公告失败: {}", e))),
                );
            }
        }
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "message": "群组信息更新成功"
        }))),
    )
}

/// 获取群公告
pub async fn get_group_announcement(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    axum::extract::Path(conversation_id): axum::extract::Path<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户 ID")),
            );
        }
    };

    let conv_uuid = match conversation_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_CONVERSATION_ID", "无效的会话 ID")),
            );
        }
    };

    // 检查用户是否是会话参与者
    match crate::db::conversation::is_conversation_participant(&pool, &conv_uuid, &user_uuid).await {
        Ok(true) => {}
        Ok(false) => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("NOT_PARTICIPANT", "您不是该会话的参与者")),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("CHECK_PARTICIPANT_FAILED", format!("检查参与者失败: {}", e))),
            );
        }
    }

    match crate::db::conversation::get_group_announcement(&pool, &conv_uuid).await {
        Ok(announcement) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "announcement": announcement
            }))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("GET_ANNOUNCEMENT_FAILED", format!("获取群公告失败: {}", e))),
        ),
    }
}

/// 更新群公告（处理器）
pub async fn update_group_announcement_handler(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    axum::extract::Path(conversation_id): axum::extract::Path<String>,
    announcement: &str,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户 ID")),
            );
        }
    };

    let conv_uuid = match conversation_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_CONVERSATION_ID", "无效的会话 ID")),
            );
        }
    };

    // 检查用户是否是群主
    match crate::db::conversation::is_group_owner(&pool, &conv_uuid, &user_uuid).await {
        Ok(true) => {}
        Ok(false) => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("NOT_GROUP_OWNER", "只有群主可以更新群公告")),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("CHECK_OWNER_FAILED", format!("检查群主失败: {}", e))),
            );
        }
    }

    match crate::db::conversation::update_group_announcement(&pool, &conv_uuid, announcement).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "message": "群公告更新成功"
            }))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("UPDATE_ANNOUNCEMENT_FAILED", format!("更新群公告失败: {}", e))),
        ),
    }
}

/// 切换会话置顶状态
pub async fn toggle_pin(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    axum::extract::Path(conversation_id): axum::extract::Path<String>,
    Json(req): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户 ID")),
            );
        }
    };

    let conv_uuid = match conversation_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_CONVERSATION_ID", "无效的会话 ID")),
            );
        }
    };

    // 检查用户是否是会话参与者
    match crate::db::conversation::is_conversation_participant(&pool, &conv_uuid, &user_uuid).await {
        Ok(true) => {}
        Ok(false) => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("NOT_PARTICIPANT", "您不是该会话的参与者")),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("CHECK_PARTICIPANT_FAILED", format!("检查参与者失败: {}", e))),
            );
        }
    }

    let is_pinned = req.get("isPinned").and_then(|v| v.as_bool()).unwrap_or(false);

    match toggle_pin_conversation(&pool, &conv_uuid, is_pinned).await {
        Ok(conv) => {
            let conversation = conv.to_conversation();
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::to_value(conversation).unwrap())),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("TOGGLE_PIN_FAILED", format!("更新置顶状态失败: {}", e))),
        ),
    }
}

/// 切换会话免打扰状态
pub async fn toggle_mute(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    axum::extract::Path(conversation_id): axum::extract::Path<String>,
    Json(req): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户 ID")),
            );
        }
    };

    let conv_uuid = match conversation_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_CONVERSATION_ID", "无效的会话 ID")),
            );
        }
    };

    // 检查用户是否是会话参与者
    match crate::db::conversation::is_conversation_participant(&pool, &conv_uuid, &user_uuid).await {
        Ok(true) => {}
        Ok(false) => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("NOT_PARTICIPANT", "您不是该会话的参与者")),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("CHECK_PARTICIPANT_FAILED", format!("检查参与者失败: {}", e))),
            );
        }
    }

    let is_muted = req.get("isMuted").and_then(|v| v.as_bool()).unwrap_or(false);

    match toggle_mute_conversation(&pool, &conv_uuid, is_muted).await {
        Ok(conv) => {
            let conversation = conv.to_conversation();
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::to_value(conversation).unwrap())),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("TOGGLE_MUTE_FAILED", format!("更新免打扰状态失败: {}", e))),
        ),
    }
}

/// 切换会话归档状态
pub async fn toggle_archive(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    axum::extract::Path(conversation_id): axum::extract::Path<String>,
    Json(req): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户 ID")),
            );
        }
    };

    let conv_uuid = match conversation_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_CONVERSATION_ID", "无效的会话 ID")),
            );
        }
    };

    // 检查用户是否是会话参与者
    match crate::db::conversation::is_conversation_participant(&pool, &conv_uuid, &user_uuid).await {
        Ok(true) => {}
        Ok(false) => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("NOT_PARTICIPANT", "您不是该会话的参与者")),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("CHECK_PARTICIPANT_FAILED", format!("检查参与者失败: {}", e))),
            );
        }
    }

    let is_archived = req.get("isArchived").and_then(|v| v.as_bool()).unwrap_or(true);

    match toggle_archive_conversation(&pool, &conv_uuid, is_archived).await {
        Ok(conv) => {
            let conversation = conv.to_conversation();
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::to_value(conversation).unwrap())),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("TOGGLE_ARCHIVE_FAILED", format!("更新归档状态失败: {}", e))),
        ),
    }
}

/// 搜索会话
pub async fn search(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    axum::extract::Query(query): axum::extract::Query<SearchConversationsQuery>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户 ID")),
            );
        }
    };

    if query.q.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("EMPTY_QUERY", "搜索关键词不能为空")),
        );
    }

    match search_conversations(&pool, &user_uuid, &query.q, query.include_archived).await {
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
                Json(ApiResponse::success(serde_json::json!({
                    "conversations": conversations,
                    "total": conversations.len()
                }))),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SEARCH_CONVERSATIONS_FAILED", format!("搜索会话失败: {}", e))),
        ),
    }
}
