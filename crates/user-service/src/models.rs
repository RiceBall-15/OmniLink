use serde::{Deserialize, Serialize};
use validator::Validate;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 用户注册请求
#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 50))]
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

    #[validate(length(max = 100))]
    pub device_id: String,

    #[validate(length(max = 50))]
    pub device_name: Option<String>,
}

/// 用户登录响应
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub user: UserInfo,
}

/// 用户信息
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub created_at: DateTime<Utc>,
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
#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub user_id: Uuid,
    pub device_name: String,
    pub device_type: String,
    pub os_version: String,
    pub last_active: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// 设备列表响应
#[derive(Debug, Serialize)]
pub struct DevicesResponse {
    pub devices: Vec<DeviceInfo>,
    pub current_device: String,
}