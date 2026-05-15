use std::env;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

/// 测试环境配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub base_url: String,
    pub ws_url: String,
    pub auth_token: String,
    pub admin_token: String,
    pub test_user_id: String,
    pub test_conversation_id: String,
    pub database_url: String,
    pub redis_url: String,
}

/// 全局测试配置（从环境变量加载）
pub static TEST_CONFIG: Lazy<TestConfig> = Lazy::new(|| {
    dotenv::dotenv().ok();
    
    TestConfig {
        base_url: env::var("OMNILINK_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string()),
        ws_url: env::var("OMNILINK_WS_URL")
            .unwrap_or_else(|_| "ws://localhost:8080/ws".to_string()),
        auth_token: env::var("AUTH_TOKEN")
            .unwrap_or_else(|_| "test-token".to_string()),
        admin_token: env::var("ADMIN_TOKEN")
            .unwrap_or_else(|_| "admin-token".to_string()),
        test_user_id: env::var("TEST_USER_ID")
            .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string()),
        test_conversation_id: env::var("TEST_CONVERSATION_ID")
            .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string()),
        database_url: env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://omnilink:omnilink@localhost:5432/omnilink".to_string()),
        redis_url: env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
    }
});

/// 测试环境变量文件模板
pub const ENV_TEMPLATE: &str = r#"# OmniLink 集成测试环境配置
# 复制此文件为 .env 并填写实际值

# 服务地址
OMNILINK_URL=http://localhost:8080
OMNILINK_WS_URL=ws://localhost:8080/ws

# 认证令牌（需要先通过 API 获取）
AUTH_TOKEN=your-auth-token
ADMIN_TOKEN=your-admin-token

# 测试用户和会话 ID
TEST_USER_ID=
TEST_CONVERSATION_ID=

# 数据库连接
DATABASE_URL=postgres://omnilink:omnilink@localhost:5432/omnilink

# Redis 连接
REDIS_URL=redis://localhost:6379
"#;

/// 生成测试配置文件
pub fn generate_env_file(path: &str) -> std::io::Result<()> {
    std::fs::write(path, ENV_TEMPLATE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_loads() {
        let config = &*TEST_CONFIG;
        assert!(!config.base_url.is_empty());
        assert!(!config.ws_url.is_empty());
    }
}
