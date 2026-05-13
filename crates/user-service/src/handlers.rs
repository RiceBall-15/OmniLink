use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::Json,
    Json as JsonResponse,
};
use common::ApiResponse;
use std::sync::Arc;

use validator::Validate;
use crate::middleware::AuthContext;
use crate::models::*;
use crate::services::UserService;

/// 用户注册
pub async fn register(
    State(user_service): State<Arc<UserService>>,
    Json(req): JsonResponse<RegisterRequest>,
) -> Result<Json<ApiResponse<User>>, (StatusCode, String)> {
    match req.validate() {
        Ok(_) => {}
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                e.to_string(),
            ));
        }
    }

    user_service
        .register(req)
        .await
        .map(|user| Json(ApiResponse::success(user)))
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

/// 用户登录
pub async fn login(
    State(user_service): State<Arc<UserService>>,
    Json(req): JsonResponse<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, (StatusCode, String)> {
    match req.validate() {
        Ok(_) => {}
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                e.to_string(),
            ));
        }
    }

    user_service
        .login(req)
        .await
        .map(|response| Json(ApiResponse::success(response)))
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("not found") || msg.contains("User not found") {
                (StatusCode::NOT_FOUND, msg)
            } else if msg.contains("password") || msg.contains("Invalid") {
                (StatusCode::UNAUTHORIZED, msg)
            } else {
                (StatusCode::BAD_REQUEST, msg)
            }
        })
}

/// 刷新 Token
pub async fn refresh_token(
    State(user_service): State<Arc<UserService>>,
    Json(req): JsonResponse<RefreshTokenRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, (StatusCode, String)> {
    user_service
        .refresh_token(req)
        .await
        .map(|response| Json(ApiResponse::success(response)))
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

/// 退出登录
pub async fn logout(
    State(user_service): State<Arc<UserService>>,
    Extension(auth): Extension<AuthContext>,
    Json(req): JsonResponse<LogoutRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, String)> {
    user_service
        .logout(auth.user_id, req)
        .await
        .map(|_| Json(ApiResponse::success(())))
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

/// 获取当前用户信息
pub async fn get_profile(
    State(user_service): State<Arc<UserService>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<ApiResponse<User>>, (StatusCode, String)> {
    user_service
        .get_profile(auth.user_id)
        .await
        .map(|user| Json(ApiResponse::success(user)))
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

/// 更新用户资料
pub async fn update_profile(
    State(user_service): State<Arc<UserService>>,
    Extension(auth): Extension<AuthContext>,
    Json(req): JsonResponse<UpdateProfileRequest>,
) -> Result<Json<ApiResponse<User>>, (StatusCode, String)> {
    match req.validate() {
        Ok(_) => {}
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                e.to_string(),
            ));
        }
    }

    user_service
        .update_profile(auth.user_id, req)
        .await
        .map(|user| Json(ApiResponse::success(user)))
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

/// 修改密码
pub async fn change_password(
    State(user_service): State<Arc<UserService>>,
    Extension(auth): Extension<AuthContext>,
    Json(req): JsonResponse<ChangePasswordRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, String)> {
    match req.validate() {
        Ok(_) => {}
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                e.to_string(),
            ));
        }
    }

    user_service
        .change_password(auth.user_id, req)
        .await
        .map(|_| Json(ApiResponse::success(())))
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("password") || msg.contains("Invalid old password") {
                (StatusCode::UNAUTHORIZED, msg)
            } else {
                (StatusCode::BAD_REQUEST, msg)
            }
        })
}

/// 获取设备列表
pub async fn get_devices(
    State(user_service): State<Arc<UserService>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<ApiResponse<DevicesResponse>>, (StatusCode, String)> {
    // 从请求头或查询参数获取当前设备ID
    let current_device_id = "unknown".to_string(); // 简化版本

    user_service
        .get_devices(auth.user_id, current_device_id)
        .await
        .map(|devices| Json(ApiResponse::success(devices)))
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

/// 删除设备
pub async fn delete_device(
    State(user_service): State<Arc<UserService>>,
    Extension(auth): Extension<AuthContext>,
    Path(device_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, String)> {
    user_service
        .delete_device(auth.user_id, device_id)
        .await
        .map(|_| Json(ApiResponse::success(())))
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

/// 删除账号
pub async fn delete_account(
    State(user_service): State<Arc<UserService>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, String)> {
    user_service
        .delete_account(auth.user_id)
        .await
        .map(|_| Json(ApiResponse::success(())))
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

/// 屏蔽用户
pub async fn block_user(
    State(user_service): State<Arc<UserService>>,
    Extension(auth): Extension<AuthContext>,
    Path(user_id): Path<uuid::Uuid>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, String)> {
    user_service
        .block_user(auth.user_id, user_id)
        .await
        .map(|_| Json(ApiResponse::success(())))
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("not found") || msg.contains("User not found") {
                (StatusCode::NOT_FOUND, msg)
            } else if msg.contains("Cannot block yourself") {
                (StatusCode::BAD_REQUEST, msg)
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
        })
}

/// 取消屏蔽用户
pub async fn unblock_user(
    State(user_service): State<Arc<UserService>>,
    Extension(auth): Extension<AuthContext>,
    Path(user_id): Path<uuid::Uuid>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, String)> {
    let removed = user_service
        .unblock_user(auth.user_id, user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if removed {
        Ok(Json(ApiResponse::success(())))
    } else {
        Err((StatusCode::NOT_FOUND, "Block relationship not found".to_string()))
    }
}

/// 获取屏蔽列表
pub async fn get_blocked_users(
    State(user_service): State<Arc<UserService>>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ApiResponse<crate::models::BlockedUsersResponse>>, (StatusCode, String)> {
    let page = params.get("page")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(1)
        .max(1);
    let page_size = params.get("page_size")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(20)
        .min(100);

    user_service
        .get_blocked_users(auth.user_id, page, page_size)
        .await
        .map(|resp| Json(ApiResponse::success(resp)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// 检查是否已屏蔽某用户
pub async fn check_blocked(
    State(user_service): State<Arc<UserService>>,
    Extension(auth): Extension<AuthContext>,
    Path(user_id): Path<uuid::Uuid>,
) -> Result<Json<ApiResponse<bool>>, (StatusCode, String)> {
    user_service
        .is_blocked(auth.user_id, user_id)
        .await
        .map(|blocked| Json(ApiResponse::success(blocked)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[cfg(test)]
#[path = "handlers_test.rs"]
mod tests;
