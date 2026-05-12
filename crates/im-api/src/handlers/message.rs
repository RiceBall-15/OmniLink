//! 消息处理器模块
//!
//! 提供消息管理相关的 API 端点：
//! - `GET /api/im/conversations/:id/messages` - 获取消息列表（分页）
//! - `POST /api/im/conversations/:id/messages` - 发送消息
//! - `PUT /api/im/conversations/:id/messages/:msg_id` - 编辑消息
//! - `POST /api/im/conversations/:id/messages/:msg_id/recall` - 撤回消息
//! - `POST /api/im/conversations/:id/read` - 标记已读
//! - `GET /api/im/conversations/:id/messages/search` - 搜索消息
//! - `GET /api/im/conversations/:id/messages/stats` - 消息统计

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;
use serde::Deserialize;
use sqlx::PgPool;

use crate::models::auth::ApiResponse;
use crate::models::message::{Message, SendMessageRequest, EditMessageRequest, CreateMessageParams};
use crate::db::message::{create_message, get_messages_by_conversation, get_message_by_id, update_message_content, recall_message, mark_conversation_as_read, can_edit_message, can_recall_message};

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
    let reply_to_uuid = req.reply_to.and_then(|s| s.parse::<Uuid>().ok());

    let params = CreateMessageParams {
        conversation_id: conv_uuid,
        sender_id: sender_uuid,
        content: req.content,
        type_: req.type_,
        reply_to: reply_to_uuid,
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
    Path((_conversation_id, message_id)): Path<(String, String)>,
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
    Path((_conversation_id, message_id)): Path<(String, String)>,
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

/// 搜索消息请求
#[derive(Debug, Deserialize)]
pub struct SearchMessagesQuery {
    pub keyword: String,
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

/// 搜索会话中的消息
pub async fn search_messages(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    Path(conversation_id): Path<String>,
    Query(query): Query<SearchMessagesQuery>,
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

    // 验证搜索关键词
    if query.keyword.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("EMPTY_KEYWORD", "搜索关键词不能为空")),
        );
    }

    // 搜索消息
    match crate::db::message::search_messages_in_conversation(
        &pool,
        &conv_uuid,
        &query.keyword,
        query.page,
        query.limit,
    )
    .await
    {
        Ok(messages) => {
            let message_list: Vec<Message> = messages.iter().map(|m| m.to_message()).collect();
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "messages": message_list,
                    "keyword": query.keyword,
                    "page": query.page,
                    "limit": query.limit,
                }))),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SEARCH_FAILED", format!("搜索消息失败: {}", e))),
        ),
    }
}

/// 获取消息统计
pub async fn get_message_stats_handler(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    Path(conversation_id): Path<String>,
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

    // 获取统计
    match crate::db::message::get_message_stats(&pool, &conv_uuid).await {
        Ok(stats) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "total_count": stats.total_count,
                "sender_count": stats.sender_count,
                "first_message_at": stats.first_message_at,
                "last_message_at": stats.last_message_at,
            }))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("GET_STATS_FAILED", format!("获取消息统计失败: {}", e))),
        ),
    }
}

/// 添加消息表情回应
pub async fn add_reaction(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(message_id): Path<String>,
    Json(req): Json<AddReactionRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let msg_uuid = match message_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的消息ID")),
            );
        }
    };

    let user_uuid = match auth.user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID")),
            );
        }
    };

    // 验证 emoji 非空
    if req.emoji.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_EMOJI", "表情不能为空")),
        );
    }

    // 检查消息是否存在
    let message = match sqlx::query_as::<_, crate::db::message::MessageEntity>(
        "SELECT * FROM messages WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(msg_uuid)
    .fetch_optional(&*pool)
    .await
    {
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
                Json(ApiResponse::error("DB_ERROR", format!("查询消息失败: {}", e))),
            );
        }
    };

    // 检查用户是否在会话中
    let is_member = match sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM conversation_members WHERE conversation_id = $1 AND user_id = $2)"
    )
    .bind(message.conversation_id)
    .bind(user_uuid)
    .fetch_one(&*pool)
    .await
    {
        Ok(exists) => exists,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("DB_ERROR", format!("检查成员失败: {}", e))),
            );
        }
    };

    if !is_member {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("NOT_MEMBER", "您不是该会话的成员")),
        );
    }

    // 插入或更新表情回应（使用 UPSERT）
    let reaction_id = Uuid::new_v4();
    match sqlx::query(
        "INSERT INTO message_reactions (id, message_id, user_id, emoji, created_at)
         VALUES ($1, $2, $3, $4, NOW())
         ON CONFLICT (message_id, user_id, emoji) DO NOTHING
         RETURNING id"
    )
    .bind(reaction_id)
    .bind(msg_uuid)
    .bind(user_uuid)
    .bind(&req.emoji)
    .fetch_optional(&*pool)
    .await
    {
        Ok(_) => {
            // 获取该消息的所有回应统计
            match get_reaction_summaries(&pool, msg_uuid).await {
                Ok(reactions) => (
                    StatusCode::OK,
                    Json(ApiResponse::success(serde_json::json!({
                        "message_id": msg_uuid,
                        "reactions": reactions,
                    }))),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("DB_ERROR", format!("获取回应失败: {}", e))),
                ),
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("DB_ERROR", format!("添加回应失败: {}", e))),
        ),
    }
}

/// 删除消息表情回应
pub async fn remove_reaction(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path((message_id, emoji)): Path<(String, String)>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let msg_uuid = match message_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的消息ID")),
            );
        }
    };

    let user_uuid = match auth.user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID")),
            );
        }
    };

    match sqlx::query(
        "DELETE FROM message_reactions WHERE message_id = $1 AND user_id = $2 AND emoji = $3"
    )
    .bind(msg_uuid)
    .bind(user_uuid)
    .bind(&emoji)
    .execute(&*pool)
    .await
    {
        Ok(_) => {
            match get_reaction_summaries(&pool, msg_uuid).await {
                Ok(reactions) => (
                    StatusCode::OK,
                    Json(ApiResponse::success(serde_json::json!({
                        "message_id": msg_uuid,
                        "reactions": reactions,
                    }))),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("DB_ERROR", format!("获取回应失败: {}", e))),
                ),
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("DB_ERROR", format!("删除回应失败: {}", e))),
        ),
    }
}

/// 获取消息的表情回应列表
pub async fn get_reactions(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(message_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let msg_uuid = match message_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的消息ID")),
            );
        }
    };

    match get_reaction_summaries(&pool, msg_uuid).await {
        Ok(reactions) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "message_id": msg_uuid,
                "reactions": reactions,
            }))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("DB_ERROR", format!("获取回应失败: {}", e))),
        ),
    }
}

/// 获取消息回应统计的辅助函数
async fn get_reaction_summaries(
    pool: &PgPool,
    message_id: Uuid,
) -> Result<Vec<ReactionSummary>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, i64, Vec<Uuid>)>(
        "SELECT emoji, COUNT(*) as count, array_agg(user_id) as users
         FROM message_reactions
         WHERE message_id = $1
         GROUP BY emoji
         ORDER BY count DESC"
    )
    .bind(message_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(emoji, count, users)| ReactionSummary {
        emoji,
        count,
        users,
    }).collect())
}

/// 转发消息请求
#[derive(Debug, Deserialize)]
pub struct ForwardMessageRequest {
    /// 目标会话 ID 列表
    pub target_conversation_ids: Vec<String>,
}

/// 转发消息到多个会话
pub async fn forward_message(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    Path((conversation_id, message_id)): Path<(String, String)>,
    Json(req): Json<ForwardMessageRequest>,
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

    // 获取原始消息
    let original_message = match get_message_by_id(&pool, &msg_uuid).await {
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

    // 检查用户是否是原始会话的参与者
    let is_participant = match crate::db::conversation::is_conversation_participant(
        &pool,
        &original_message.conversation_id,
        &user_uuid,
    ).await {
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
            Json(ApiResponse::error("FORBIDDEN", "您不是原始会话的参与者")),
        );
    }

    // 验证目标会话数量
    if req.target_conversation_ids.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("EMPTY_TARGETS", "至少需要一个目标会话")),
        );
    }

    if req.target_conversation_ids.len() > 10 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("TOO_MANY_TARGETS", "最多同时转发到10个会话")),
        );
    }

    // 转发消息到每个目标会话
    let mut forwarded_messages = Vec::new();
    let mut errors = Vec::new();

    for target_id_str in &req.target_conversation_ids {
        let target_conv_id = match target_id_str.parse::<Uuid>() {
            Ok(uuid) => uuid,
            Err(_) => {
                errors.push(format!("无效的会话 ID: {}", target_id_str));
                continue;
            }
        };

        // 检查用户是否是目标会话的参与者
        let is_target_participant = match crate::db::conversation::is_conversation_participant(
            &pool,
            &target_conv_id,
            &user_uuid,
        ).await {
            Ok(result) => result,
            Err(e) => {
                errors.push(format!("检查目标会话 {} 参与者失败: {}", target_id_str, e));
                continue;
            }
        };

        if !is_target_participant {
            errors.push(format!("您不是会话 {} 的参与者", target_id_str));
            continue;
        }

        // 创建转发消息的 metadata
        let forward_metadata = serde_json::json!({
            "forwarded": true,
            "original_message_id": original_message.id,
            "original_sender_id": original_message.sender_id,
            "original_conversation_id": original_message.conversation_id,
            "forwarded_by": user_uuid,
            "forwarded_at": chrono::Utc::now().to_rfc3339(),
        });

        // 创建转发消息
        let params = CreateMessageParams {
            conversation_id: target_conv_id,
            sender_id: user_uuid,
            content: original_message.content.clone(),
            type_: original_message.type_.clone().parse().unwrap_or(crate::models::message::MessageType::Text),
            reply_to: None,
            metadata: Some(forward_metadata),
        };

        match create_message(&pool, params).await {
            Ok(message_entity) => {
                let message = message_entity.to_message();

                // 更新目标会话的更新时间
                let _ = sqlx::query(
                    r#"
                    UPDATE conversations
                    SET updated_at = NOW()
                    WHERE id = $1
                    "#
                )
                .bind(target_conv_id)
                .execute(&pool)
                .await;

                forwarded_messages.push(message);
            }
            Err(e) => {
                errors.push(format!("转发到会话 {} 失败: {}", target_id_str, e));
            }
        }
    }

    // 返回结果
    if forwarded_messages.is_empty() && !errors.is_empty() {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("FORWARD_FAILED", serde_json::json!({
                "message": "转发失败",
                "errors": errors,
            }).to_string())),
        )
    } else {
        (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "forwarded_messages": forwarded_messages,
                "forwarded_count": forwarded_messages.len(),
                "errors": if errors.is_empty() { None } else { Some(errors) },
            }))),
        )
    }
}
