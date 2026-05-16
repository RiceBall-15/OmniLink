//! 用户与会话成员管理 API 处理器

use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::middleware::AuthUser;
use crate::conversation_service::ConversationService;
use crate::user_repository::UserRepository;
use common::ApiResponse;

/// 用户搜索请求
#[derive(Debug, Deserialize)]
pub struct UserSearchQuery {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 { 20 }

/// 用户搜索结果
#[derive(Debug, Serialize)]
pub struct UserSearchResult {
    pub users: Vec<UserInfo>,
    pub total: i64,
}

/// 用户简要信息
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
}

/// 添加成员请求
#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub user_id: Uuid,
}

/// 移除成员请求
#[derive(Debug, Deserialize)]
pub struct RemoveMemberRequest {
    pub user_id: Uuid,
}

/// 成员信息
#[derive(Debug, Serialize)]
pub struct MemberInfo {
    pub user_id: Uuid,
    pub username: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub joined_at: i64,
}

/// 搜索用户
pub async fn search_users(
    State(user_repo): State<Arc<UserRepository>>,
    Query(query): Query<UserSearchQuery>,
    _auth: AuthUser,
) -> impl IntoResponse {
    if query.q.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("搜索关键词不能为空".to_string())),
        );
    }

    let limit = query.limit.min(50); // 最多50条
    let offset = query.offset.max(0);

    match user_repo.search(&query.q, limit, offset).await {
        Ok(users) => {
            let total = match user_repo.count().await {
                Ok(c) => c,
                Err(_) => 0,
            };
            let user_infos: Vec<UserInfo> = users.into_iter().map(|u| UserInfo {
                id: u.id,
                username: u.username,
                email: u.email,
                avatar_url: u.avatar_url,
                bio: u.bio,
            }).collect();

            (
                StatusCode::OK,
                Json(ApiResponse::success(UserSearchResult {
                    users: user_infos,
                    total,
                })),
            )
        }
        Err(e) => {
            tracing::error!("Failed to search users: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error("搜索用户失败".to_string())),
            )
        }
    }
}

/// 添加会话成员
pub async fn add_conversation_member(
    State(conv_service): State<Arc<ConversationService>>,
    Path(conversation_id): Path<Uuid>,
    auth: AuthUser,
    Json(req): Json<AddMemberRequest>,
) -> impl IntoResponse {
    let operator_id = auth.user_id;

    match conv_service.add_member(conversation_id, req.user_id, operator_id).await {
        Ok(participant) => {
            (
                StatusCode::OK,
                Json(ApiResponse::success(MemberInfo {
                    user_id: participant.user_id,
                    username: participant.username,
                    avatar_url: participant.avatar_url,
                    role: participant.role,
                    joined_at: participant.joined_at,
                })),
            )
        }
        Err(e) => {
            let (status, msg) = match &e {
                common::AppError::Authorization(m) => (StatusCode::FORBIDDEN, m.clone()),
                common::AppError::Validation(m) => (StatusCode::CONFLICT, m.clone()),
                common::AppError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
                _ => {
                    tracing::error!("Failed to add member: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "添加成员失败".to_string())
                }
            };
            (status, Json(ApiResponse::<()>::error(msg)))
        }
    }
}

/// 移除会话成员
pub async fn remove_conversation_member(
    State(conv_service): State<Arc<ConversationService>>,
    Path((conversation_id, user_id)): Path<(Uuid, Uuid)>,
    auth: AuthUser,
) -> impl IntoResponse {
    let operator_id = auth.user_id;

    match conv_service.remove_member(conversation_id, user_id, operator_id).await {
        Ok(()) => {
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "message": "成员已移除"
                }))),
            )
        }
        Err(e) => {
            let (status, msg) = match &e {
                common::AppError::Authorization(m) => (StatusCode::FORBIDDEN, m.clone()),
                common::AppError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
                _ => {
                    tracing::error!("Failed to remove member: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "移除成员失败".to_string())
                }
            };
            (status, Json(ApiResponse::<()>::error(msg)))
        }
    }
}

/// 获取会话成员列表
pub async fn get_conversation_members(
    State(conv_service): State<Arc<ConversationService>>,
    Path(conversation_id): Path<Uuid>,
    auth: AuthUser,
) -> impl IntoResponse {
    match conv_service.get_members(conversation_id).await {
        Ok(members) => {
            let member_infos: Vec<MemberInfo> = members.into_iter().map(|m| MemberInfo {
                user_id: m.user_id,
                username: m.username,
                avatar_url: m.avatar_url,
                role: m.role,
                joined_at: m.joined_at,
            }).collect();
            (
                StatusCode::OK,
                Json(ApiResponse::success(member_infos)),
            )
        }
        Err(e) => {
            tracing::error!("Failed to get members: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error("获取成员列表失败".to_string())),
            )
        }
    }
}
