use bcrypt::{hash, verify, DEFAULT_COST};
use crate::error::{AppError, Result};

/// 密码管理器（使用 bcrypt）
pub struct PasswordManager;

impl PasswordManager {
    /// 哈希密码
    ///
    /// 使用 bcrypt 加密密码，cost=12
    ///
    /// # Arguments
    /// * `password` - 原始密码
    ///
    /// # Returns
    /// 返回加密后的密码哈希
    pub fn hash_password(password: &str) -> Result<String> {
        hash(password, DEFAULT_COST)
            .map_err(|e| AppError::Auth(format!("Password hash failed: {}", e)))
    }

    /// 验证密码
    ///
    /// 使用 bcrypt 验证密码是否匹配哈希值
    ///
    /// # Arguments
    /// * `password` - 原始密码
    /// * `hash` - 密码哈希
    ///
    /// # Returns
    /// 返回密码是否匹配
    pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
        verify(password, hash)
            .map_err(|e| AppError::Auth(format!("Password verification failed: {}", e)))
    }
}
