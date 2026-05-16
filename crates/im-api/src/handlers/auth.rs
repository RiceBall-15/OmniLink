//! 认证处理器模块
//!
//! 提供用户认证相关的 API 端点：
//! - `POST /api/auth/register` - 用户注册
//! - `POST /api/auth/login` - 用户登录
//! - `GET /api/user/me` - 获取当前用户信息
//! - `PUT /api/user/me` - 更新用户资料

use axum::{
    extract::{Extension, State, Path, Query},
    http::StatusCode,
    Json,
};
use validator::Validate;
use email_validator::validate_email;
use sqlx::PgPool;

use crate::models::auth::{
    ApiResponse, RegisterRequest, LoginRequest, LoginResponse, UpdateUserRequest,
    BlockUserRequest, BlockListResponse, BlockStatusResponse,
};
use crate::db::user::{
    create_user, find_user_by_email, find_user_by_id, update_user, verify_password, update_user_profile,
    block_user, unblock_user, get_blocked_users, is_user_blocked,
    update_user_online_status, get_user_status, batch_get_user_status,
};
use crate::middleware::auth::AuthUser;
use crate::utils::jwt::generate_token;
use common::request_validation::ValidationSchemas;

/// 安全序列化为 JSON Value，避免 unwrap 导致 panic
fn to_json_value<T: serde::Serialize>(value: &T) -> Result<serde_json::Value, (StatusCode, Json<ApiResponse<serde_json::Value>>)> {
    serde_json::to_value(value).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERIALIZATION_FAILED", format!("数据序列化失败: {}", e))),
        )
    })
}



/// 用户注册
#[utoipa::path(
    post,
    path = "/api/auth/register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "注册成功", body = ApiResponse<serde_json::Value>),
        (status = 400, description = "请求参数错误", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn register(
    State(pool): State<PgPool>,
    Json(req): Json<RegisterRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    // 验证请求数据
    if let Err(errors) = req.validate() {
        let error_msg = errors
            .field_errors()
            .iter()
            .flat_map(|(field, errors)| {
                errors.iter().map(move |e| format!("{}: {}", field, e.message.as_ref().map(|m| m.as_ref()).unwrap_or("")))
            })
            .collect::<Vec<_>>()
            .join("; ");
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", error_msg)),
        );
    }

    // 验证邮箱格式
    if !validate_email(&req.email) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_EMAIL", "邮箱格式不正确")),
        );
    }

    // 业务级验证（密码强度、用户名格式等）
    if let Err(errors) = ValidationSchemas::user_register().validate_serializable(&req) {
        let error_msg = errors.to_response().message;
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::error("VALIDATION_ERROR", error_msg)),
        );
    }

    // 创建用户
    let params = crate::models::auth::CreateUserParams {
        username: req.username.clone(),
        email: req.email.clone(),
        password_hash: req.password.clone(),
    };

    match create_user(&pool, params).await {
        Ok(user_entity) => {
            let user = user_entity.to_user();
            (
                StatusCode::CREATED,
                match to_json_value(&user) {
                    Ok(v) => Json(ApiResponse::success(v)),
                    Err(e) => return e,
                },
            )
        }
        Err(e) => {
            let (code, msg) = if e.contains("邮箱") {
                ("EMAIL_EXISTS", e)
            } else if e.contains("用户名") {
                ("USERNAME_EXISTS", e)
            } else {
                ("REGISTER_FAILED", e)
            };
            (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(code, msg)),
            )
        }
    }
}

/// 用户登录
#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "登录成功", body = ApiResponse<LoginResponse>),
        (status = 400, description = "请求参数错误", body = ApiResponse<LoginResponse>),
        (status = 401, description = "邮箱或密码错误", body = ApiResponse<LoginResponse>),
    )
)]
pub async fn login(
    State(pool): State<PgPool>,
    Json(req): Json<LoginRequest>,
) -> (StatusCode, Json<ApiResponse<LoginResponse>>) {
    // 验证请求数据
    if let Err(errors) = req.validate() {
        let error_msg = errors
            .field_errors()
            .iter()
            .flat_map(|(field, errors)| {
                errors.iter().map(move |e| format!("{}: {}", field, e.message.as_ref().map(|m| m.as_ref()).unwrap_or("")))
            })
            .collect::<Vec<_>>()
            .join("; ");
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", error_msg)),
        );
    }

    // 查找用户
    match find_user_by_email(&pool, &req.email).await {
        Ok(Some(user_entity)) => {
            // 验证密码
            match verify_password(&req.password, &user_entity.password_hash) {
                Ok(true) => {
                    // 生成 token
                    let user_id = user_entity.id.to_string();
                    match generate_token(&user_id, &user_entity.email) {
                        Ok(token) => {
                            let user = user_entity.to_user();
                            let login_response = LoginResponse { token, user };
                            (
                                StatusCode::OK,
                                Json(ApiResponse::success(login_response)),
                            )
                        }
                        Err(e) => (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::error("TOKEN_GENERATION_FAILED", e)),
                        ),
                    }
                }
                Ok(false) => (
                    StatusCode::UNAUTHORIZED,
                    Json(ApiResponse::error("INVALID_CREDENTIALS", "邮箱或密码错误")),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("PASSWORD_VERIFY_FAILED", e)),
                ),
            }
        }
        Ok(None) => (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("INVALID_CREDENTIALS", "邮箱或密码错误")),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("LOGIN_FAILED", e)),
        ),
    }
}

/// 获取当前用户信息
#[utoipa::path(
    get,
    path = "/api/user/me",
    tag = "auth",
    responses(
        (status = 200, description = "获取成功", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "用户不存在", body = ApiResponse<serde_json::Value>),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_me(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    match find_user_by_id(&pool, &user_id).await {
        Ok(Some(user_entity)) => {
            let user = user_entity.to_user();
            (
                StatusCode::OK,
                match to_json_value(&user) {
                    Ok(v) => Json(ApiResponse::success(v)),
                    Err(e) => return e,
                },
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("USER_NOT_FOUND", "用户不存在")),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("GET_USER_FAILED", e)),
        ),
    }
}

/// 更新用户资料
pub async fn update_me(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    Json(req): Json<UpdateUserRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    // 验证请求数据
    if let Err(errors) = req.validate() {
        let error_msg = errors
            .field_errors()
            .iter()
            .flat_map(|(field, errors)| {
                errors.iter().map(move |e| format!("{}: {}", field, e.message.as_ref().map(|m| m.as_ref()).unwrap_or("")))
            })
            .collect::<Vec<_>>()
            .join("; ");
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", error_msg)),
        );
    }

    // 验证邮箱格式（如果提供了邮箱）
    if let Some(ref email) = req.email {
        if !validate_email(email) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_EMAIL", "邮箱格式不正确")),
            );
        }
    }

    // 更新用户
    match update_user(&pool, &user_id, req.username, req.email, req.avatar).await {
        Ok(user_entity) => {
            let user = user_entity.to_user();
            (
                StatusCode::OK,
                match to_json_value(&user) {
                    Ok(v) => Json(ApiResponse::success(v)),
                    Err(e) => return e,
                },
            )
        }
        Err(e) => {
            let (code, msg) = if e.contains("邮箱") {
                ("EMAIL_EXISTS", e)
            } else if e.contains("用户名") {
                ("USERNAME_EXISTS", e)
            } else {
                ("UPDATE_FAILED", e)
            };
            (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(code, msg)),
            )
        }
    }
}

/// 更新用户资料（扩展字段：nickname, bio, status_message, avatar）
pub async fn update_profile(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    Json(req): Json<crate::models::auth::UpdateUserProfileRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    // 验证请求数据
    if let Err(errors) = req.validate() {
        let error_msg = errors
            .field_errors()
            .iter()
            .flat_map(|(field, errors)| {
                errors.iter().map(move |e| format!("{}: {}", field, e.message.as_ref().map(|m| m.as_ref()).unwrap_or("")))
            })
            .collect::<Vec<_>>()
            .join("; ");
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", error_msg)),
        );
    }

    // 更新用户资料
    match update_user_profile(&pool, &user_id, req.nickname, req.bio, req.status_message, req.avatar).await {
        Ok(user_entity) => {
            let user = user_entity.to_user();
            (
                StatusCode::OK,
                match to_json_value(&user) {
                    Ok(v) => Json(ApiResponse::success(v)),
                    Err(e) => return e,
                },
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("UPDATE_PROFILE_FAILED", e)),
        ),
    }
}

/// 获取指定用户公开资料
pub async fn get_user_profile(
    State(pool): State<PgPool>,
    Extension(_user_id): Extension<String>,
    Path(target_user_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    match find_user_by_id(&pool, &target_user_id).await {
        Ok(Some(user_entity)) => {
            let user = user_entity.to_user();
            (
                StatusCode::OK,
                match to_json_value(&user) {
                    Ok(v) => Json(ApiResponse::success(v)),
                    Err(e) => return e,
                },
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("USER_NOT_FOUND", "用户不存在")),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("GET_USER_FAILED", e)),
        ),
    }
}

/// 屏蔽用户
pub async fn block_user_handler(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    Json(req): Json<BlockUserRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    if req.blocked_user_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_REQUEST", "被屏蔽用户ID不能为空")),
        );
    }

    match block_user(&pool, &user_id, &req.blocked_user_id).await {
        Ok(()) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "blocked": true,
                "blockedUserId": req.blocked_user_id,
                "message": "已屏蔽该用户"
            }))),
        ),
        Err(e) if e.contains("已经屏蔽") => (
            StatusCode::CONFLICT,
            Json(ApiResponse::error("ALREADY_BLOCKED", e)),
        ),
        Err(e) if e.contains("不能屏蔽自己") => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("CANNOT_BLOCK_SELF", e)),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("BLOCK_FAILED", e)),
        ),
    }
}

/// 取消屏蔽用户
pub async fn unblock_user_handler(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    Path(blocked_user_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    match unblock_user(&pool, &user_id, &blocked_user_id).await {
        Ok(()) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "unblocked": true,
                "blockedUserId": blocked_user_id,
                "message": "已取消屏蔽"
            }))),
        ),
        Err(e) if e.contains("未找到") => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("BLOCK_NOT_FOUND", e)),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("UNBLOCK_FAILED", e)),
        ),
    }
}

/// 获取屏蔽列表
pub async fn get_blocked_list_handler(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    match get_blocked_users(&pool, &user_id).await {
        Ok(blocks) => {
            let total = blocks.len() as i64;
            let response = BlockListResponse { blocks, total };
            (
                StatusCode::OK,
                match to_json_value(&response) {
                    Ok(v) => Json(ApiResponse::success(v)),
                    Err(e) => return e,
                },
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("GET_BLOCKED_FAILED", e)),
        ),
    }
}

/// 检查屏蔽状态
pub async fn check_block_status_handler(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
    Path(other_user_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let is_blocked = match is_user_blocked(&pool, &user_id, &other_user_id).await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("CHECK_BLOCK_FAILED", e)),
            );
        }
    };

    let has_blocked = match is_user_blocked(&pool, &other_user_id, &user_id).await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("CHECK_BLOCK_FAILED", e)),
            );
        }
    };

    let response = BlockStatusResponse {
        is_blocked,
        has_blocked,
    };

    (
        StatusCode::OK,
        match to_json_value(&response) {
                    Ok(v) => Json(ApiResponse::success(v)),
                    Err(e) => return e,
                },
    )
}

/// 更新用户在线状态
///
/// PUT /api/users/status
pub async fn update_user_status_handler(
    State(pool): State<PgPool>,
    AuthUser { user_id, .. }: AuthUser,
    Json(request): Json<crate::models::message::UpdateStatusRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let status_str = request.status.to_string();
    let msg_ref = request.status_message.as_deref();

    match update_user_online_status(&pool, &user_id, &status_str, msg_ref).await {
        Ok(()) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "status": status_str,
                "statusMessage": request.status_message,
                "message": "在线状态已更新"
            }))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("UPDATE_STATUS_FAILED", format!("更新状态失败: {}", e))),
        ),
    }
}

/// 获取用户在线状态详情
///
/// GET /api/users/:id/status
pub async fn get_user_status_handler(
    State(pool): State<PgPool>,
    Path(target_user_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    match get_user_status(&pool, &target_user_id).await {
        Ok(status_info) => (
            StatusCode::OK,
            match to_json_value(&status_info) {
                Ok(v) => Json(ApiResponse::success(v)),
                Err(e) => return e,
            },
        ),
        Err(e) => {
            if e.contains("不存在") {
                (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::error("USER_NOT_FOUND", &e)),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("GET_STATUS_FAILED", format!("获取状态失败: {}", e))),
                )
            }
        }
    }
}

/// 批量查询用户在线状态
///
/// GET /api/users/presence?ids=id1,id2,id3
pub async fn batch_get_presence_handler(
    State(pool): State<PgPool>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let ids_str = match params.get("ids") {
        Some(ids) => ids,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("MISSING_IDS", "缺少 ids 参数")),
            );
        }
    };

    let user_ids: Vec<String> = ids_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if user_ids.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("EMPTY_IDS", "ids 参数不能为空")),
        );
    }

    if user_ids.len() > 100 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("TOO_MANY_IDS", "单次查询最多 100 个用户")),
        );
    }

    match batch_get_user_status(&pool, &user_ids).await {
        Ok(status_list) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "users": status_list,
                "count": status_list.len()
            }))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("BATCH_PRESENCE_FAILED", format!("批量查询失败: {}", e))),
        ),
    }
}
