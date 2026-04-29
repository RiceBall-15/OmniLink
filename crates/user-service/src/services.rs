use common::{AppError, Result};
use common::auth::{TokenManager, PasswordManager};
use uuid::Uuid;
use chrono::Utc;
use crate::models::{
    RegisterRequest, LoginRequest, LoginResponse, RefreshTokenRequest,
    UpdateProfileRequest, ChangePasswordRequest, LogoutRequest,
    UserInfo, DevicesResponse, DeviceInfo
};
use crate::repository::{UserRepository, DeviceRepository};
use sqlx::Pool;
use redis::aio::ConnectionManager;
use std::sync::Arc;

/// 用户服务
pub struct UserService {
    user_repo: UserRepository,
    device_repo: DeviceRepository,
    token_manager: Arc<TokenManager>,
    redis: ConnectionManager,
}

impl UserService {
    pub fn new(
        pool: Pool<sqlx::Postgres>,
        redis: ConnectionManager,
        token_manager: Arc<TokenManager>,
    ) -> Self {
        Self {
            user_repo: UserRepository::new(pool.clone()),
            device_repo: DeviceRepository::new(pool),
            token_manager,
            redis,
        }
    }

    /// 用户注册
    pub async fn register(&self, req: RegisterRequest) -> Result<UserInfo> {
        // 验证用户名是否已存在
        if self.user_repo.username_exists(&req.username).await? {
            return Err(AppError::BadRequest("Username already exists".to_string()));
        }

        // 验证邮箱是否已存在
        if self.user_repo.email_exists(&req.email).await? {
            return Err(AppError::BadRequest("Email already exists".to_string()));
        }

        // 哈希密码
        let password_hash = PasswordManager::hash_password(&req.password)?;

        // 创建用户
        let user_id = Uuid::new_v4();
        let user = self.user_repo.create(
            user_id,
            req.username,
            req.email,
            password_hash,
            req.avatar_url,
            req.bio,
        ).await?;

        Ok(UserInfo {
            id: user.id,
            username: user.username,
            email: user.email,
            avatar_url: user.avatar_url,
            bio: user.bio,
            created_at: user.created_at,
        })
    }

    /// 用户登录
    pub async fn login(&self, req: LoginRequest) -> Result<LoginResponse> {
        // 查找用户
        let user = if req.email_or_username.contains('@') {
            self.user_repo
                .find_by_email(&req.email_or_username)
                .await?
                .ok_or_else(|| AppError::NotFound("User not found".to_string()))?
        } else {
            self.user_repo
                .find_by_username(&req.email_or_username)
                .await?
                .ok_or_else(|| AppError::NotFound("User not found".to_string()))?
        };

        // 验证密码
        let password_valid = PasswordManager::verify_password(&req.password, &user.password_hash)?;
        if !password_valid {
            return Err(AppError::Auth("Invalid password".to_string()));
        }

        // 生成Token
        let device_id = req.device_id.clone();
        let access_token = self.token_manager.generate_access_token(user.id, device_id.clone());
        let refresh_token = Uuid::new_v4().to_string();

        // 存储Refresh Token到Redis
        let refresh_key = format!("refresh_token:{}", refresh_token);
        let _: () = redis::cmd("SETEX")
            .arg(&refresh_key)
            .arg(30 * 24 * 60 * 60) // 30天过期
            .arg(user_id.to_string())
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e| AppError::Redis(e))?;

        // 注册设备
        self.device_repo.register_device(
            user.id,
            device_id.clone(),
            req.device_name.unwrap_or_else(|| "Unknown".to_string()),
            "unknown".to_string(),
            "unknown".to_string(),
        ).await?;

        // 更新最后登录时间
        self.user_repo.update_last_login(user.id).await?;

        Ok(LoginResponse {
            access_token,
            refresh_token,
            expires_in: 7 * 24 * 60 * 60, // 7天
            user: UserInfo {
                id: user.id,
                username: user.username,
                email: user.email,
                avatar_url: user.avatar_url,
                bio: user.bio,
                created_at: user.created_at,
            },
        })
    }

    /// 刷新Token
    pub async fn refresh_token(&self, req: RefreshTokenRequest) -> Result<LoginResponse> {
        // 验证Refresh Token
        let refresh_key = format!("refresh_token:{}", req.refresh_token);
        let user_id_str: String = redis::cmd("GET")
            .arg(&refresh_key)
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e| AppError::Redis(e))?;

        let user_id: Uuid = user_id_str.parse()
            .map_err(|_| AppError::BadRequest("Invalid token".to_string()))?;

        // 获取用户信息
        let user = self.user_repo.find_by_id(user_id).await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        // 生成新的Access Token
        let access_token = self.token_manager.generate_access_token(user_id, "refresh".to_string());

        Ok(LoginResponse {
            access_token,
            refresh_token: req.refresh_token,
            expires_in: 7 * 24 * 60 * 60,
            user: UserInfo {
                id: user.id,
                username: user.username,
                email: user.email,
                avatar_url: user.avatar_url,
                bio: user.bio,
                created_at: user.created_at,
            },
        })
    }

    /// 退出登录
    pub async fn logout(&self, user_id: Uuid, req: LogoutRequest) -> Result<()> {
        // 删除Refresh Token（如果有）
        // 注意：这里需要额外的存储来管理用户的所有refresh token
        // 简化版本只删除当前设备的token

        // 删除设备
        self.device_repo.delete_device(user_id, req.device_id).await?;

        Ok(())
    }

    /// 获取当前用户信息
    pub async fn get_profile(&self, user_id: Uuid) -> Result<UserInfo> {
        let user = self.user_repo.find_by_id(user_id).await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        Ok(UserInfo {
            id: user.id,
            username: user.username,
            email: user.email,
            avatar_url: user.avatar_url,
            bio: user.bio,
            created_at: user.created_at,
        })
    }

    /// 更新用户资料
    pub async fn update_profile(&self, user_id: Uuid, req: UpdateProfileRequest) -> Result<UserInfo> {
        // 如果要修改用户名，检查是否已存在
        if let Some(ref username) = req.username {
            let existing_user = self.user_repo.find_by_username(username).await?;
            if let Some(existing) = existing_user {
                if existing.id != user_id {
                    return Err(AppError::BadRequest("Username already exists".to_string()));
                }
            }
        }

        // 如果要修改邮箱，检查是否已存在
        if let Some(ref email) = req.email {
            let existing_user = self.user_repo.find_by_email(email).await?;
            if let Some(existing) = existing_user {
                if existing.id != user_id {
                    return Err(AppError::BadRequest("Email already exists".to_string()));
                }
            }
        }

        // 更新用户资料
        let user = self.user_repo.update_profile(
            user_id,
            req.username,
            req.email,
            req.avatar_url,
            req.bio,
        ).await?;

        Ok(UserInfo {
            id: user.id,
            username: user.username,
            email: user.email,
            avatar_url: user.avatar_url,
            bio: user.bio,
            created_at: user.created_at,
        })
    }

    /// 修改密码
    pub async fn change_password(&self, user_id: Uuid, req: ChangePasswordRequest) -> Result<()> {
        // 获取用户信息
        let user = self.user_repo.find_by_id(user_id).await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        // 验证旧密码
        let password_valid = PasswordManager::verify_password(&req.old_password, &user.password_hash)?;
        if !password_valid {
            return Err(AppError::Auth("Invalid old password".to_string()));
        }

        // 哈希新密码
        let new_password_hash = PasswordManager::hash_password(&req.new_password)?;

        // 更新密码
        self.user_repo.update_password(user_id, new_password_hash).await?;

        Ok(())
    }

    /// 获取设备列表
    pub async fn get_devices(&self, user_id: Uuid, current_device_id: String) -> Result<DevicesResponse> {
        let devices = self.device_repo.get_user_devices(user_id).await?;

        Ok(DevicesResponse {
            devices,
            current_device: current_device_id,
        })
    }

    /// 删除设备
    pub async fn delete_device(&self, user_id: Uuid, device_id: String) -> Result<()> {
        self.device_repo.delete_device(user_id, device_id).await?;
        Ok(())
    }

    /// 删除账号
    pub async fn delete_account(&self, user_id: Uuid) -> Result<()> {
        // 删除用户（级联删除相关数据）
        self.user_repo.delete(user_id).await?;
        Ok(())
    }
}