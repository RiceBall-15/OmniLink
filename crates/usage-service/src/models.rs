use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Token使用记录
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TokenUsage {
    pub id: Uuid,
    pub user_id: Uuid,
    pub conversation_id: Option<Uuid>,
    pub model_name: String,
    pub provider: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub cost: f64,
    pub created_at: DateTime<Utc>,
}

/// 统计记录
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UsageStat {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub stat_type: String, // 'daily', 'weekly', 'monthly'
    pub model_name: Option<String>,
    pub provider: Option<String>,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub request_count: i64,
    pub stat_date: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// API调用记录
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ApiCall {
    pub id: Uuid,
    pub user_id: Uuid,
    pub api_endpoint: String,
    pub method: String,
    pub status_code: i32,
    pub response_time_ms: i32,
    pub created_at: DateTime<Utc>,
}

/// 创建Token使用记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTokenUsage {
    pub user_id: Uuid,
    pub conversation_id: Option<Uuid>,
    pub model_name: String,
    pub provider: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub cost: f64,
}

/// 创建API调用记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiCall {
    pub user_id: Uuid,
    pub api_endpoint: String,
    pub method: String,
    pub status_code: i32,
    pub response_time_ms: i32,
}

/// 统计查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageQuery {
    pub user_id: Option<Uuid>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub model_name: Option<String>,
    pub provider: Option<String>,
    pub stat_type: Option<String>,
}

/// 统计结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub total_tokens: i64,
    pub total_cost: f64,
    pub request_count: i64,
    pub by_model: Vec<ModelStats>,
    pub by_provider: Vec<ProviderStats>,
    pub by_date: Vec<DateStats>,
}

/// 模型统计
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ModelStats {
    pub model_name: String,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub request_count: i64,
}

/// 提供商统计
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProviderStats {
    pub provider: String,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub request_count: i64,
}

/// 日期统计
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DateStats {
    pub date: chrono::NaiveDate,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub request_count: i64,
}

/// 统计类型常量
pub mod stat_types {
    pub const DAILY: &str = "daily";
    pub const WEEKLY: &str = "weekly";
    pub const MONTHLY: &str = "monthly";

    pub fn is_valid(stat_type: &str) -> bool {
        matches!(stat_type, DAILY | WEEKLY | MONTHLY)
    }
}

/// 成本计算工具
pub mod cost {
    /// 计算token使用成本
    /// model: 模型名称
    /// prompt_tokens: 输入token数
    /// completion_tokens: 输出token数
    pub fn calculate_cost(model: &str, prompt_tokens: i32, completion_tokens: i32) -> f64 {
        let (input_rate, output_rate) = match model {
            "gpt-4" | "gpt-4-turbo" => (0.00003, 0.00006),
            "gpt-4o" | "gpt-4o-mini" => (0.000005, 0.000015),
            "gpt-3.5-turbo" => (0.0000005, 0.0000015),
            "claude-3-opus" => (0.000015, 0.000075),
            "claude-3-sonnet" => (0.000003, 0.000015),
            "claude-3-haiku" => (0.00000025, 0.00000125),
            _ => (0.000001, 0.000002), // 默认价格
        };

        (prompt_tokens as f64 * input_rate) + (completion_tokens as f64 * output_rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    // === 统计类型测试 ===

    #[test]
    fn test_valid_stat_types() {
        assert!(stat_types::is_valid("daily"));
        assert!(stat_types::is_valid("weekly"));
        assert!(stat_types::is_valid("monthly"));
    }

    #[test]
    fn test_invalid_stat_types() {
        assert!(!stat_types::is_valid("hourly"));
        assert!(!stat_types::is_valid("yearly"));
        assert!(!stat_types::is_valid(""));
    }

    // === CreateTokenUsage 测试 ===

    #[test]
    fn test_create_token_usage_deserialization() {
        let json = r#"{
            "user_id": "550e8400-e29b-41d4-a716-446655440000",
            "model_name": "gpt-4",
            "provider": "openai",
            "prompt_tokens": 100,
            "completion_tokens": 50,
            "total_tokens": 150,
            "cost": 0.006
        }"#;

        let usage: CreateTokenUsage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.model_name, "gpt-4");
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
        assert!(usage.conversation_id.is_none());
    }

    #[test]
    fn test_create_token_usage_with_conversation() {
        let json = r#"{
            "user_id": "550e8400-e29b-41d4-a716-446655440000",
            "conversation_id": "660e8400-e29b-41d4-a716-446655440000",
            "model_name": "claude-3-sonnet",
            "provider": "anthropic",
            "prompt_tokens": 200,
            "completion_tokens": 100,
            "total_tokens": 300,
            "cost": 0.0021
        }"#;

        let usage: CreateTokenUsage = serde_json::from_str(json).unwrap();
        assert!(usage.conversation_id.is_some());
        assert_eq!(usage.provider, "anthropic");
    }

    // === CreateApiCall 测试 ===

    #[test]
    fn test_create_api_call_deserialization() {
        let json = r#"{
            "user_id": "550e8400-e29b-41d4-a716-446655440000",
            "api_endpoint": "/api/chat/completions",
            "method": "POST",
            "status_code": 200,
            "response_time_ms": 1500
        }"#;

        let call: CreateApiCall = serde_json::from_str(json).unwrap();
        assert_eq!(call.api_endpoint, "/api/chat/completions");
        assert_eq!(call.method, "POST");
        assert_eq!(call.status_code, 200);
        assert_eq!(call.response_time_ms, 1500);
    }

    // === UsageQuery 测试 ===

    #[test]
    fn test_usage_query_full() {
        let json = r#"{
            "user_id": "550e8400-e29b-41d4-a716-446655440000",
            "model_name": "gpt-4",
            "provider": "openai",
            "stat_type": "daily"
        }"#;

        let query: UsageQuery = serde_json::from_str(json).unwrap();
        assert!(query.user_id.is_some());
        assert_eq!(query.model_name, Some("gpt-4".to_string()));
        assert_eq!(query.stat_type, Some("daily".to_string()));
    }

    #[test]
    fn test_usage_query_empty() {
        let json = r#"{}"#;
        let query: UsageQuery = serde_json::from_str(json).unwrap();
        assert!(query.user_id.is_none());
        assert!(query.model_name.is_none());
        assert!(query.provider.is_none());
        assert!(query.stat_type.is_none());
    }

    // === UsageStats 序列化测试 ===

    #[test]
    fn test_usage_stats_serialization() {
        let stats = UsageStats {
            total_tokens: 10000,
            total_cost: 0.5,
            request_count: 100,
            by_model: vec![ModelStats {
                model_name: "gpt-4".to_string(),
                total_tokens: 5000,
                total_cost: 0.3,
                request_count: 50,
            }],
            by_provider: vec![ProviderStats {
                provider: "openai".to_string(),
                total_tokens: 5000,
                total_cost: 0.3,
                request_count: 50,
            }],
            by_date: vec![],
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total_tokens\":10000"));
        assert!(json.contains("gpt-4"));
        assert!(json.contains("openai"));
    }

    // === 成本计算测试 ===

    #[test]
    fn test_cost_gpt4() {
        let c = cost::calculate_cost("gpt-4", 1000, 500);
        // 1000 * 0.00003 + 500 * 0.00006 = 0.03 + 0.03 = 0.06
        assert!((c - 0.06).abs() < 0.0001);
    }

    #[test]
    fn test_cost_gpt4o() {
        let c = cost::calculate_cost("gpt-4o", 1000, 500);
        // 1000 * 0.000005 + 500 * 0.000015 = 0.005 + 0.0075 = 0.0125
        assert!((c - 0.0125).abs() < 0.0001);
    }

    #[test]
    fn test_cost_claude3_sonnet() {
        let c = cost::calculate_cost("claude-3-sonnet", 1000, 500);
        // 1000 * 0.000003 + 500 * 0.000015 = 0.003 + 0.0075 = 0.0105
        assert!((c - 0.0105).abs() < 0.0001);
    }

    #[test]
    fn test_cost_unknown_model() {
        let c = cost::calculate_cost("unknown-model", 1000, 500);
        // 1000 * 0.000001 + 500 * 0.000002 = 0.001 + 0.001 = 0.002
        assert!((c - 0.002).abs() < 0.0001);
    }

    #[test]
    fn test_cost_zero_tokens() {
        let c = cost::calculate_cost("gpt-4", 0, 0);
        assert_eq!(c, 0.0);
    }

    // === ModelStats 序列化测试 ===

    #[test]
    fn test_model_stats_serialization() {
        let stats = ModelStats {
            model_name: "gpt-4-turbo".to_string(),
            total_tokens: 50000,
            total_cost: 1.5,
            request_count: 200,
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("gpt-4-turbo"));
        assert!(json.contains("50000"));
    }
}