//! 应用配置模块
//!
//! 统一管理所有服务配置，支持从环境变量加载和验证。

use std::env;
use std::fmt;

/// 应用配置验证错误
#[derive(Debug)]
pub enum ConfigError {
    Missing(String),
    Invalid(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Missing(key) => write!(f, "Missing required config: {}", key),
            ConfigError::Invalid(msg) => write!(f, "Invalid config: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

/// 应用配置
///
/// 从环境变量加载，支持验证。所有配置项都有合理的默认值。
#[derive(Debug, Clone)]
pub struct AppConfig {
    // 数据库
    pub database_url: String,
    pub db_max_connections: u32,
    pub db_min_connections: u32,
    pub db_acquire_timeout_secs: u64,
    pub db_idle_timeout_secs: u64,
    pub db_max_lifetime_secs: u64,

    // Redis
    pub redis_url: String,

    // JWT
    pub jwt_secret: String,
    pub jwt_expiration_hours: u64,

    // 服务端口
    pub server_host: String,
    pub server_port: u16,

    // 速率限制
    pub rate_limit_max_requests: u32,
    pub rate_limit_window_secs: u64,

    // 日志
    pub log_level: String,

    // 环境
    pub app_env: String,
}

impl AppConfig {
    /// 从环境变量加载配置并验证
    pub fn load() -> Result<Self, ConfigError> {
        let config = AppConfig {
            // 数据库
            database_url: Self::require_env("DATABASE_URL")?,
            db_max_connections: Self::parse_env("DB_MAX_CONNECTIONS", 10),
            db_min_connections: Self::parse_env("DB_MIN_CONNECTIONS", 2),
            db_acquire_timeout_secs: Self::parse_env("DB_ACQUIRE_TIMEOUT_SECS", 15),
            db_idle_timeout_secs: Self::parse_env("DB_IDLE_TIMEOUT_SECS", 300),
            db_max_lifetime_secs: Self::parse_env("DB_MAX_LIFETIME_SECS", 1800),

            // Redis
            redis_url: Self::get_env("REDIS_URL", "redis://127.0.0.1:6379"),

            // JWT
            jwt_secret: Self::require_env("JWT_SECRET")?,
            jwt_expiration_hours: Self::parse_env("JWT_EXPIRATION_HOURS", 24),

            // 服务端口
            server_host: Self::get_env("SERVER_HOST", "0.0.0.0"),
            server_port: Self::parse_env("SERVER_PORT", 8002),

            // 速率限制
            rate_limit_max_requests: Self::parse_env("RATE_LIMIT_MAX_REQUESTS", 100),
            rate_limit_window_secs: Self::parse_env("RATE_LIMIT_WINDOW_SECS", 60),

            // 日志
            log_level: Self::get_env("RUST_LOG", "info"),

            // 环境
            app_env: Self::get_env("APP_ENV", "development"),
        };

        config.validate()?;
        Ok(config)
    }

    /// 验证配置值的合法性
    fn validate(&self) -> Result<(), ConfigError> {
        // JWT secret 不能是默认值
        if self.jwt_secret == "your-secret-key-change-in-production"
            || self.jwt_secret == "your-jwt-secret-change-in-production"
            || self.jwt_secret.len() < 16
        {
            return Err(ConfigError::Invalid(
                "JWT_SECRET must be at least 16 characters and not a default value".to_string(),
            ));
        }

        // 数据库连接数范围检查
        if self.db_max_connections < 1 || self.db_max_connections > 100 {
            return Err(ConfigError::Invalid(
                "DB_MAX_CONNECTIONS must be between 1 and 100".to_string(),
            ));
        }

        // 端口范围检查
        if self.server_port == 0 {
            return Err(ConfigError::Invalid(
                "SERVER_PORT must be a valid port number".to_string(),
            ));
        }

        // 速率限制检查
        if self.rate_limit_max_requests == 0 {
            return Err(ConfigError::Invalid(
                "RATE_LIMIT_MAX_REQUESTS must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }

    /// 获取服务器绑定地址
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }

    /// 是否为生产环境
    pub fn is_production(&self) -> bool {
        self.app_env == "production"
    }

    // 辅助方法

    fn require_env(key: &str) -> Result<String, ConfigError> {
        env::var(key).map_err(|_| ConfigError::Missing(key.to_string()))
    }

    fn get_env(key: &str, default: &str) -> String {
        env::var(key).unwrap_or_else(|_| default.to_string())
    }

    fn parse_env<T: std::str::FromStr>(key: &str, default: T) -> T {
        env::var(key)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }
}

impl fmt::Display for AppConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "AppConfig:")?;
        writeln!(f, "  env: {}", self.app_env)?;
        writeln!(f, "  server: {}:{}", self.server_host, self.server_port)?;
        writeln!(f, "  db_max_connections: {}", self.db_max_connections)?;
        writeln!(f, "  rate_limit: {}/{}s", self.rate_limit_max_requests, self.rate_limit_window_secs)?;
        writeln!(f, "  jwt_expiration: {}h", self.jwt_expiration_hours)?;
        writeln!(f, "  log_level: {}", self.log_level)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation_jwt_too_short() {
        let config = AppConfig {
            database_url: "postgres://localhost/test".to_string(),
            db_max_connections: 10,
            db_min_connections: 2,
            db_acquire_timeout_secs: 15,
            db_idle_timeout_secs: 300,
            db_max_lifetime_secs: 1800,
            redis_url: "redis://localhost".to_string(),
            jwt_secret: "short".to_string(),
            jwt_expiration_hours: 24,
            server_host: "0.0.0.0".to_string(),
            server_port: 8002,
            rate_limit_max_requests: 100,
            rate_limit_window_secs: 60,
            log_level: "info".to_string(),
            app_env: "development".to_string(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_default_jwt() {
        let config = AppConfig {
            database_url: "postgres://localhost/test".to_string(),
            db_max_connections: 10,
            db_min_connections: 2,
            db_acquire_timeout_secs: 15,
            db_idle_timeout_secs: 300,
            db_max_lifetime_secs: 1800,
            redis_url: "redis://localhost".to_string(),
            jwt_secret: "your-secret-key-change-in-production".to_string(),
            jwt_expiration_hours: 24,
            server_host: "0.0.0.0".to_string(),
            server_port: 8002,
            rate_limit_max_requests: 100,
            rate_limit_window_secs: 60,
            log_level: "info".to_string(),
            app_env: "development".to_string(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_valid() {
        let config = AppConfig {
            database_url: "postgres://localhost/test".to_string(),
            db_max_connections: 10,
            db_min_connections: 2,
            db_acquire_timeout_secs: 15,
            db_idle_timeout_secs: 300,
            db_max_lifetime_secs: 1800,
            redis_url: "redis://localhost".to_string(),
            jwt_secret: "my-secure-jwt-secret-key-2026".to_string(),
            jwt_expiration_hours: 24,
            server_host: "0.0.0.0".to_string(),
            server_port: 8002,
            rate_limit_max_requests: 100,
            rate_limit_window_secs: 60,
            log_level: "info".to_string(),
            app_env: "development".to_string(),
        };
        assert!(config.validate().is_ok());
    }
}
