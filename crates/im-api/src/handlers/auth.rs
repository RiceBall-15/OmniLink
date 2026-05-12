//! 认证处理器模块
//!
//! 提供用户认证相关的 API 端点：
//! - `POST /api/auth/register` - 用户注册
//! - `POST /api/auth/login` - 用户登录
//! - `GET /api/user/me` - 获取当前用户信息
//! - `PUT /api/user/me` - 更新用户资料

use axum::{
    extract::{Extension, State, Path},
    http::StatusCode,
    Json,
};
use validator::Validate;
use email_validator::validate_email;
use sqlx::PgPool;

use crate::models::auth::{
    ApiResponse, RegisterRequest, LoginRequest, LoginResponse, UpdateUserRequest,
};
use crate::db::user::{create_user, find_user_by_email, find_user_by_id, update_user, verify_password, update_user_profile};
use crate::utils::jwt::generate_token;

/// 用户注册
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
                errors.iter().map(move |e| format!("{}: {}", field, e.message.as_ref().unwrap()))
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
                Json(ApiResponse::success(serde_json::to_value(user).unwrap())),
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
                errors.iter().map(move |e| format!("{}: {}", field, e.message.as_ref().unwrap()))
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
pub async fn get_me(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    match find_user_by_id(&pool, &user_id).await {
        Ok(Some(user_entity)) => {
            let user = user_entity.to_user();
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::to_value(user).unwrap())),
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
                errors.iter().map(move |e| format!("{}: {}", field, e.message.as_ref().unwrap()))
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
                Json(ApiResponse::success(serde_json::to_value(user).unwrap())),
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
                errors.iter().map(move |e| format!("{}: {}", field, e.message.as_ref().unwrap()))
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
                Json(ApiResponse::success(serde_json::to_value(user).unwrap())),
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
                Json(ApiResponse::success(serde_json::to_value(user).unwrap())),
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
