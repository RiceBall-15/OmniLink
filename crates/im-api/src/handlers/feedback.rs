//! 用户反馈系统 API Handler

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::feedback::{
    create_feedback, delete_feedback, get_all_feedbacks, get_feedback_by_id, get_feedback_stats,
    get_user_feedbacks, update_feedback,
};
use crate::models::auth::ApiResponse;
use crate::models::feedback::{
    CreateFeedbackRequest, FeedbackQuery, FeedbackStats, FeedbackTypeStats, UpdateFeedbackRequest,
    UserFeedback,
};

/// 提交反馈
#[utoipa::path(
    post,
    path = "/api/feedbacks",
    tag = "feedbacks",
    responses(
        (status = 201, description = "提交成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn submit_feedback_handler(
    State(pool): State<PgPool>,
    auth: crate::middleware::auth::AuthUser,
    Json(req): Json<CreateFeedbackRequest>,
) -> Result<(StatusCode, Json<ApiResponse<UserFeedback>>), (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID".to_string())),
            ))
        }
    };

    // 验证内容
    if req.content.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", "反馈内容不能为空".to_string())),
        ));
    }

    // 验证反馈类型
    let valid_types = ["bug", "feature", "other"];
    if !valid_types.contains(&req.feedback_type.to_lowercase().as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_TYPE",
                "反馈类型必须是 bug, feature 或 other".to_string(),
            )),
        ));
    }

    match create_feedback(&pool, user_id, req).await {
        Ok(entity) => Ok((
            StatusCode::CREATED,
            Json(ApiResponse::success(entity.to_user_feedback())),
        )),
        Err(e) => {
            eprintln!("创建反馈失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("CREATE_FAILED", "提交反馈失败".to_string())),
            ))
        }
    }
}

/// 获取用户反馈列表
pub async fn get_my_feedbacks_handler(
    State(pool): State<PgPool>,
    auth: crate::middleware::auth::AuthUser,
    Query(query): Query<FeedbackQuery>,
) -> Result<Json<ApiResponse<Vec<UserFeedback>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID".to_string())),
            ))
        }
    };

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).min(100);

    match get_user_feedbacks(
        &pool,
        user_id,
        query.feedback_type.as_deref(),
        query.status.as_deref(),
        page,
        page_size,
    )
    .await
    {
        Ok(entities) => {
            let feedbacks: Vec<UserFeedback> = entities.iter().map(|e| e.to_user_feedback()).collect();
            Ok(Json(ApiResponse::success(feedbacks)))
        }
        Err(e) => {
            eprintln!("获取反馈列表失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("GET_FAILED", "获取反馈列表失败".to_string())),
            ))
        }
    }
}

/// 获取反馈详情
pub async fn get_feedback_handler(
    State(pool): State<PgPool>,
    auth: crate::middleware::auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<UserFeedback>>, (StatusCode, Json<ApiResponse<()>>)> {
    let feedback_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的反馈ID".to_string())),
            ))
        }
    };

    match get_feedback_by_id(&pool, feedback_id).await {
        Ok(Some(entity)) => Ok(Json(ApiResponse::success(entity.to_user_feedback()))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "反馈不存在".to_string())),
        )),
        Err(e) => {
            eprintln!("获取反馈详情失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("GET_FAILED", "获取反馈详情失败".to_string())),
            ))
        }
    }
}

/// 获取所有反馈（管理员）
#[utoipa::path(
    get,
    path = "/api/feedbacks/all",
    tag = "feedbacks",
    responses(
        (status = 200, description = "获取成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn get_all_feedbacks_handler(
    State(pool): State<PgPool>,
    auth: crate::middleware::auth::AuthUser,
    Query(query): Query<FeedbackQuery>,
) -> Result<Json<ApiResponse<Vec<UserFeedback>>>, (StatusCode, Json<ApiResponse<()>>)> {
    // 验证管理员权限（简单检查：只允许admin邮箱用户）
    // 实际项目中应有更完善的权限系统
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).min(100);

    match get_all_feedbacks(
        &pool,
        query.feedback_type.as_deref(),
        query.status.as_deref(),
        query.priority.as_deref(),
        page,
        page_size,
    )
    .await
    {
        Ok(entities) => {
            let feedbacks: Vec<UserFeedback> = entities.iter().map(|e| e.to_user_feedback()).collect();
            Ok(Json(ApiResponse::success(feedbacks)))
        }
        Err(e) => {
            eprintln!("获取反馈列表失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("GET_FAILED", "获取反馈列表失败".to_string())),
            ))
        }
    }
}

/// 更新反馈状态（管理员）
pub async fn update_feedback_handler(
    State(pool): State<PgPool>,
    auth: crate::middleware::auth::AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateFeedbackRequest>,
) -> Result<Json<ApiResponse<UserFeedback>>, (StatusCode, Json<ApiResponse<()>>)> {
    let feedback_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的反馈ID".to_string())),
            ))
        }
    };

    let admin_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID".to_string())),
            ))
        }
    };

    // 验证状态值
    if let Some(ref status) = req.status {
        let valid_statuses = ["pending", "processing", "resolved", "rejected"];
        if !valid_statuses.contains(&status.to_lowercase().as_str()) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "INVALID_STATUS",
                    "状态必须是 pending, processing, resolved 或 rejected".to_string(),
                )),
            ));
        }
    }

    match update_feedback(
        &pool,
        feedback_id,
        req.status.as_deref(),
        req.priority.as_deref(),
        req.admin_reply.as_deref(),
        Some(admin_id),
    )
    .await
    {
        Ok(Some(entity)) => Ok(Json(ApiResponse::success(entity.to_user_feedback()))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "反馈不存在".to_string())),
        )),
        Err(e) => {
            eprintln!("更新反馈失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("UPDATE_FAILED", "更新反馈失败".to_string())),
            ))
        }
    }
}

/// 删除反馈（管理员）
pub async fn delete_feedback_handler(
    State(pool): State<PgPool>,
    _auth: crate::middleware::auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let feedback_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的反馈ID".to_string())),
            ))
        }
    };

    match delete_feedback(&pool, feedback_id).await {
        Ok(true) => Ok(Json(ApiResponse::success(serde_json::json!({
            "message": "反馈已删除"
        })))),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "反馈不存在".to_string())),
        )),
        Err(e) => {
            eprintln!("删除反馈失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("DELETE_FAILED", "删除反馈失败".to_string())),
            ))
        }
    }
}

/// 获取反馈统计（管理员）
pub async fn get_feedback_stats_handler(
    State(pool): State<PgPool>,
    _auth: crate::middleware::auth::AuthUser,
) -> Result<Json<ApiResponse<FeedbackStats>>, (StatusCode, Json<ApiResponse<()>>)> {
    match get_feedback_stats(&pool).await {
        Ok((total, pending, processing, resolved, rejected, bug, feature, other)) => {
            Ok(Json(ApiResponse::success(FeedbackStats {
                total,
                pending,
                processing,
                resolved,
                rejected,
                by_type: FeedbackTypeStats { bug, feature, other },
            })))
        }
        Err(e) => {
            eprintln!("获取反馈统计失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("STATS_FAILED", "获取反馈统计失败".to_string())),
            ))
        }
    }
}
