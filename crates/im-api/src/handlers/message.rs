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
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, FromRow};
use chrono::{DateTime, Utc};

use crate::models::auth::ApiResponse;
use crate::models::message::{Message, SendMessageRequest, EditMessageRequest, CreateMessageParams, AddReactionRequest, ReactionSummary, MessageEntity};
use crate::db::message::{create_message, get_messages_by_conversation, get_message_by_id, update_message_content, recall_message, mark_conversation_as_read, can_edit_message, can_recall_message};
use crate::middleware::auth::AuthUser;

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
    pub conversation_id: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

/// 全局搜索消息请求
#[derive(Debug, Deserialize)]
pub struct GlobalSearchMessagesQuery {
    /// 搜索关键词（必填）
    pub q: String,
    /// 可选：限制到特定会话
    pub conversation_id: Option<String>,
    /// 可选：起始时间（ISO 8601 格式）
    pub start_date: Option<String>,
    /// 可选：结束时间（ISO 8601 格式）
    pub end_date: Option<String>,
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

/// 搜索结果项（含高亮和会话信息）
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SearchResultItem {
    /// 消息信息
    pub message: Message,
    /// 消息所属会话 ID
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    /// 高亮后的消息片段（关键词用 <mark> 标签包裹）
    #[serde(rename = "highlightedContent")]
    pub highlighted_content: String,
}

/// 全局搜索响应
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct GlobalSearchResponse {
    pub results: Vec<SearchResultItem>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    pub keyword: String,
}

/// 高亮搜索关键词（在文本中用 <mark> 标签包裹关键词，保留上下文）
fn highlight_keyword(text: &str, keyword: &str, context_chars: usize) -> String {
    if keyword.is_empty() {
        return text.chars().take(context_chars * 2).collect::<String>();
    }

    let lower_text = text.to_lowercase();
    let lower_keyword = keyword.to_lowercase();

    if let Some(pos) = lower_text.find(&lower_keyword) {
        let start = pos.saturating_sub(context_chars);
        let end = (pos + keyword.len() + context_chars).min(text.len());

        let prefix = if start > 0 { "..." } else { "" };
        let suffix = if end < text.len() { "..." } else { "" };

        // 安全地切片（避免截断 UTF-8 字符）
        let snippet: String = text.chars().skip(
            text[..start].chars().count()
        ).take(
            text[start..end].chars().count()
        ).collect();

        // 在 snippet 中高亮关键词（大小写不敏感）
        let mut result = String::new();
        let mut last_end = 0;
        let snippet_lower = snippet.to_lowercase();
        let keyword_lower = keyword.to_lowercase();
        let mut search_from = 0;
        while let Some(p) = snippet_lower[search_from..].find(&keyword_lower) {
            let abs_pos = search_from + p;
            result.push_str(&snippet[last_end..abs_pos]);
            result.push_str("<mark>");
            result.push_str(&snippet[abs_pos..abs_pos + keyword.len()]);
            result.push_str("</mark>");
            last_end = abs_pos + keyword.len();
            search_from = abs_pos + keyword.len();
        }
        result.push_str(&snippet[last_end..]);

        format!("{}{}{}", prefix, result, suffix)
    } else {
        // 没找到关键词，返回前 N 个字符
        let snippet: String = text.chars().take(context_chars * 2).collect();
        if text.chars().count() > context_chars * 2 {
            format!("{}...", snippet)
        } else {
            snippet
        }
    }
}

/// 全局搜索消息（跨会话搜索，支持按会话过滤和时间范围过滤）
pub async fn search_all_messages(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    Query(query): Query<GlobalSearchMessagesQuery>,
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

    // 验证搜索关键词
    if query.q.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("EMPTY_QUERY", "搜索关键词不能为空")),
        );
    }

    // 如果指定了会话ID，验证用户是否是该会话参与者
    if let Some(ref conv_id_str) = query.conversation_id {
        if let Ok(conv_uuid) = conv_id_str.parse::<Uuid>() {
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
        }
    }

    // 执行搜索
    let conv_uuid = query.conversation_id.as_ref().and_then(|s| s.parse::<Uuid>().ok());

    let messages = if let Some(ref conv_id) = conv_uuid {
        // 搜索特定会话
        crate::db::message::search_messages_in_conversation(
            &pool, conv_id, &query.q,
            query.start_date.as_deref(), query.end_date.as_deref(),
            query.page, query.limit,
        ).await
    } else {
        // 跨会话全局搜索
        crate::db::message::search_user_messages(
            &pool, &user_uuid, &query.q,
            query.start_date.as_deref(), query.end_date.as_deref(),
            query.page, query.limit,
        ).await
    };

    match messages {
        Ok(msg_entities) => {
            let total = msg_entities.len() as i64;
            let results: Vec<SearchResultItem> = msg_entities.iter().map(|m| {
                let message = m.to_message();
                let highlighted = highlight_keyword(&m.content, &query.q, 40);
                SearchResultItem {
                    message,
                    conversation_id: m.conversation_id.to_string(),
                    highlighted_content: highlighted,
                }
            }).collect();

            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "results": results,
                    "total": total,
                    "page": query.page,
                    "limit": query.limit,
                    "keyword": query.q,
                }))),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SEARCH_FAILED", format!("搜索消息失败: {}", e))),
        ),
    }
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

    // 搜索消息（带时间范围过滤）
    match crate::db::message::search_messages_in_conversation(
        &pool,
        &conv_uuid,
        &query.keyword,
        query.start_date.as_deref(),
        query.end_date.as_deref(),
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
    let message = match sqlx::query_as::<_, MessageEntity>(
        "SELECT * FROM messages WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(msg_uuid)
    .fetch_optional(&pool)
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
    .fetch_one(&pool)
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
    .fetch_optional(&pool)
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
    .execute(&pool)
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
    _auth: AuthUser,
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
    Path((_conversation_id, message_id)): Path<(String, String)>,
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

// ==================== 会话置顶消息 ====================

/// 置顶消息请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PinMessageRequest {
    /// 要置顶的消息 ID
    #[serde(rename = "messageId")]
    pub message_id: String,
}

/// 置顶消息响应
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PinnedMessageItem {
    /// 置顶记录 ID
    pub id: String,
    /// 消息信息
    pub message: Message,
    /// 置顶人 ID
    #[serde(rename = "pinnedBy")]
    pub pinned_by: String,
    /// 置顶时间
    #[serde(rename = "pinnedAt")]
    pub pinned_at: String,
}

/// 数据库中的置顶消息实体
#[derive(Debug, Clone, FromRow)]
pub struct PinnedMessageEntity {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub message_id: Uuid,
    pub pinned_by: Uuid,
    pub pinned_at: DateTime<Utc>,
}

/// 置顶一条消息到会话
pub async fn pin_message(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    Path(conversation_id): Path<String>,
    Json(request): Json<PinMessageRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    // 解析 ID
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

    let msg_uuid = match request.message_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_MESSAGE_ID", "无效的消息 ID")),
            );
        }
    };

    // 验证用户是会话参与者
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

    // 验证消息存在且属于该会话
    let msg_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM messages WHERE id = $1 AND conversation_id = $2)"
    )
    .bind(&msg_uuid)
    .bind(&conv_uuid)
    .fetch_one(&pool)
    .await;

    match msg_exists {
        Ok(true) => {}
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("MESSAGE_NOT_FOUND", "消息不存在或不属于此会话")),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("CHECK_MESSAGE_FAILED", format!("检查消息失败: {}", e))),
            );
        }
    }

    // 插入置顶记录（如果已置顶则返回冲突）
    let result = sqlx::query_as::<_, PinnedMessageEntity>(
        r#"
        INSERT INTO pinned_messages (conversation_id, message_id, pinned_by)
        VALUES ($1, $2, $3)
        ON CONFLICT (conversation_id, message_id) DO NOTHING
        RETURNING id, conversation_id, message_id, pinned_by, pinned_at
        "#
    )
    .bind(&conv_uuid)
    .bind(&msg_uuid)
    .bind(&user_uuid)
    .fetch_optional(&pool)
    .await;

    match result {
        Ok(Some(entity)) => {
            // 获取消息详情用于响应
            let msg_entity = sqlx::query_as::<_, crate::models::message::MessageEntity>(
                "SELECT * FROM messages WHERE id = $1"
            )
            .bind(&msg_uuid)
            .fetch_one(&pool)
            .await;

            match msg_entity {
                Ok(msg) => {
                    (
                        StatusCode::CREATED,
                        Json(ApiResponse::success(serde_json::json!({
                            "id": entity.id.to_string(),
                            "message": msg.to_message(),
                            "pinnedBy": entity.pinned_by.to_string(),
                            "pinnedAt": entity.pinned_at.to_rfc3339(),
                        }))),
                    )
                }
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("FETCH_MESSAGE_FAILED", format!("获取消息详情失败: {}", e))),
                ),
            }
        }
        Ok(None) => {
            // 已经置顶了，返回现有记录
            let existing = sqlx::query_as::<_, PinnedMessageEntity>(
                "SELECT id, conversation_id, message_id, pinned_by, pinned_at FROM pinned_messages WHERE conversation_id = $1 AND message_id = $2"
            )
            .bind(&conv_uuid)
            .bind(&msg_uuid)
            .fetch_one(&pool)
            .await;

            match existing {
                Ok(entity) => {
                    let msg_entity = sqlx::query_as::<_, crate::models::message::MessageEntity>(
                        "SELECT * FROM messages WHERE id = $1"
                    )
                    .bind(&msg_uuid)
                    .fetch_one(&pool)
                    .await;

                    match msg_entity {
                        Ok(msg) => (
                            StatusCode::OK,
                            Json(ApiResponse::success(serde_json::json!({
                                "id": entity.id.to_string(),
                                "message": msg.to_message(),
                                "pinnedBy": entity.pinned_by.to_string(),
                                "pinnedAt": entity.pinned_at.to_rfc3339(),
                                "alreadyPinned": true,
                            }))),
                        ),
                        Err(e) => (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::error("FETCH_MESSAGE_FAILED", format!("获取消息详情失败: {}", e))),
                        ),
                    }
                }
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("FETCH_PINNED_FAILED", format!("获取置顶记录失败: {}", e))),
                ),
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("PIN_FAILED", format!("置顶消息失败: {}", e))),
        ),
    }
}

/// 取消置顶消息
pub async fn unpin_message(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    Path((conversation_id, message_id)): Path<(String, String)>,
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

    let msg_uuid = match message_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_MESSAGE_ID", "无效的消息 ID")),
            );
        }
    };

    // 验证用户是会话参与者
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

    // 删除置顶记录
    let result = sqlx::query(
        "DELETE FROM pinned_messages WHERE conversation_id = $1 AND message_id = $2"
    )
    .bind(&conv_uuid)
    .bind(&msg_uuid)
    .execute(&pool)
    .await;

    match result {
        Ok(rows) => {
            if rows.rows_affected() > 0 {
                (
                    StatusCode::OK,
                    Json(ApiResponse::success(serde_json::json!({
                        "message": "已取消置顶",
                        "conversationId": conversation_id,
                        "messageId": message_id,
                    }))),
                )
            } else {
                (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::error("NOT_PINNED", "该消息未被置顶")),
                )
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("UNPIN_FAILED", format!("取消置顶失败: {}", e))),
        ),
    }
}

/// 获取会话的置顶消息列表
pub async fn get_pinned_messages(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    Path(conversation_id): Path<String>,
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

    // 验证用户是会话参与者
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

    // 查询置顶消息（联表获取消息详情）
    let pinned = sqlx::query_as::<_, PinnedMessageEntity>(
        r#"
        SELECT id, conversation_id, message_id, pinned_by, pinned_at
        FROM pinned_messages
        WHERE conversation_id = $1
        ORDER BY pinned_at DESC
        "#
    )
    .bind(&conv_uuid)
    .fetch_all(&pool)
    .await;

    match pinned {
        Ok(entities) => {
            let mut items: Vec<serde_json::Value> = Vec::new();
            for entity in &entities {
                // 获取消息详情
                let msg = sqlx::query_as::<_, crate::models::message::MessageEntity>(
                    "SELECT * FROM messages WHERE id = $1"
                )
                .bind(&entity.message_id)
                .fetch_optional(&pool)
                .await;

                if let Ok(Some(msg_entity)) = msg {
                    items.push(serde_json::json!({
                        "id": entity.id.to_string(),
                        "message": msg_entity.to_message(),
                        "pinnedBy": entity.pinned_by.to_string(),
                        "pinnedAt": entity.pinned_at.to_rfc3339(),
                    }));
                }
            }

            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "pinnedMessages": items,
                    "total": items.len(),
                }))),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("FETCH_PINNED_FAILED", format!("获取置顶消息失败: {}", e))),
        ),
    }
}

/// 批量发送消息
#[utoipa::path(
    post,
    path = "/api/v1/messages/batch/send",
    tag = "消息",
    request_body = BatchSendMessageRequest,
    responses(
        (status = 201, description = "批量发送成功", body = ApiResponse<BatchOperationResult>),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "未授权"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn batch_send_messages(
    Extension(pool): Extension<PgPool>,
    Extension(user_id): Extension<String>,
    Json(req): Json<crate::models::message::BatchSendMessageRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let conversation_id = match Uuid::parse_str(&req.conversation_id) {
        Ok(id) => id,
        Err(_) => return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_CONVERSATION_ID", "无效的会话ID")),
        ),
    };

    let sender_id = match Uuid::parse_str(&user_id) {
        Ok(id) => id,
        Err(_) => return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("INVALID_USER", "无效的用户身份")),
        ),
    };

    if req.messages.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("EMPTY_BATCH", "批量消息不能为空")),
        );
    }

    if req.messages.len() > 100 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("BATCH_TOO_LARGE", "单次批量发送最多100条消息")),
        );
    }

    let total = req.messages.len();
    let mut errors = Vec::new();
    let mut params_list = Vec::new();

    for (idx, msg_req) in req.messages.into_iter().enumerate() {
        // 内容非空校验
        if msg_req.content.trim().is_empty() {
            errors.push(serde_json::json!({"index": idx, "error": "消息内容不能为空"}));
            continue;
        }

        let msg_type = msg_req.type_;

        let reply_to = msg_req.reply_to.as_ref().and_then(|r| Uuid::parse_str(r).ok());

        params_list.push(crate::models::message::CreateMessageParams {
            conversation_id,
            sender_id,
            content: msg_req.content,
            type_: msg_type,
            reply_to,
            metadata: None,
        });
    }

    let success_count = params_list.len();

    match crate::db::message::batch_create_messages(&pool, params_list).await {
        Ok(_messages) => {
            tracing::info!("批量发送 {} 条消息到会话 {}", success_count, conversation_id);
            (
                StatusCode::CREATED,
                Json(ApiResponse::success(serde_json::json!({
                    "total": total,
                    "success": success_count,
                    "failed": errors.len(),
                    "errors": errors,
                }))),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("BATCH_SEND_FAILED", format!("批量发送消息失败: {}", e))),
        ),
    }
}

/// 批量删除消息
#[utoipa::path(
    post,
    path = "/api/v1/messages/batch/delete",
    tag = "消息",
    request_body = BatchDeleteMessagesRequest,
    responses(
        (status = 200, description = "批量删除成功", body = ApiResponse<BatchOperationResult>),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "未授权"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn batch_delete_messages(
    Extension(pool): Extension<PgPool>,
    Extension(user_id): Extension<String>,
    Json(req): Json<crate::models::message::BatchDeleteMessagesRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match Uuid::parse_str(&user_id) {
        Ok(id) => id,
        Err(_) => return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("INVALID_USER", "无效的用户身份")),
        ),
    };

    if req.message_ids.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("EMPTY_BATCH", "批量删除列表不能为空")),
        );
    }

    if req.message_ids.len() > 200 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("BATCH_TOO_LARGE", "单次批量删除最多200条消息")),
        );
    }

    let mut message_uuids = Vec::new();
    let mut errors = Vec::new();
    for (idx, id_str) in req.message_ids.iter().enumerate() {
        match Uuid::parse_str(id_str) {
            Ok(uuid) => message_uuids.push(uuid),
            Err(_) => errors.push(serde_json::json!({"index": idx, "id": id_str, "error": "无效的消息ID"})),
        }
    }

    let total = message_uuids.len();
    match crate::db::message::batch_delete_messages(&pool, &message_uuids, user_uuid).await {
        Ok(deleted_count) => {
            tracing::info!("批量删除 {} 条消息", deleted_count);
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "total": total,
                    "success": deleted_count,
                    "failed": errors.len() + (total - deleted_count as usize),
                    "errors": errors,
                }))),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("BATCH_DELETE_FAILED", format!("批量删除消息失败: {}", e))),
        ),
    }
}

/// 批量标记会话已读
#[utoipa::path(
    post,
    path = "/api/v1/messages/batch/mark-read",
    tag = "消息",
    request_body = BatchMarkReadRequest,
    responses(
        (status = 200, description = "批量标记已读成功", body = ApiResponse<BatchOperationResult>),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "未授权"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn batch_mark_as_read(
    Extension(pool): Extension<PgPool>,
    Extension(user_id): Extension<String>,
    Json(req): Json<crate::models::message::BatchMarkReadRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match Uuid::parse_str(&user_id) {
        Ok(id) => id,
        Err(_) => return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("INVALID_USER", "无效的用户身份")),
        ),
    };

    if req.conversation_ids.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("EMPTY_BATCH", "会话列表不能为空")),
        );
    }

    let mut conv_uuids = Vec::new();
    let mut errors = Vec::new();
    for (idx, id_str) in req.conversation_ids.iter().enumerate() {
        match Uuid::parse_str(id_str) {
            Ok(uuid) => conv_uuids.push(uuid),
            Err(_) => errors.push(serde_json::json!({"index": idx, "id": id_str, "error": "无效的会话ID"})),
        }
    }

    let total = conv_uuids.len();
    match crate::db::message::batch_mark_conversations_as_read(&pool, &conv_uuids, user_uuid).await {
        Ok(marked_count) => {
            tracing::info!("批量标记 {} 个会话已读，共 {} 条消息", total, marked_count);
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "conversationsTotal": total,
                    "messagesMarked": marked_count,
                    "errors": errors,
                }))),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("BATCH_READ_FAILED", format!("批量标记已读失败: {}", e))),
        ),
    }
}

// === 消息收藏/书签处理 ===

use crate::models::message::{AddBookmarkRequest, BookmarkQuery};
use crate::db::message::{add_bookmark, remove_bookmark, get_bookmarks, is_bookmarked};

/// 收藏消息
pub async fn add_bookmark_handler(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(message_id): Path<String>,
    Json(req): Json<AddBookmarkRequest>,
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

    // 检查消息是否存在
    let message = match sqlx::query_as::<_, MessageEntity>(
        "SELECT * FROM messages WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(msg_uuid)
    .fetch_optional(&pool)
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
    .fetch_one(&pool)
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

    match add_bookmark(&pool, &user_uuid, &msg_uuid, req.note.as_deref()).await {
        Ok(bookmark) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "id": bookmark.id.to_string(),
                "messageId": bookmark.message_id.to_string(),
                "note": bookmark.note,
                "createdAt": bookmark.created_at.to_rfc3339(),
            }))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("BOOKMARK_FAILED", format!("收藏失败: {}", e))),
        ),
    }
}

/// 取消收藏消息
pub async fn remove_bookmark_handler(
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

    let user_uuid = match auth.user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID")),
            );
        }
    };

    match remove_bookmark(&pool, &user_uuid, &msg_uuid).await {
        Ok(removed) => {
            if removed {
                (
                    StatusCode::OK,
                    Json(ApiResponse::success(serde_json::json!({
                        "message": "已取消收藏"
                    }))),
                )
            } else {
                (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::error("NOT_FOUND", "未找到该收藏")),
                )
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("REMOVE_BOOKMARK_FAILED", format!("取消收藏失败: {}", e))),
        ),
    }
}

/// 获取用户收藏列表
pub async fn get_bookmarks_handler(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Query(query): Query<BookmarkQuery>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match auth.user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID")),
            );
        }
    };

    match get_bookmarks(&pool, &user_uuid, query.page, query.limit).await {
        Ok(bookmarks) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "bookmarks": bookmarks,
                "page": query.page,
                "limit": query.limit,
            }))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("GET_BOOKMARKS_FAILED", format!("获取收藏列表失败: {}", e))),
        ),
    }
}

/// 检查消息收藏状态
pub async fn check_bookmark_handler(
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

    let user_uuid = match auth.user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID")),
            );
        }
    };

    match is_bookmarked(&pool, &user_uuid, &msg_uuid).await {
        Ok(bookmarked) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "bookmarked": bookmarked,
            }))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("CHECK_FAILED", format!("检查收藏状态失败: {}", e))),
        ),
    }
}

// === 草稿消息处理 ===

use crate::models::message::{SaveDraftRequest, DraftQuery};
use crate::db::message::{save_draft, get_draft, delete_draft, get_all_drafts};

/// 保存草稿
pub async fn save_draft_handler(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
    Json(req): Json<SaveDraftRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let conv_uuid = match conversation_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的会话ID")),
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

    // 检查用户是否在会话中
    let is_member = match sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM conversation_members WHERE conversation_id = $1 AND user_id = $2)"
    )
    .bind(conv_uuid)
    .bind(user_uuid)
    .fetch_one(&pool)
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

    let reply_to_uuid = req.reply_to.as_deref().and_then(|s| s.parse::<Uuid>().ok());

    match save_draft(
        &pool,
        &user_uuid,
        &conv_uuid,
        &req.content,
        &req.type_,
        reply_to_uuid.as_ref(),
        req.metadata.as_ref(),
    ).await {
        Ok(draft) => {
            let info = draft.to_draft_info();
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::to_value(info).unwrap())),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SAVE_FAILED", format!("保存草稿失败: {}", e))),
        ),
    }
}

/// 获取指定会话的草稿
pub async fn get_draft_handler(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let conv_uuid = match conversation_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的会话ID")),
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

    match get_draft(&pool, &user_uuid, &conv_uuid).await {
        Ok(Some(draft)) => {
            let info = draft.to_draft_info();
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::to_value(info).unwrap())),
            )
        }
        Ok(None) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!(null))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("GET_FAILED", format!("获取草稿失败: {}", e))),
        ),
    }
}

/// 删除指定会话的草稿
pub async fn delete_draft_handler(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let conv_uuid = match conversation_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的会话ID")),
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

    match delete_draft(&pool, &user_uuid, &conv_uuid).await {
        Ok(deleted) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({ "deleted": deleted }))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("DELETE_FAILED", format!("删除草稿失败: {}", e))),
        ),
    }
}

/// 获取用户的所有草稿列表
pub async fn get_all_drafts_handler(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Query(query): Query<DraftQuery>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match auth.user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID")),
            );
        }
    };

    match get_all_drafts(&pool, &user_uuid, query.page, query.limit).await {
        Ok(drafts) => {
            let draft_infos: Vec<_> = drafts.iter().map(|d| d.to_draft_info()).collect();
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "drafts": draft_infos,
                    "page": query.page,
                    "limit": query.limit,
                }))),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("LIST_FAILED", format!("获取草稿列表失败: {}", e))),
        ),
    }
}

/// 创建定时消息
pub async fn create_scheduled_message_handler(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<CreateScheduledMessageRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match auth.user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID")),
            );
        }
    };

    let conversation_uuid = match req.conversation_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_CONVERSATION_ID", "无效的会话ID")),
            );
        }
    };

    let reply_to_uuid = req.reply_to.and_then(|id| id.parse::<Uuid>().ok());

    // 验证定时时间必须在未来
    if req.scheduled_at <= Utc::now() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_SCHEDULE_TIME", "定时发送时间必须在未来")),
        );
    }

    match create_scheduled_message(
        &pool,
        &user_uuid,
        &conversation_uuid,
        &req.content,
        &req.type_.to_string(),
        reply_to_uuid.as_ref(),
        req.metadata.as_ref(),
        req.scheduled_at,
    )
    .await
    {
        Ok(message) => (
            StatusCode::CREATED,
            Json(ApiResponse::success(serde_json::json!(message.to_info()))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("CREATE_FAILED", format!("创建定时消息失败: {}", e))),
        ),
    }
}

/// 获取定时消息详情
pub async fn get_scheduled_message_handler(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(message_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match auth.user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID")),
            );
        }
    };

    let msg_uuid = match message_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_MESSAGE_ID", "无效的消息ID")),
            );
        }
    };

    match get_scheduled_message(&pool, &msg_uuid, &user_uuid).await {
        Ok(Some(message)) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!(message.to_info()))),
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "定时消息不存在")),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("GET_FAILED", format!("获取定时消息失败: {}", e))),
        ),
    }
}

/// 更新定时消息
pub async fn update_scheduled_message_handler(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(message_id): Path<String>,
    Json(req): Json<UpdateScheduledMessageRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match auth.user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID")),
            );
        }
    };

    let msg_uuid = match message_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_MESSAGE_ID", "无效的消息ID")),
            );
        }
    };

    let reply_to_uuid = req.reply_to.and_then(|id| id.parse::<Uuid>().ok());
    let type_str = req.type_.map(|t| t.to_string());

    match update_scheduled_message(
        &pool,
        &msg_uuid,
        &user_uuid,
        req.content.as_deref(),
        type_str.as_deref(),
        reply_to_uuid.as_ref(),
        req.metadata.as_ref(),
        req.scheduled_at,
    )
    .await
    {
        Ok(message) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!(message.to_info()))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("UPDATE_FAILED", format!("更新定时消息失败: {}", e))),
        ),
    }
}

/// 取消定时消息
pub async fn cancel_scheduled_message_handler(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(message_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match auth.user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID")),
            );
        }
    };

    let msg_uuid = match message_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_MESSAGE_ID", "无效的消息ID")),
            );
        }
    };

    match cancel_scheduled_message(&pool, &msg_uuid, &user_uuid).await {
        Ok(true) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({"message": "定时消息已取消"}))),
        ),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "定时消息不存在或已发送")),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("CANCEL_FAILED", format!("取消定时消息失败: {}", e))),
        ),
    }
}

/// 获取定时消息列表
pub async fn get_scheduled_messages_handler(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Query(query): Query<ScheduledMessageQuery>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match auth.user_id.parse::<Uuid>() {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID")),
            );
        }
    };

    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);

    match get_scheduled_messages(&pool, &user_uuid, query.status.as_deref(), page, limit).await {
        Ok(messages) => {
            let message_infos: Vec<_> = messages.iter().map(|m| m.to_info()).collect();
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "scheduled_messages": message_infos,
                    "page": page,
                    "limit": limit,
                }))),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("LIST_FAILED", format!("获取定时消息列表失败: {}", e))),
        ),
    }
}
use crate::models::message::{CreateScheduledMessageRequest, UpdateScheduledMessageRequest, ScheduledMessageQuery};
use crate::db::message::{create_scheduled_message, get_scheduled_message, update_scheduled_message, cancel_scheduled_message, get_scheduled_messages};
