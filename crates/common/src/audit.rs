//! 审计日志模块
//!
//! 提供系统操作审计功能，记录用户关键操作（登录、注册、密码修改、
//! 会话创建、消息发送等），支持审计日志查询和敏感操作追踪。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, FromRow};
use uuid::Uuid;

/// 审计操作类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditAction {
    /// 用户登录
    UserLogin,
    /// 用户注册
    UserRegister,
    /// 密码修改
    PasswordChange,
    /// 用户资料更新
    ProfileUpdate,
    /// 会话创建
    ConversationCreate,
    /// 会话删除
    ConversationDelete,
    /// 消息发送
    MessageSend,
    /// 消息删除
    MessageDelete,
    /// 消息编辑
    MessageEdit,
    /// 消息撤回
    MessageRecall,
    /// 用户屏蔽
    UserBlock,
    /// 用户取消屏蔽
    UserUnblock,
    /// 文件上传
    FileUpload,
    /// 文件下载
    FileDownload,
    /// 加密密钥生成
    EncryptionKeyGenerate,
    /// 管理员操作
    AdminAction,
    /// 自定义操作
    Custom(String),
}

impl AuditAction {
    /// 转换为字符串标识
    pub fn as_str(&self) -> String {
        match self {
            Self::UserLogin => "user_login".to_string(),
            Self::UserRegister => "user_register".to_string(),
            Self::PasswordChange => "password_change".to_string(),
            Self::ProfileUpdate => "profile_update".to_string(),
            Self::ConversationCreate => "conversation_create".to_string(),
            Self::ConversationDelete => "conversation_delete".to_string(),
            Self::MessageSend => "message_send".to_string(),
            Self::MessageDelete => "message_delete".to_string(),
            Self::MessageEdit => "message_edit".to_string(),
            Self::MessageRecall => "message_recall".to_string(),
            Self::UserBlock => "user_block".to_string(),
            Self::UserUnblock => "user_unblock".to_string(),
            Self::FileUpload => "file_upload".to_string(),
            Self::FileDownload => "file_download".to_string(),
            Self::EncryptionKeyGenerate => "encryption_key_generate".to_string(),
            Self::AdminAction => "admin_action".to_string(),
            Self::Custom(s) => s.clone(),
        }
    }

    /// 从字符串解析
    pub fn from_str(s: &str) -> Self {
        match s {
            "user_login" => Self::UserLogin,
            "user_register" => Self::UserRegister,
            "password_change" => Self::PasswordChange,
            "profile_update" => Self::ProfileUpdate,
            "conversation_create" => Self::ConversationCreate,
            "conversation_delete" => Self::ConversationDelete,
            "message_send" => Self::MessageSend,
            "message_delete" => Self::MessageDelete,
            "message_edit" => Self::MessageEdit,
            "message_recall" => Self::MessageRecall,
            "user_block" => Self::UserBlock,
            "user_unblock" => Self::UserUnblock,
            "file_upload" => Self::FileUpload,
            "file_download" => Self::FileDownload,
            "encryption_key_generate" => Self::EncryptionKeyGenerate,
            "admin_action" => Self::AdminAction,
            other => Self::Custom(other.to_string()),
        }
    }
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 审计日志严重级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditSeverity {
    /// 信息级别（普通操作）
    Info,
    /// 警告级别（敏感操作）
    Warning,
    /// 危险级别（高风险操作）
    Danger,
}

impl AuditSeverity {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Danger => "danger",
        }
    }
}

/// 审计日志条目
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLog {
    /// 日志ID
    pub id: Uuid,
    /// 用户ID
    pub user_id: Uuid,
    /// 操作类型
    pub action: String,
    /// 操作描述
    pub description: String,
    /// 操作严重级别
    pub severity: String,
    /// 资源类型（如 user, conversation, message）
    pub resource_type: Option<String>,
    /// 资源ID
    pub resource_id: Option<String>,
    /// 请求IP地址
    pub ip_address: Option<String>,
    /// User-Agent
    pub user_agent: Option<String>,
    /// 附加数据（JSON格式）
    pub extra_data: Option<serde_json::Value>,
    /// 操作结果（success/failure）
    pub result: String,
    /// 错误信息（如果失败）
    pub error_message: Option<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 创建审计日志请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAuditLog {
    /// 用户ID
    pub user_id: Uuid,
    /// 操作类型
    pub action: AuditAction,
    /// 操作描述
    pub description: String,
    /// 操作严重级别
    pub severity: AuditSeverity,
    /// 资源类型
    pub resource_type: Option<String>,
    /// 资源ID
    pub resource_id: Option<String>,
    /// 请求IP地址
    pub ip_address: Option<String>,
    /// User-Agent
    pub user_agent: Option<String>,
    /// 附加数据
    pub extra_data: Option<serde_json::Value>,
    /// 操作结果
    pub result: AuditResult,
    /// 错误信息
    pub error_message: Option<String>,
}

/// 审计操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditResult {
    Success,
    Failure,
}

impl AuditResult {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
        }
    }
}

/// 审计日志查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogQuery {
    /// 用户ID过滤
    pub user_id: Option<Uuid>,
    /// 操作类型过滤
    pub action: Option<String>,
    /// 严重级别过滤
    pub severity: Option<String>,
    /// 资源类型过滤
    pub resource_type: Option<String>,
    /// 开始时间
    pub start_time: Option<DateTime<Utc>>,
    /// 结束时间
    pub end_time: Option<DateTime<Utc>>,
    /// 结果过滤（success/failure）
    pub result: Option<String>,
    /// 页码（从1开始）
    pub page: Option<i64>,
    /// 每页数量
    pub page_size: Option<i64>,
}

/// 分页审计日志结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogPage {
    /// 日志列表
    pub logs: Vec<AuditLog>,
    /// 总数量
    pub total: i64,
    /// 当前页码
    pub page: i64,
    /// 每页数量
    pub page_size: i64,
    /// 总页数
    pub total_pages: i64,
}

/// 审计日志统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogStats {
    /// 总操作数
    pub total_count: i64,
    /// 成功操作数
    pub success_count: i64,
    /// 失败操作数
    pub failure_count: i64,
    /// 按操作类型统计
    pub by_action: Vec<ActionCount>,
    /// 按严重级别统计
    pub by_severity: Vec<SeverityCount>,
    /// 最近24小时操作数
    pub last_24h_count: i64,
}

/// 操作类型计数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionCount {
    pub action: String,
    pub count: i64,
}

/// 严重级别计数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeverityCount {
    pub severity: String,
    pub count: i64,
}

/// 审计日志存储库
pub struct AuditLogRepository {
    pool: PgPool,
}

impl AuditLogRepository {
    /// 创建新的审计日志存储库
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 初始化审计日志表
    pub async fn init_table(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_logs (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_id UUID NOT NULL,
                action VARCHAR(100) NOT NULL,
                description TEXT NOT NULL,
                severity VARCHAR(20) NOT NULL DEFAULT 'info',
                resource_type VARCHAR(50),
                resource_id VARCHAR(100),
                ip_address VARCHAR(45),
                user_agent TEXT,
                extra_data JSONB,
                result VARCHAR(20) NOT NULL DEFAULT 'success',
                error_message TEXT,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

            -- 创建索引以优化查询性能
            CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON audit_logs(user_id);
            CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);
            CREATE INDEX IF NOT EXISTS idx_audit_logs_severity ON audit_logs(severity);
            CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_audit_logs_resource ON audit_logs(resource_type, resource_id);
            CREATE INDEX IF NOT EXISTS idx_audit_logs_result ON audit_logs(result);
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 记录审计日志
    pub async fn log(&self, entry: &CreateAuditLog) -> Result<AuditLog, sqlx::Error> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let record = sqlx::query_as::<_, AuditLog>(
            r#"
            INSERT INTO audit_logs (id, user_id, action, description, severity, resource_type,
                                     resource_id, ip_address, user_agent, extra_data, result,
                                     error_message, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(entry.user_id)
        .bind(entry.action.as_str())
        .bind(&entry.description)
        .bind(entry.severity.as_str())
        .bind(&entry.resource_type)
        .bind(&entry.resource_id)
        .bind(&entry.ip_address)
        .bind(&entry.user_agent)
        .bind(&entry.extra_data)
        .bind(entry.result.as_str())
        .bind(&entry.error_message)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// 查询审计日志（分页）
    pub async fn query_logs(&self, query: &AuditLogQuery) -> Result<AuditLogPage, sqlx::Error> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
        let offset = (page - 1) * page_size;

        // 构建动态查询
        let mut conditions = Vec::new();
        let mut param_index = 1;
        let mut count_sql = "SELECT COUNT(*) FROM audit_logs WHERE 1=1".to_string();
        let mut select_sql = "SELECT * FROM audit_logs WHERE 1=1".to_string();

        if query.user_id.is_some() {
            param_index += 1;
            let clause = format!(" AND user_id = ${}", param_index);
            count_sql.push_str(&clause);
            select_sql.push_str(&clause);
            conditions.push("user_id");
        }

        if query.action.is_some() {
            param_index += 1;
            let clause = format!(" AND action = ${}", param_index);
            count_sql.push_str(&clause);
            select_sql.push_str(&clause);
            conditions.push("action");
        }

        if query.severity.is_some() {
            param_index += 1;
            let clause = format!(" AND severity = ${}", param_index);
            count_sql.push_str(&clause);
            select_sql.push_str(&clause);
            conditions.push("severity");
        }

        if query.resource_type.is_some() {
            param_index += 1;
            let clause = format!(" AND resource_type = ${}", param_index);
            count_sql.push_str(&clause);
            select_sql.push_str(&clause);
            conditions.push("resource_type");
        }

        if query.start_time.is_some() {
            param_index += 1;
            let clause = format!(" AND created_at >= ${}", param_index);
            count_sql.push_str(&clause);
            select_sql.push_str(&clause);
            conditions.push("start_time");
        }

        if query.end_time.is_some() {
            param_index += 1;
            let clause = format!(" AND created_at <= ${}", param_index);
            count_sql.push_str(&clause);
            select_sql.push_str(&clause);
            conditions.push("end_time");
        }

        if query.result.is_some() {
            param_index += 1;
            let clause = format!(" AND result = ${}", param_index);
            count_sql.push_str(&clause);
            select_sql.push_str(&clause);
            conditions.push("result");
        }

        // 使用简单方式构建查询（避免 sqlx 动态参数绑定的复杂性）
        // 先获取总数
        let total: i64 = self.count_logs(query).await?;
        let total_pages = (total + page_size - 1) / page_size;

        // 获取日志列表
        let logs = self.fetch_logs(query, page_size, offset).await?;

        Ok(AuditLogPage {
            logs,
            total,
            page,
            page_size,
            total_pages,
        })
    }

    /// 统计日志数量
    async fn count_logs(&self, query: &AuditLogQuery) -> Result<i64, sqlx::Error> {
        // 使用条件构建查询
        let mut sql = "SELECT COUNT(*) as count FROM audit_logs WHERE 1=1".to_string();
        let mut binds: Vec<String> = Vec::new();

        if let Some(user_id) = &query.user_id {
            sql.push_str(&format!(" AND user_id = '{}'", user_id));
        }
        if let Some(action) = &query.action {
            sql.push_str(&format!(" AND action = '{}'", action));
        }
        if let Some(severity) = &query.severity {
            sql.push_str(&format!(" AND severity = '{}'", severity));
        }
        if let Some(resource_type) = &query.resource_type {
            sql.push_str(&format!(" AND resource_type = '{}'", resource_type));
        }
        if let Some(start_time) = &query.start_time {
            sql.push_str(&format!(" AND created_at >= '{}'", start_time));
        }
        if let Some(end_time) = &query.end_time {
            sql.push_str(&format!(" AND created_at <= '{}'", end_time));
        }
        if let Some(result) = &query.result {
            sql.push_str(&format!(" AND result = '{}'", result));
        }

        let row: (i64,) = sqlx::query_as(&sql)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.0)
    }

    /// 获取日志列表
    async fn fetch_logs(
        &self,
        query: &AuditLogQuery,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditLog>, sqlx::Error> {
        let mut sql = "SELECT * FROM audit_logs WHERE 1=1".to_string();

        if let Some(user_id) = &query.user_id {
            sql.push_str(&format!(" AND user_id = '{}'", user_id));
        }
        if let Some(action) = &query.action {
            sql.push_str(&format!(" AND action = '{}'", action));
        }
        if let Some(severity) = &query.severity {
            sql.push_str(&format!(" AND severity = '{}'", severity));
        }
        if let Some(resource_type) = &query.resource_type {
            sql.push_str(&format!(" AND resource_type = '{}'", resource_type));
        }
        if let Some(start_time) = &query.start_time {
            sql.push_str(&format!(" AND created_at >= '{}'", start_time));
        }
        if let Some(end_time) = &query.end_time {
            sql.push_str(&format!(" AND created_at <= '{}'", end_time));
        }
        if let Some(result) = &query.result {
            sql.push_str(&format!(" AND result = '{}'", result));
        }

        sql.push_str(&format!(" ORDER BY created_at DESC LIMIT {} OFFSET {}", limit, offset));

        let logs = sqlx::query_as::<_, AuditLog>(&sql)
            .fetch_all(&self.pool)
            .await?;

        Ok(logs)
    }

    /// 获取审计日志统计
    pub async fn get_stats(&self) -> Result<AuditLogStats, sqlx::Error> {
        // 总操作数
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_logs")
            .fetch_one(&self.pool)
            .await?;

        // 成功操作数
        let success: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_logs WHERE result = 'success'",
        )
        .fetch_one(&self.pool)
        .await?;

        // 失败操作数
        let failure: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_logs WHERE result = 'failure'",
        )
        .fetch_one(&self.pool)
        .await?;

        // 按操作类型统计
        let by_action: Vec<ActionCount> = sqlx::query_as(
            "SELECT action, COUNT(*) as count FROM audit_logs GROUP BY action ORDER BY count DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        // 按严重级别统计
        let by_severity: Vec<SeverityCount> = sqlx::query_as(
            "SELECT severity, COUNT(*) as count FROM audit_logs GROUP BY severity ORDER BY count DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        // 最近24小时操作数
        let last_24h: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_logs WHERE created_at >= NOW() - INTERVAL '24 hours'",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(AuditLogStats {
            total_count: total,
            success_count: success,
            failure_count: failure,
            by_action,
            by_severity,
            last_24h_count: last_24h,
        })
    }

    /// 获取指定用户的最近操作
    pub async fn get_user_recent_actions(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AuditLog>, sqlx::Error> {
        let logs = sqlx::query_as::<_, AuditLog>(
            "SELECT * FROM audit_logs WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(logs)
    }

    /// 清理过期日志（默认保留90天）
    pub async fn cleanup_old_logs(&self, retention_days: i64) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM audit_logs WHERE created_at < NOW() - INTERVAL '1 day' * $1",
        )
        .bind(retention_days)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

/// 审计日志便捷记录宏
#[macro_export]
macro_rules! audit_log {
    ($repo:expr, $user_id:expr, $action:expr, $desc:expr, $severity:expr) => {
        $repo
            .log(&CreateAuditLog {
                user_id: $user_id,
                action: $action,
                description: $desc.to_string(),
                severity: $severity,
                resource_type: None,
                resource_id: None,
                ip_address: None,
                user_agent: None,
                extra_data: None,
                result: AuditResult::Success,
                error_message: None,
            })
            .await
    };
    ($repo:expr, $user_id:expr, $action:expr, $desc:expr, $severity:expr, $resource_type:expr, $resource_id:expr) => {
        $repo
            .log(&CreateAuditLog {
                user_id: $user_id,
                action: $action,
                description: $desc.to_string(),
                severity: $severity,
                resource_type: Some($resource_type.to_string()),
                resource_id: Some($resource_id.to_string()),
                ip_address: None,
                user_agent: None,
                extra_data: None,
                result: AuditResult::Success,
                error_message: None,
            })
            .await
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_action_as_str() {
        assert_eq!(AuditAction::UserLogin.as_str(), "user_login");
        assert_eq!(AuditAction::MessageSend.as_str(), "message_send");
        assert_eq!(AuditAction::Custom("test".to_string()).as_str(), "test");
    }

    #[test]
    fn test_audit_action_from_str() {
        assert_eq!(AuditAction::from_str("user_login"), AuditAction::UserLogin);
        assert_eq!(AuditAction::from_str("message_send"), AuditAction::MessageSend);
        assert_eq!(
            AuditAction::from_str("custom_action"),
            AuditAction::Custom("custom_action".to_string())
        );
    }

    #[test]
    fn test_audit_severity_as_str() {
        assert_eq!(AuditSeverity::Info.as_str(), "info");
        assert_eq!(AuditSeverity::Warning.as_str(), "warning");
        assert_eq!(AuditSeverity::Danger.as_str(), "danger");
    }

    #[test]
    fn test_audit_result_as_str() {
        assert_eq!(AuditResult::Success.as_str(), "success");
        assert_eq!(AuditResult::Failure.as_str(), "failure");
    }

    #[test]
    fn test_audit_log_query_defaults() {
        let query = AuditLogQuery {
            user_id: None,
            action: None,
            severity: None,
            resource_type: None,
            start_time: None,
            end_time: None,
            result: None,
            page: None,
            page_size: None,
        };
        assert_eq!(query.page.unwrap_or(1), 1);
        assert_eq!(query.page_size.unwrap_or(20), 20);
    }
}
