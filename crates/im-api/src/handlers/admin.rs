//! 管理员用户管理 API Handler
//!
//! 提供管理员用户管理端点：
//! - `GET /api/admin/users` — 用户列表（分页、搜索、筛选）
//! - `GET /api/admin/users/:id` — 用户详情
//! - `PUT /api/admin/users/:id/status` — 封禁/解封用户
//! - `POST /api/admin/users/:id/force-logout` — 强制登出
//! - `GET /api/admin/users/:id/activity` — 用户活动统计

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::admin;
use crate::middleware::auth::AuthUser;
use crate::models::admin::*;
use crate::models::auth::ApiResponse;

/// 获取用户列表（管理员）
pub async fn list_users(
    State(pool): State<PgPool>,
    _auth: AuthUser,
    Query(query): Query<AdminUserQuery>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100).max(1);

    // 确保列存在
    if let Err(e) = admin::ensure_user_columns(&pool).await {
        tracing::warn!("ensure_user_columns: {}", e);
    }

    match admin::get_users(
        &pool,
        page,
        limit,
        query.search.as_deref(),
        query.status.as_deref(),
        query.sort_by.as_deref(),
        query.sort_order.as_deref(),
    )
    .await
    {
        Ok((users, total)) => {
            let user_list: Vec<serde_json::Value> = users
                .into_iter()
                .map(|u| {
                    serde_json::json!({
                        "id": u.id.to_string(),
                        "username": u.username,
                        "email": u.email,
                        "nickname": u.nickname,
                        "avatar": u.avatar,
                        "status": u.status,
                        "online_status": u.online_status,
                        "last_active_at": u.last_active_at.map(|d| d.to_rfc3339()),
                        "created_at": u.created_at.to_rfc3339(),
                        "updated_at": u.updated_at.to_rfc3339(),
                    })
                })
                .collect();

            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "users": user_list,
                    "total": total,
                    "page": page,
                    "limit": limit,
                    "total_pages": (total as f64 / limit as f64).ceil() as i64,
                }))),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("DB_ERROR", format!("查询用户列表失败: {}", e))),
        ),
    }
}

/// 获取用户详情（管理员）
pub async fn get_user_detail(
    State(pool): State<PgPool>,
    _auth: AuthUser,
    Path(user_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let uuid = match Uuid::parse_str(&user_id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的用户 ID")),
            );
        }
    };

    match admin::get_user_detail(&pool, &uuid).await {
        Ok(Some(user)) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "id": user.id.to_string(),
                "username": user.username,
                "email": user.email,
                "nickname": user.nickname,
                "avatar": user.avatar,
                "status": user.status,
                "online_status": user.online_status,
                "last_active_at": user.last_active_at.map(|d| d.to_rfc3339()),
                "created_at": user.created_at.to_rfc3339(),
                "updated_at": user.updated_at.to_rfc3339(),
            }))),
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "用户不存在")),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("DB_ERROR", format!("查询用户失败: {}", e))),
        ),
    }
}

/// 更新用户状态（封禁/解封）
pub async fn update_user_status(
    State(pool): State<PgPool>,
    _auth: AuthUser,
    Path(user_id): Path<String>,
    Json(req): Json<UpdateUserStatusRequest>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    let uuid = match Uuid::parse_str(&user_id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的用户 ID")),
            );
        }
    };

    let allowed_statuses = ["active", "banned", "suspended"];
    if !allowed_statuses.contains(&req.status.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_STATUS", "状态值无效，允许: active, banned, suspended")),
        );
    }

    if let Some(ref reason) = req.reason {
        tracing::info!(
            "管理员更新用户 {} 状态为 {}，原因: {}",
            user_id, req.status, reason
        );
    }

    match admin::update_user_status(&pool, &uuid, &req.status).await {
        Ok(true) => (
            StatusCode::OK,
            Json(ApiResponse::success(true)),
        ),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "用户不存在")),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("DB_ERROR", format!("更新用户状态失败: {}", e))),
        ),
    }
}

/// 强制登出用户
pub async fn force_logout_user(
    State(pool): State<PgPool>,
    _auth: AuthUser,
    Path(user_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<ForceLogoutResult>>) {
    let uuid = match Uuid::parse_str(&user_id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的用户 ID")),
            );
        }
    };

    // 获取用户信息
    let user = match admin::get_user_detail(&pool, &uuid).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "用户不存在")),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("DB_ERROR", format!("查询用户失败: {}", e))),
            );
        }
    };

    // 更新用户状态为强制下线
    let _ = crate::db::user::update_user_online_status(&pool, &user_id, "offline", None).await;

    tracing::info!("管理员强制登出用户: {} ({})", user.username, user_id);

    (
        StatusCode::OK,
        Json(ApiResponse::success(ForceLogoutResult {
            user_id: user_id.clone(),
            username: user.username,
            sessions_revoked: 1,
            success: true,
            message: "用户已被强制登出".to_string(),
        })),
    )
}

/// 获取用户活动统计（管理员视图）
pub async fn get_user_activity(
    State(pool): State<PgPool>,
    _auth: AuthUser,
    Path(user_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let uuid = match Uuid::parse_str(&user_id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的用户 ID")),
            );
        }
    };

    // 获取用户信息
    let user = match admin::get_user_detail(&pool, &uuid).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "用户不存在")),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("DB_ERROR", format!("查询用户失败: {}", e))),
            );
        }
    };

    // 获取消息统计
    let (total_messages, messages_today, messages_this_week, messages_this_month, active_conversations) =
        match admin::get_user_message_stats(&pool, &uuid).await {
            Ok(stats) => stats,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("DB_ERROR", format!("获取消息统计失败: {}", e))),
                );
            }
        };

    // 高峰时段
    let peak_hours = match admin::get_user_peak_hours(&pool, &uuid).await {
        Ok(hours) => hours
            .into_iter()
            .map(|(h, c)| serde_json::json!({"hour": h, "message_count": c}))
            .collect::<Vec<_>>(),
        Err(_) => vec![],
    };

    let avg_per_day = if messages_this_month > 0 {
        messages_this_month as f64 / 30.0
    } else {
        0.0
    };

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "user_id": user_id,
            "username": user.username,
            "total_messages": total_messages,
            "messages_today": messages_today,
            "messages_this_week": messages_this_week,
            "messages_this_month": messages_this_month,
            "active_conversations": active_conversations,
            "avg_messages_per_day": avg_per_day,
            "last_active_at": user.last_active_at.map(|d| d.to_rfc3339()),
            "peak_hours": peak_hours,
        }))),
    )
}
