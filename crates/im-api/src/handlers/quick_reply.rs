//! 快捷回复模板 API Handler

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::quick_reply::{
    create_global_quick_reply, create_quick_reply, delete_quick_reply, get_quick_reply_by_id,
    get_user_quick_replies, update_quick_reply,
};
use crate::models::auth::ApiResponse;
use crate::models::quick_reply::{CreateQuickReplyRequest, QuickReply, UpdateQuickReplyRequest};

/// 查询参数
#[derive(Debug, Deserialize)]
pub struct QuickReplyQuery {
    pub category: Option<String>,
}

/// 创建快捷回复
pub async fn create_quick_reply_handler(
    State(pool): State<PgPool>,
    auth: crate::middleware::auth::AuthUser,
    Json(req): Json<CreateQuickReplyRequest>,
) -> Result<(StatusCode, Json<ApiResponse<QuickReply>>), (StatusCode, Json<ApiResponse<()>>)> {
    if req.title.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", "标题不能为空".to_string())),
        ));
    }
    if req.content.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", "内容不能为空".to_string())),
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

    match create_quick_reply(&pool, user_id, req).await {
        Ok(entity) => Ok((
            StatusCode::CREATED,
            Json(ApiResponse::success(entity.to_quick_reply())),
        )),
        Err(e) => {
            eprintln!("创建快捷回复失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("CREATE_FAILED", "创建快捷回复失败".to_string())),
            ))
        }
    }
}

/// 获取快捷回复列表
pub async fn get_quick_replies_handler(
    State(pool): State<PgPool>,
    auth: crate::middleware::auth::AuthUser,
    Query(params): Query<QuickReplyQuery>,
) -> Result<Json<ApiResponse<Vec<QuickReply>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID".to_string())),
            ))
        }
    };

    match get_user_quick_replies(&pool, user_id, params.category.as_deref()).await {
        Ok(entities) => {
            let replies: Vec<QuickReply> = entities.iter().map(|e| e.to_quick_reply()).collect();
            Ok(Json(ApiResponse::success(replies)))
        }
        Err(e) => {
            eprintln!("获取快捷回复列表失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("GET_FAILED", "获取快捷回复列表失败".to_string())),
            ))
        }
    }
}

/// 获取单个快捷回复
pub async fn get_quick_reply_handler(
    State(pool): State<PgPool>,
    auth: crate::middleware::auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<QuickReply>>, (StatusCode, Json<ApiResponse<()>>)> {
    let reply_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的快捷回复ID".to_string())),
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

    match get_quick_reply_by_id(&pool, reply_id).await {
        Ok(Some(entity)) => {
            // 检查权限：必须是拥有者或全局模板
            if entity.user_id != user_id && !entity.is_global {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ApiResponse::error("FORBIDDEN", "无权访问此快捷回复".to_string())),
                ));
            }
            Ok(Json(ApiResponse::success(entity.to_quick_reply())))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "快捷回复不存在".to_string())),
        )),
        Err(e) => {
            eprintln!("获取快捷回复失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("GET_FAILED", "获取快捷回复失败".to_string())),
            ))
        }
    }
}

/// 更新快捷回复
pub async fn update_quick_reply_handler(
    State(pool): State<PgPool>,
    auth: crate::middleware::auth::AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateQuickReplyRequest>,
) -> Result<Json<ApiResponse<QuickReply>>, (StatusCode, Json<ApiResponse<()>>)> {
    let reply_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的快捷回复ID".to_string())),
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

    match update_quick_reply(&pool, reply_id, user_id, req).await {
        Ok(Some(entity)) => Ok(Json(ApiResponse::success(entity.to_quick_reply()))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "快捷回复不存在或无权修改".to_string())),
        )),
        Err(e) => {
            eprintln!("更新快捷回复失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("UPDATE_FAILED", "更新快捷回复失败".to_string())),
            ))
        }
    }
}

/// 删除快捷回复
pub async fn delete_quick_reply_handler(
    State(pool): State<PgPool>,
    auth: crate::middleware::auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let reply_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的快捷回复ID".to_string())),
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

    match delete_quick_reply(&pool, reply_id, user_id).await {
        Ok(true) => Ok(Json(ApiResponse::success(serde_json::json!({
            "message": "快捷回复已删除"
        })))),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "快捷回复不存在或无权删除".to_string())),
        )),
        Err(e) => {
            eprintln!("删除快捷回复失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("DELETE_FAILED", "删除快捷回复失败".to_string())),
            ))
        }
    }
}

/// 创建全局快捷回复（管理员）
pub async fn create_global_quick_reply_handler(
    State(pool): State<PgPool>,
    auth: crate::middleware::auth::AuthUser,
    Json(req): Json<CreateQuickReplyRequest>,
) -> Result<(StatusCode, Json<ApiResponse<QuickReply>>), (StatusCode, Json<ApiResponse<()>>)> {
    if req.title.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", "标题不能为空".to_string())),
        ));
    }
    if req.content.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", "内容不能为空".to_string())),
        ));
    }

    let admin_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID".to_string())),
            ))
        }
    };

    match create_global_quick_reply(&pool, admin_id, req).await {
        Ok(entity) => Ok((
            StatusCode::CREATED,
            Json(ApiResponse::success(entity.to_quick_reply())),
        )),
        Err(e) => {
            eprintln!("创建全局快捷回复失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("CREATE_FAILED", "创建全局快捷回复失败".to_string())),
            ))
        }
    }
}
