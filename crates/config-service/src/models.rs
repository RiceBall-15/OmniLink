use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 配置项
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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