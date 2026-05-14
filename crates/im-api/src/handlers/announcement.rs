//! 系统公告 API Handler

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::announcement::{
    create_announcement, delete_announcement, get_active_announcements, get_all_announcements,
    get_announcement_by_id, get_unread_announcement_count, mark_announcement_read,
    update_announcement,
};
use crate::models::announcement::{Announcement, CreateAnnouncementRequest};
use crate::models::auth::ApiResponse;

/// 分页查询参数
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 创建系统公告（管理员）
#[utoipa::path(
    post,
    path = "/api/announcements",
    tag = "announcements",
    responses(
        (status = 201, description = "创建成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn create_announcement_handler(
    State(pool): State<PgPool>,
    auth: crate::middleware::auth::AuthUser,
    Json(req): Json<CreateAnnouncementRequest>,
) -> Result<(StatusCode, Json<ApiResponse<Announcement>>), (StatusCode, Json<ApiResponse<()>>)> {
    // 验证标题和内容不为空
    if req.title.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", "公告标题不能为空".to_string())),
        ));
    }
    if req.content.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", "公告内容不能为空".to_string())),
        ));
    }

    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID".to_string())),
            ))
        }
    };

    match create_announcement(&pool, req, user_id).await {
        Ok(entity) => {
            let announcement = entity.to_announcement();
            Ok((
                StatusCode::CREATED,
                Json(ApiResponse::success(announcement)),
            ))
        }
        Err(e) => {
            eprintln!("创建公告失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("CREATE_FAILED", "创建公告失败".to_string())),
            ))
        }
    }
}

/// 获取公告列表（管理员视图）
#[utoipa::path(
    get,
    path = "/api/announcements/all",
    tag = "announcements",
    responses(
        (status = 200, description = "获取成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn get_all_announcements_handler(
    State(pool): State<PgPool>,
    _auth: crate::middleware::auth::AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<ApiResponse<Vec<Announcement>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    match get_all_announcements(&pool, page, page_size).await {
        Ok(entities) => {
            let announcements: Vec<Announcement> = entities.iter().map(|e| e.to_announcement()).collect();
            Ok(Json(ApiResponse::success(announcements)))
        }
        Err(e) => {
            eprintln!("获取公告列表失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("GET_FAILED", "获取公告列表失败".to_string())),
            ))
        }
    }
}

/// 获取活跃公告列表（用户视图，含已读状态）
#[utoipa::path(
    get,
    path = "/api/announcements",
    tag = "announcements",
    responses(
        (status = 200, description = "获取成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn get_active_announcements_handler(
    State(pool): State<PgPool>,
    auth: crate::middleware::auth::AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<ApiResponse<Vec<Announcement>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID".to_string())),
            ))
        }
    };

    match get_active_announcements(&pool, user_id, page, page_size).await {
        Ok(rows) => {
            let announcements: Vec<Announcement> = rows
                .iter()
                .map(|row| row.to_announcement())
                .collect();
            Ok(Json(ApiResponse::success(announcements)))
        }
        Err(e) => {
            eprintln!("获取活跃公告失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("GET_FAILED", "获取公告列表失败".to_string())),
            ))
        }
    }
}

/// 获取单个公告详情
pub async fn get_announcement_handler(
    State(pool): State<PgPool>,
    auth: crate::middleware::auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Announcement>>, (StatusCode, Json<ApiResponse<()>>)> {
    let announcement_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的公告ID".to_string())),
            ))
        }
    };

    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID".to_string())),
            ))
        }
    };

    match get_announcement_by_id(&pool, announcement_id).await {
        Ok(Some(entity)) => {
            // 检查已读状态
            let is_read = crate::db::announcement::is_announcement_read(
                &pool,
                announcement_id,
                user_id,
            )
            .await
            .unwrap_or(false);

            let announcement = if is_read {
                entity.to_announcement_with_read(true, None)
            } else {
                entity.to_announcement()
            };

            Ok(Json(ApiResponse::success(announcement)))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "公告不存在".to_string())),
        )),
        Err(e) => {
            eprintln!("获取公告详情失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("GET_FAILED", "获取公告详情失败".to_string())),
            ))
        }
    }
}

/// 标记公告为已读
#[utoipa::path(
    post,
    path = "/api/announcements/{id}/read",
    tag = "announcements",
    params(("id" = String, Path, description = "公告ID")),
    responses(
        (status = 200, description = "标记成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn mark_announcement_read_handler(
    State(pool): State<PgPool>,
    auth: crate::middleware::auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let announcement_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的公告ID".to_string())),
            ))
        }
    };

    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID".to_string())),
            ))
        }
    };

    match mark_announcement_read(&pool, announcement_id, user_id).await {
        Ok(record) => Ok(Json(ApiResponse::success(serde_json::json!({
            "announcement_id": record.announcement_id.to_string(),
            "user_id": record.user_id.to_string(),
            "read_at": record.read_at.to_rfc3339(),
            "message": "已标记为已读"
        })))),
        Err(e) => {
            eprintln!("标记公告已读失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("UPDATE_FAILED", "标记已读失败".to_string())),
            ))
        }
    }
}

/// 更新公告（管理员）
pub async fn update_announcement_handler(
    State(pool): State<PgPool>,
    _auth: crate::middleware::auth::AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateAnnouncementRequest>,
) -> Result<Json<ApiResponse<Announcement>>, (StatusCode, Json<ApiResponse<()>>)> {
    let announcement_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的公告ID".to_string())),
            ))
        }
    };

    match update_announcement(
        &pool,
        announcement_id,
        req.title,
        req.content,
        req.type_,
        req.priority,
        req.is_active,
        req.expires_at,
    )
    .await
    {
        Ok(entity) => {
            let announcement = entity.to_announcement();
            Ok(Json(ApiResponse::success(announcement)))
        }
        Err(e) => {
            eprintln!("更新公告失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("UPDATE_FAILED", "更新公告失败".to_string())),
            ))
        }
    }
}

/// 删除公告（管理员）
pub async fn delete_announcement_handler(
    State(pool): State<PgPool>,
    _auth: crate::middleware::auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let announcement_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的公告ID".to_string())),
            ))
        }
    };

    match delete_announcement(&pool, announcement_id).await {
        Ok(true) => Ok(Json(ApiResponse::success(serde_json::json!({
            "message": "公告已删除"
        })))),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "公告不存在".to_string())),
        )),
        Err(e) => {
            eprintln!("删除公告失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("DELETE_FAILED", "删除公告失败".to_string())),
            ))
        }
    }
}

/// 获取未读公告数量
pub async fn get_unread_announcement_count_handler(
    State(pool): State<PgPool>,
    auth: crate::middleware::auth::AuthUser,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID".to_string())),
            ))
        }
    };

    match get_unread_announcement_count(&pool, user_id).await {
        Ok(count) => Ok(Json(ApiResponse::success(serde_json::json!({
            "unread_count": count
        })))),
        Err(e) => {
            eprintln!("获取未读公告数量失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("GET_FAILED", "获取未读数量失败".to_string())),
            ))
        }
    }
}

/// 更新公告请求
#[derive(Debug, Deserialize)]
pub struct UpdateAnnouncementRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub priority: Option<i32>,
    pub is_active: Option<bool>,
    pub expires_at: Option<Option<String>>,
}
