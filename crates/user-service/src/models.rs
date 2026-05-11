use serde::{Deserialize, Serialize};
use validator::Validate;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 用户注册请求
#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 20))]
    pub username: String,

    #[validate(email)]
    pub email: String,

    #[validate(length(min = 8))]
    pub password: String,

    #[validate(length(max = 500))]
    pub avatar_url: Option<String>,

    #[validate(length(max = 500))]
    pub bio: Option<String>,
}

/// 用户登录请求
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 1))]
    pub email_or_username: String,

    #[validate(length(min = 1))]
    pub password: String,

    #[validate(length(min = 1))]
    pub device_id: String,

    pub device_name: Option<String>,
}

/// 用户登录响应（匹配前端类型定义）
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: User,
}

/// 用户信息（匹配前端类型定义）
#[derive(Debug, Serialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub avatar: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl User {
    /// 从数据库用户模型转换为前端用户信息
    pub fn from_db_user(db_user: &common::models::User) -> Self {
        Self {
            id: db_user.id.to_string(),
            username: db_user.username.clone(),
            email: db_user.email.clone(),
            avatar: db_user.avatar_url.clone(),
            created_at: db_user.created_at.to_rfc3339(),
            updated_at: db_user.updated_at.to_rfc3339(),
        }
    }
}

/// 刷新Token请求
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// 退出登录请求
#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    pub device_id: String,
}

/// 更新用户资料请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProfileRequest {
    #[validate(length(max = 50))]
    pub username: Option<String>,

    #[validate(email)]
    pub email: Option<String>,

    #[validate(length(max = 500))]
    pub avatar_url: Option<String>,

    #[validate(length(max = 500))]
    pub bio: Option<String>,
}

/// 修改密码请求
#[derive(Debug, Deserialize, Validate)]
pub struct ChangePasswordRequest {
    #[validate(length(min = 1))]
    pub old_password: String,

    #[validate(length(min = 8))]
    pub new_password: String,
}

/// 设备信息
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct DeviceInfo {
    pub id: String,
    pub user_id: Uuid,
    pub device_type: String,
    pub device_name: String,
    pub platform: String,
    pub last_active_at: String,
    pub created_at: String,
}

/// 设备列表响应
#[derive(Debug, Serialize)]
pub struct DevicesResponse {
    pub devices: Vec<DeviceInfo>,
    pub current_device: String,
}
