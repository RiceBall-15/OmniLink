//! 审计日志 API 处理器
//!
//! 提供审计日志的查询、统计和管理接口。

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use common::audit::{AuditLogRepository, AuditLogQuery as AuditQuery};

/// 审计日志查询请求
#[derive(Debug, Deserialize)]
pub struct GetAuditLogsQuery {
    pub user_id: Option<String>,
    pub action: Option<String>,
    pub severity: Option<String>,
    pub resource_type: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub result: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 审计日志 API 响应
#[derive(Debug, Serialize)]
pub struct AuditLogsResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// 获取审计日志列表
pub async fn get_audit_logs(
    State(pool): State<PgPool>,
    Query(query): Query<GetAuditLogsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo = AuditLogRepository::new(pool);

    let audit_query = AuditQuery {
        user_id: query.user_id.and_then(|s| Uuid::parse_str(&s).ok()),
        action: query.action,
        severity: query.severity,
        resource_type: query.resource_type,
        start_time: query.start_time.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&chrono::Utc))),
        end_time: query.end_time.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&chrono::Utc))),
        result: query.result,
        page: query.page,
        page_size: query.page_size,
    };

    match repo.query_logs(&audit_query).await {
        Ok(page) => Ok(Json(serde_json::json!({
            "success": true,
            "data": {
                "logs": page.logs,
                "total": page.total,
                "page": page.page,
                "page_size": page.page_size,
                "total_pages": page.total_pages
            }
        }))),
        Err(e) => {
            tracing::error!("Failed to query audit logs: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取审计日志统计
pub async fn get_audit_stats(
    State(pool): State<PgPool>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo = AuditLogRepository::new(pool);

    match repo.get_stats().await {
        Ok(stats) => Ok(Json(serde_json::json!({
            "success": true,
            "data": stats
        }))),
        Err(e) => {
            tracing::error!("Failed to get audit stats: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取指定用户的最近操作
#[derive(Debug, Deserialize)]
pub struct UserAuditQuery {
    pub user_id: String,
    pub limit: Option<i64>,
}

pub async fn get_user_audit_logs(
    State(pool): State<PgPool>,
    Query(query): Query<UserAuditQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo = AuditLogRepository::new(pool);

    let user_id = match Uuid::parse_str(&query.user_id) {
        Ok(id) => id,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    let limit = query.limit.unwrap_or(20).clamp(1, 100);

    match repo.get_user_recent_actions(user_id, limit).await {
        Ok(logs) => Ok(Json(serde_json::json!({
            "success": true,
            "data": logs
        }))),
        Err(e) => {
            tracing::error!("Failed to get user audit logs: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 清理旧审计日志请求
#[derive(Debug, Deserialize)]
pub struct CleanupAuditLogsRequest {
    pub retention_days: Option<i64>,
}

/// 清理旧审计日志（管理员接口）
pub async fn cleanup_audit_logs(
    State(pool): State<PgPool>,
    Json(req): Json<CleanupAuditLogsRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo = AuditLogRepository::new(pool);
    let retention_days = req.retention_days.unwrap_or(90);

    match repo.cleanup_old_logs(retention_days).await {
        Ok(deleted_count) => Ok(Json(serde_json::json!({
            "success": true,
            "data": {
                "deleted_count": deleted_count,
                "retention_days": retention_days
            }
        }))),
        Err(e) => {
            tracing::error!("Failed to cleanup audit logs: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
