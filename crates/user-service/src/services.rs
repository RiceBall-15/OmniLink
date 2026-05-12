use common::{AppError, Result};
use uuid::Uuid;
use chrono::Utc;
use crate::models::{
    RegisterRequest, LoginRequest, LoginResponse, RefreshTokenRequest,
    UpdateProfileRequest, ChangePasswordRequest, LogoutRequest,
    User, DevicesResponse
};
use crate::repository::{UserRepository, DeviceRepository};
use crate::jwt::JwtManager;
use crate::password::PasswordManager;
use sqlx::Pool;
use redis::aio::ConnectionManager;
use std::sync::Arc;

/// 用户服务
pub struct UserService {
    user_repo: UserRepository,
    device_repo: DeviceRepository,
    jwt_manager: Arc<JwtManager>,
    redis: ConnectionManager,
}

impl UserService {
    /// 创建新的用户服务实例
    pub fn new(
        pool: Pool<sqlx::Postgres>,
        redis: ConnectionManager,
        jwt_manager: Arc<JwtManager>,
    ) -> Self {
        Self {
            user_repo: UserRepository::new(pool.clone()),
            device_repo: DeviceRepository::new(pool),
            jwt_manager,
            redis,
        }
    }

    /// 获取 JWT 管理器
    pub fn get_token_manager(&self) -> Arc<JwtManager> {
        self.jwt_manager.clone()
    }

    /// 用户注册
    ///
    /// 注册新用户的逻辑：
    /// 1. 验证用户名长度（3-20 字符）
    /// 2. 检查邮箱是否已存在
    /// 3. 检查用户名是否已存在
    /// 4. 使用 bcrypt 加密密码（cost=12）
    /// 5. 生成 UUID 作为用户 ID
    /// 6. 记录创建时间和更新时间
    pub async fn register(&self, req: RegisterRequest) -> Result<User> {
        // 验证用户名长度（3-20 字符）
        if req.username.len() < 3 || req.username.len() > 20 {
            return Err(AppError::BadRequest(
                "Username must be between 3 and 20 characters".to_string(),
            ));
        }

        // 检查邮箱是否已存在
        if self.user_repo.email_exists(&req.email).await? {
            return Err(AppError::BadRequest(
                "Email already exists".to_string(),
            ));
        }

        // 验证用户名是否已存在
        if self.user_repo.username_exists(&req.username).await? {
            return Err(AppError::BadRequest(
                "Username already exists".to_string(),
            ));
        }

        // 使用 bcrypt 加密密码（cost=12）
        let password_hash = PasswordManager::hash_password(&req.password)?;

        // 生成 UUID 作为用户 ID
        let user_id = Uuid::new_v4();

        // 记录创建时间和更新时间
        let _now = Utc::now();

        // 创建用户
        let user = self.user_repo.create(
            user_id,
            req.username,
            req.email,
            password_hash,
            req.avatar_url,
        ).await?;

        // 转换为前端用户信息格式
        Ok(User::from_db_user(&user))
    }

    /// 用户登录
    ///
    /// 用户登录的逻辑：
    /// 1. 根据邮箱查询用户
    /// 2. 使用 bcrypt 验证密码
    /// 3. 生成 JWT Token（有效期 7 天）
    /// 4. 返回 token 和用户信息
    pub async fn login(&self, req: LoginRequest) -> Result<LoginResponse> {
        // 根据邮箱或用户名查询用户
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

        // 使用 bcrypt 验证密码
        let password_valid = PasswordManager::verify_password(&req.password, &user.password_hash)?;
        if !password_valid {
            return Err(AppError::Auth("Invalid password".to_string()));
        }

        // 生成 JWT Token（有效期 7 天）
        let token = self.jwt_manager.generate_token(user.id);

        // 更新最后登录时间
        self.user_repo.update_last_login(user.id).await?;

        // 返回 token 和用户信息（匹配前端格式）
        Ok(LoginResponse {
            token,
            user: User::from_db_user(&user),
        })
    }

    /// 刷新 Token
    pub async fn refresh_token(&self, req: RefreshTokenRequest) -> Result<LoginResponse> {
        // 验证 Refresh Token
        let refresh_key = format!("refresh_token:{}", req.refresh_token);
        let user_id_str: String = redis::cmd("GET")
            .arg(&refresh_key)
            .query_async(&mut self.redis.clone())
            .await
            .map_err(|e| AppError::Internal(format!("Redis error: {}", e)))?;

        let user_id: Uuid = user_id_str.parse()
            .map_err(|_| AppError::BadRequest("Invalid token".to_string()))?;

        // 获取用户信息
        let user = self.user_repo.find_by_id(user_id).await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        // 生成新的 Access Token
        let token = self.jwt_manager.generate_token(user_id);

        Ok(LoginResponse {
            token,
            user: User::from_db_user(&user),
        })
    }

    /// 退出登录
    pub async fn logout(&self, user_id: Uuid, req: LogoutRequest) -> Result<()> {
        // 删除设备
        self.device_repo.delete_device(user_id, req.device_id).await?;
        Ok(())
    }

    /// 获取当前用户信息
    pub async fn get_profile(&self, user_id: Uuid) -> Result<User> {
        let user = self.user_repo.find_by_id(user_id).await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        Ok(User::from_db_user(&user))
    }

    /// 更新用户资料
    pub async fn update_profile(&self, user_id: Uuid, req: UpdateProfileRequest) -> Result<User> {
        // 如果要修改用户名，检查是否已存在
        if let Some(ref username) = req.username {
            // 验证用户名长度（3-20 字符）
            if username.len() < 3 || username.len() > 20 {
                return Err(AppError::BadRequest(
                    "Username must be between 3 and 20 characters".to_string(),
                ));
            }

            let existing_user = self.user_repo.find_by_username(username).await?;
            if let Some(existing) = existing_user {
                if existing.id != user_id {
                    return Err(AppError::BadRequest(
                        "Username already exists".to_string(),
                    ));
                }
            }
        }

        // 如果要修改邮箱，检查是否已存在
        if let Some(ref email) = req.email {
            let existing_user = self.user_repo.find_by_email(email).await?;
            if let Some(existing) = existing_user {
                if existing.id != user_id {
                    return Err(AppError::BadRequest(
                        "Email already exists".to_string(),
                    ));
                }
            }
        }

        // 更新用户资料
        let user = self.user_repo.update_profile(
            user_id,
            req.username,
            req.email,
            req.avatar_url,
        ).await?;

        Ok(User::from_db_user(&user))
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

        // 使用 bcrypt 加密新密码（cost=12）
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