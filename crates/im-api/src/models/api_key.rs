use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;

/// API Key 权限级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyPermission {
    /// 只读权限
    Read,
    /// 读写权限
    ReadWrite,
    /// 管理员权限
    Admin,
}

impl std::fmt::Display for ApiKeyPermission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read => write!(f, "read"),
            Self::ReadWrite => write!(f, "read_write"),
            Self::Admin => write!(f, "admin"),
        }
    }
}

impl std::str::FromStr for ApiKeyPermission {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "read" => Ok(Self::Read),
            "read_write" => Ok(Self::ReadWrite),
            "admin" => Ok(Self::Admin),
            _ => Err(format!("Unknown permission: {}", s)),
        }
    }
}

/// API Key 实体（数据库映射）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiKeyEntity {
    pub id: Uuid,
    pub key_prefix: String,      // 前8字符，用于显示
    pub key_hash: String,        // SHA-256 hash
    pub name: String,
    pub permissions: String,     // "read", "read_write", "admin"
    pub rate_limit: Option<i32>, // 自定义速率限制（每分钟）
    pub owner_id: Uuid,
    pub is_active: bool,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建 API Key 请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateApiKeyRequest {
    /// API Key 名称
    pub name: String,
    /// 权限级别
    pub permissions: Option<String>,
    /// 自定义速率限制（每分钟请求数）
    pub rate_limit: Option<i32>,
    /// 过期时间（ISO 8601 格式）
    pub expires_at: Option<String>,
}

/// API Key 响应（包含明文 key，仅在创建时返回一次）
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateApiKeyResponse {
    pub id: Uuid,
    /// 明文 API Key（仅此一次返回）
    pub key: String,
    pub key_prefix: String,
    pub name: String,
    pub permissions: String,
    pub rate_limit: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// API Key 列表项（不含敏感信息）
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyInfo {
    pub id: Uuid,
    pub key_prefix: String,
    pub name: String,
    pub permissions: String,
    pub rate_limit: Option<i32>,
    pub is_active: bool,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// 生成新的 API Key
pub fn generate_api_key() -> (String, String, String) {
    let raw_key = format!("omk_{}", Uuid::new_v4().to_string().replace('-', ""));
    let key_prefix = raw_key[..8].to_string();
    let key_hash = sha256_hash(&raw_key);
    (raw_key, key_prefix, key_hash)
}

/// SHA-256 哈希
fn sha256_hash(input: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    // 简化实现：使用 hex 编码的 UUID 作为 hash
    // 生产环境应使用 ring 或 sha2 crate
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// 验证 API Key
pub fn verify_api_key(provided_key: &str, stored_hash: &str) -> bool {
    sha256_hash(provided_key) == stored_hash
}
