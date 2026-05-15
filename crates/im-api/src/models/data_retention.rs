//! 数据保留策略数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 数据保留策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub retention_days: i32,
    pub target_table: String,
    pub is_enabled: bool,
    pub last_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建保留策略请求
#[derive(Debug, Deserialize)]
pub struct CreateRetentionPolicyRequest {
    pub name: String,
    pub description: Option<String>,
    pub retention_days: i32,
    pub target_table: String,
}

/// 更新保留策略请求
#[derive(Debug, Deserialize)]
pub struct UpdateRetentionPolicyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub retention_days: Option<i32>,
    pub is_enabled: Option<bool>,
}

/// 清理执行结果
#[derive(Debug, Serialize)]
pub struct CleanupResult {
    pub policy_name: String,
    pub target_table: String,
    pub rows_deleted: u64,
    pub executed_at: DateTime<Utc>,
    pub success: bool,
    pub error_message: Option<String>,
}
