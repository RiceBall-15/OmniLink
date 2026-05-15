//! 联系人管理处理器模块
//!
//! 提供联系人管理相关的 API 端点：
//! - `POST /api/users/contacts` - 添加联系人
//! - `DELETE /api/users/contacts/:id` - 删除联系人
//! - `GET /api/users/contacts` - 获取联系人列表
//! - `PUT /api/users/contacts/:id` - 更新联系人备注
//! - `GET /api/users/search` - 搜索用户

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;
use serde::Deserialize;
use sqlx::PgPool;

use crate::models::auth::{
    ApiResponse, AddContactRequest, UpdateContactRequest, ContactListResponse,
    UserSearchResult, Contact,
};
use crate::db::contact::{
    add_contact, remove_contact, get_contacts, count_contacts, update_contact_nickname,
    search_users, get_contact_by_id,
};
use crate::middleware::auth::AuthUser;

/// 安全序列化为 JSON Value
fn to_json_value<T: serde::Serialize>(value: &T) -> Result<serde_json::Value, (StatusCode, Json<ApiResponse<serde_json::Value>>)> {
    serde_json::to_value(value).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERIALIZATION_FAILED", format!("数据序列化失败: {}", e))),
        )
    })
}



/// 联系人查询参数
#[derive(Debug, Deserialize)]
pub struct ContactQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_page() -> i64 { 1 }
fn default_limit() -> i64 { 20 }

/// 用户搜索查询参数
#[derive(Debug, Deserialize)]
pub struct UserSearchQuery {
    pub q: String,
    #[serde(default = "default_search_limit")]
    pub limit: i64,
}

fn default_search_limit() -> i64 { 20 }

/// 添加联系人
///
/// POST /api/users/contacts
#[utoipa::path(
    post,
    path = "/api/users/contacts",
    tag = "contacts",
    responses(
        (status = 201, description = "添加成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn add_contact_handler(
    State(pool): State<PgPool>,
    AuthUser { user_id, .. }: AuthUser,
    Json(request): Json<AddContactRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match Uuid::parse_str(&user_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID")),
            );
        }
    };

    let contact_uuid = match Uuid::parse_str(&request.contact_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_CONTACT_ID", "无效的联系人ID")),
            );
        }
    };

    match add_contact(&pool, &user_uuid, &contact_uuid, request.nickname).await {
        Ok(contact) => (
            StatusCode::CREATED,
            match to_json_value(&contact.to_contact()) {
                    Ok(v) => Json(ApiResponse::success(v)),
                    Err(e) => return e,
                },
        ),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("不能添加自己") {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error("SELF_CONTACT", &msg)),
                )
            } else if msg.contains("用户不存在") {
                (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::error("USER_NOT_FOUND", &msg)),
                )
            } else if msg.contains("已经是您的联系人") {
                (
                    StatusCode::CONFLICT,
                    Json(ApiResponse::error("ALREADY_CONTACT", &msg)),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("ADD_CONTACT_FAILED", format!("添加联系人失败: {}", e))),
                )
            }
        }
    }
}

/// 删除联系人
///
/// DELETE /api/users/contacts/:id
pub async fn remove_contact_handler(
    State(pool): State<PgPool>,
    AuthUser { user_id, .. }: AuthUser,
    Path(contact_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match Uuid::parse_str(&user_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID")),
            );
        }
    };

    let contact_uuid = match Uuid::parse_str(&contact_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_CONTACT_ID", "无效的联系人ID")),
            );
        }
    };

    match remove_contact(&pool, &user_uuid, &contact_uuid).await {
        Ok(true) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({"message": "联系人已删除"}))),
        ),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("CONTACT_NOT_FOUND", "联系人不存在")),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("REMOVE_CONTACT_FAILED", format!("删除联系人失败: {}", e))),
        ),
    }
}

/// 获取联系人列表
///
/// GET /api/users/contacts
#[utoipa::path(
    get,
    path = "/api/users/contacts",
    tag = "contacts",
    responses(
        (status = 200, description = "获取成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn get_contacts_handler(
    State(pool): State<PgPool>,
    AuthUser { user_id, .. }: AuthUser,
    Query(query): Query<ContactQuery>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match Uuid::parse_str(&user_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID")),
            );
        }
    };

    let page = query.page.max(1);
    let limit = query.limit.clamp(1, 100);

    match get_contacts(&pool, &user_uuid, page, limit).await {
        Ok(contacts) => {
            let total = count_contacts(&pool, &user_uuid).await.unwrap_or(0);
            let contact_list: Vec<Contact> = contacts.iter().map(|c| c.to_contact()).collect();

            match serde_json::to_value(ContactListResponse {
                contacts: contact_list,
                total,
            }) {
                Ok(v) => (
                    StatusCode::OK,
                    Json(ApiResponse::success(v)),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("SERIALIZATION_FAILED", format!("数据序列化失败: {}", e))),
                ),
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("GET_CONTACTS_FAILED", format!("获取联系人列表失败: {}", e))),
        ),
    }
}

/// 更新联系人备注
///
/// PUT /api/users/contacts/:id
pub async fn update_contact_handler(
    State(pool): State<PgPool>,
    AuthUser { user_id, .. }: AuthUser,
    Path(contact_id): Path<String>,
    Json(request): Json<UpdateContactRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match Uuid::parse_str(&user_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID")),
            );
        }
    };

    let contact_uuid = match Uuid::parse_str(&contact_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_CONTACT_ID", "无效的联系人ID")),
            );
        }
    };

    match update_contact_nickname(&pool, &user_uuid, &contact_uuid, &request.nickname).await {
        Ok(contact) => (
            StatusCode::OK,
            match to_json_value(&contact.to_contact()) {
                    Ok(v) => Json(ApiResponse::success(v)),
                    Err(e) => return e,
                },
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("UPDATE_CONTACT_FAILED", format!("更新联系人备注失败: {}", e))),
        ),
    }
}

/// 搜索用户
///
/// GET /api/users/search?q=keyword
#[utoipa::path(
    get,
    path = "/api/users/search",
    tag = "contacts",
    params(("q" = String, Query, description = "搜索关键词")),
    responses(
        (status = 200, description = "搜索成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn search_users_handler(
    State(pool): State<PgPool>,
    AuthUser { user_id, .. }: AuthUser,
    Query(query): Query<UserSearchQuery>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match Uuid::parse_str(&user_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID")),
            );
        }
    };

    if query.q.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("EMPTY_QUERY", "搜索关键词不能为空")),
        );
    }

    let limit = query.limit.clamp(1, 50);

    match search_users(&pool, &query.q, &user_uuid, limit).await {
        Ok(users) => {
            let results: Vec<UserSearchResult> = users.into_iter().map(|(user, is_contact)| {
                UserSearchResult {
                    id: user.id.to_string(),
                    username: user.username,
                    nickname: user.nickname,
                    avatar: user.avatar,
                    bio: user.bio,
                    is_contact,
                }
            }).collect();

            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "users": results,
                    "total": results.len(),
                }))),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SEARCH_FAILED", format!("搜索用户失败: {}", e))),
        ),
    }
}

/// 获取单个联系人详情
///
/// GET /api/users/contacts/:id
pub async fn get_contact_handler(
    State(pool): State<PgPool>,
    AuthUser { user_id, .. }: AuthUser,
    Path(contact_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_uuid = match Uuid::parse_str(&user_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID")),
            );
        }
    };

    let contact_uuid = match Uuid::parse_str(&contact_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_CONTACT_ID", "无效的联系人ID")),
            );
        }
    };

    match get_contact_by_id(&pool, &user_uuid, &contact_uuid).await {
        Ok(Some(contact)) => (
            StatusCode::OK,
            match to_json_value(&contact.to_contact()) {
                    Ok(v) => Json(ApiResponse::success(v)),
                    Err(e) => return e,
                },
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("CONTACT_NOT_FOUND", "联系人不存在")),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("GET_CONTACT_FAILED", format!("获取联系人详情失败: {}", e))),
        ),
    }
}
