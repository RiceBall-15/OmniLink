use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 配置项
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ConfigItem {
    pub key: String,
    pub value: String,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<Uuid>,
}

/// 创建配置请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConfigRequest {
    pub key: String,
    pub value: String,
}

/// 更新配置请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfigRequest {
    pub value: String,
}

/// 配置历史版本
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ConfigHistory {
    pub key: String,
    pub value: String,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_by: Option<Uuid>,
    pub change_reason: Option<String>,
}

/// 配置查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigQueryResult {
    pub key: String,
    pub value: String,
    pub version: i32,
    pub updated_at: DateTime<Utc>,
}

/// 批量配置查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfigQuery {
    pub keys: Vec<String>,
}

/// 批量配置响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfigResponse {
    pub configs: Vec<ConfigQueryResult>,
    pub not_found: Vec<String>,
}

/// 配置订阅信息
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ConfigSubscription {
    pub id: Uuid,
    pub key: String,
    pub subscriber: String, // 服务名称或用户ID
    pub callback_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_notified_at: Option<DateTime<Utc>>,
}

/// 创建订阅请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubscriptionRequest {
    pub key: String,
    pub subscriber: String,
    pub callback_url: Option<String>,
}

/// 配置键验证工具
pub mod key_validation {
    /// 验证配置键格式
    /// 规则：只允许字母、数字、下划线、点和斜杠，长度 1-255
    pub fn is_valid_key(key: &str) -> bool {
        if key.is_empty() || key.len() > 255 {
            return false;
        }
        key.chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '/' || c == '-')
    }

    /// 检查是否为保留键
    pub fn is_reserved_key(key: &str) -> bool {
        const RESERVED: &[&str] = &["__system__", "__internal__", "__version__"];
        RESERVED.contains(&key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    // === 配置键验证测试 ===

    #[test]
    fn test_valid_config_keys() {
        assert!(key_validation::is_valid_key("app.name"));
        assert!(key_validation::is_valid_key("database/host"));
        assert!(key_validation::is_valid_key("redis_port"));
        assert!(key_validation::is_valid_key("feature-flags.dark-mode"));
        assert!(key_validation::is_valid_key("a"));
    }

    #[test]
    fn test_invalid_config_keys() {
        assert!(!key_validation::is_valid_key(""));
        assert!(!key_validation::is_valid_key("key with spaces"));
        assert!(!key_validation::is_valid_key("key@special"));
        assert!(!key_validation::is_valid_key(&"x".repeat(256)));
    }

    #[test]
    fn test_reserved_keys() {
        assert!(key_validation::is_reserved_key("__system__"));
        assert!(key_validation::is_reserved_key("__internal__"));
        assert!(key_validation::is_reserved_key("__version__"));
        assert!(!key_validation::is_reserved_key("app.name"));
        assert!(!key_validation::is_reserved_key("system"));
    }

    // === CreateConfigRequest 测试 ===

    #[test]
    fn test_create_config_request_deserialization() {
        let json = r#"{
            "key": "app.name",
            "value": "OmniLink"
        }"#;

        let request: CreateConfigRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.key, "app.name");
        assert_eq!(request.value, "OmniLink");
    }

    // === UpdateConfigRequest 测试 ===

    #[test]
    fn test_update_config_request_deserialization() {
        let json = r#"{"value": "new_value"}"#;
        let request: UpdateConfigRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.value, "new_value");
    }

    // === BatchConfigQuery 测试 ===

    #[test]
    fn test_batch_config_query_deserialization() {
        let json = r#"{
            "keys": ["app.name", "app.version", "db.host"]
        }"#;

        let query: BatchConfigQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.keys.len(), 3);
        assert_eq!(query.keys[0], "app.name");
    }

    #[test]
    fn test_batch_config_query_empty() {
        let json = r#"{"keys": []}"#;
        let query: BatchConfigQuery = serde_json::from_str(json).unwrap();
        assert!(query.keys.is_empty());
    }

    // === BatchConfigResponse 测试 ===

    #[test]
    fn test_batch_config_response_serialization() {
        let response = BatchConfigResponse {
            configs: vec![ConfigQueryResult {
                key: "app.name".to_string(),
                value: "OmniLink".to_string(),
                version: 1,
                updated_at: chrono::Utc::now(),
            }],
            not_found: vec!["missing.key".to_string()],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("app.name"));
        assert!(json.contains("OmniLink"));
        assert!(json.contains("missing.key"));
    }

    // === CreateSubscriptionRequest 测试 ===

    #[test]
    fn test_create_subscription_request_full() {
        let json = r#"{
            "key": "app.name",
            "subscriber": "user-service",
            "callback_url": "http://localhost:8080/config-changed"
        }"#;

        let request: CreateSubscriptionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.key, "app.name");
        assert_eq!(request.subscriber, "user-service");
        assert!(request.callback_url.is_some());
    }

    #[test]
    fn test_create_subscription_request_minimal() {
        let json = r#"{
            "key": "app.name",
            "subscriber": "ai-service"
        }"#;

        let request: CreateSubscriptionRequest = serde_json::from_str(json).unwrap();
        assert!(request.callback_url.is_none());
    }

    // === ConfigKey 长度边界测试 ===

    #[test]
    fn test_config_key_max_length() {
        let max_key = "a".repeat(255);
        assert!(key_validation::is_valid_key(&max_key));

        let too_long = "a".repeat(256);
        assert!(!key_validation::is_valid_key(&too_long));
    }
}