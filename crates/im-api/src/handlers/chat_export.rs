use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::chat_export as db_export;
use crate::models::auth::{ApiError, ApiResponse};
use crate::models::chat_export::ExportFormat;

#[derive(Debug, Deserialize)]
pub struct CreateExportJobRequest {
    pub format: Option<String>,
    /// 开始日期 (ISO 8601 格式，如 2024-01-01T00:00:00Z)
    pub start_date: Option<String>,
    /// 结束日期 (ISO 8601 格式，如 2024-12-31T23:59:59Z)
    pub end_date: Option<String>,
    /// 是否包含系统消息
    pub include_system_messages: Option<bool>,
}

/// 创建导出任务
#[utoipa::path(
    post,
    path = "/api/im/exports",
    tag = "chat-export",
    responses(
        (status = 201, description = "创建成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn create_export_job_handler(
    State(pool): State<PgPool>,
    auth: crate::middleware::auth::AuthUser,
    Path(conversation_id): Path<String>,
    Json(req): Json<CreateExportJobRequest>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::<serde_json::Value> {
                    success: false,
                    data: None,
                    error: Some(ApiError {
                        code: "INVALID_USER".to_string(),
                        message: "无效的用户ID".to_string(),
                    }),
                }),
            );
        }
    };

    let conv_id = match Uuid::parse_str(&conversation_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value> {
                    success: false,
                    data: None,
                    error: Some(ApiError {
                        code: "INVALID_ID".to_string(),
                        message: "无效的会话ID".to_string(),
                    }),
                }),
            );
        }
    };

    let format = match req.format.as_deref() {
        Some("json") => ExportFormat::Json,
        Some("csv") => ExportFormat::Csv,
        Some("txt") => ExportFormat::Txt,
        Some(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value> {
                    success: false,
                    data: None,
                    error: Some(ApiError {
                        code: "INVALID_FORMAT".to_string(),
                        message: "不支持的导出格式，支持 json、csv 和 txt".to_string(),
                    }),
                }),
            );
        }
        None => ExportFormat::Json,
    };

    match db_export::create_export_job(&pool, user_id, conv_id, format).await {
        Ok(job) => (
            StatusCode::CREATED,
            Json(ApiResponse::<serde_json::Value> {
                success: true,
                data: Some(serde_json::json!({
                    "job_id": job.id.to_string(),
                    "conversation_id": job.conversation_id.to_string(),
                    "format": job.format.to_string(),
                    "status": job.status.to_string(),
                    "created_at": job.created_at.to_rfc3339(),
                })),
                error: None,
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value> {
                success: false,
                data: None,
                error: Some(ApiError {
                    code: "DB_ERROR".to_string(),
                    message: format!("创建导出任务失败: {}", e),
                }),
            }),
        ),
    }
}

/// 查询导出任务状态
pub async fn get_export_job_handler(
    State(pool): State<PgPool>,
    auth: crate::middleware::auth::AuthUser,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::<serde_json::Value> {
                    success: false,
                    data: None,
                    error: Some(ApiError {
                        code: "INVALID_USER".to_string(),
                        message: "无效的用户ID".to_string(),
                    }),
                }),
            );
        }
    };

    let job_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value> {
                    success: false,
                    data: None,
                    error: Some(ApiError {
                        code: "INVALID_ID".to_string(),
                        message: "无效的任务ID".to_string(),
                    }),
                }),
            );
        }
    };

    match db_export::get_export_job(&pool, job_id, user_id).await {
        Ok(Some(job)) => {
            let mut result = serde_json::json!({
                "job_id": job.id.to_string(),
                "conversation_id": job.conversation_id.to_string(),
                "format": job.format.to_string(),
                "status": job.status.to_string(),
                "message_count": job.message_count,
                "file_size": job.file_size,
                "created_at": job.created_at.to_rfc3339(),
            });
            if let Some(ref file_path) = job.file_path {
                result["file_path"] = serde_json::json!(file_path);
            }
            if let Some(ref completed_at) = job.completed_at {
                result["completed_at"] = serde_json::json!(completed_at.to_rfc3339());
            }
            if let Some(ref error_msg) = job.error_message {
                result["error_message"] = serde_json::json!(error_msg);
            }
            (
                StatusCode::OK,
                Json(ApiResponse::<serde_json::Value> {
                    success: true,
                    data: Some(result),
                    error: None,
                }),
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<serde_json::Value> {
                success: false,
                data: None,
                error: Some(ApiError {
                    code: "NOT_FOUND".to_string(),
                    message: "导出任务不存在".to_string(),
                }),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value> {
                success: false,
                data: None,
                error: Some(ApiError {
                    code: "DB_ERROR".to_string(),
                    message: format!("查询失败: {}", e),
                }),
            }),
        ),
    }
}

/// 下载导出文件
#[utoipa::path(
    get,
    path = "/api/im/exports/{id}/download",
    tag = "chat-export",
    params(("id" = String, Path, description = "导出任务ID")),
    responses(
        (status = 200, description = "下载成功"),
    )
)]
pub async fn download_export_file_handler(
    State(pool): State<PgPool>,
    auth: crate::middleware::auth::AuthUser,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::<serde_json::Value> {
                    success: false,
                    data: None,
                    error: Some(ApiError {
                        code: "INVALID_USER".to_string(),
                        message: "无效的用户ID".to_string(),
                    }),
                }),
            )
                .into_response();
        }
    };

    let job_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value> {
                    success: false,
                    data: None,
                    error: Some(ApiError {
                        code: "INVALID_ID".to_string(),
                        message: "无效的任务ID".to_string(),
                    }),
                }),
            )
                .into_response();
        }
    };

    match db_export::get_export_job(&pool, job_id, user_id).await {
        Ok(Some(job)) => {
            if !job.is_completed() || job.file_path.is_none() {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<serde_json::Value> {
                        success: false,
                        data: None,
                        error: Some(ApiError {
                            code: "NOT_READY".to_string(),
                            message: "导出任务尚未完成".to_string(),
                        }),
                    }),
                )
                    .into_response();
            }

            let file_path = job.file_path.unwrap();
            match tokio::fs::read_to_string(&file_path).await {
                Ok(content) => {
                    let content_type = job.format.content_type();
                    axum::response::Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", content_type)
                        .header("Content-Disposition", format!("attachment; filename=\"export_{}.{}\"", job.id, job.format.to_string()))
                        .body(axum::body::Body::from(content))
                        .unwrap()
                }
                Err(_) => (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::<serde_json::Value> {
                        success: false,
                        data: None,
                        error: Some(ApiError {
                            code: "FILE_NOT_FOUND".to_string(),
                            message: "导出文件不存在或已被删除".to_string(),
                        }),
                    }),
                )
                    .into_response(),
            }
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<serde_json::Value> {
                success: false,
                data: None,
                error: Some(ApiError {
                    code: "NOT_FOUND".to_string(),
                    message: "导出任务不存在".to_string(),
                }),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value> {
                success: false,
                data: None,
                error: Some(ApiError {
                    code: "DB_ERROR".to_string(),
                    message: format!("查询失败: {}", e),
                }),
            }),
        )
            .into_response(),
    }
}

/// 获取用户导出任务列表
pub async fn list_user_export_jobs_handler(
    State(pool): State<PgPool>,
    auth: crate::middleware::auth::AuthUser,
    Query(params): Query<ListExportQuery>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::<serde_json::Value> {
                    success: false,
                    data: None,
                    error: Some(ApiError {
                        code: "INVALID_USER".to_string(),
                        message: "无效的用户ID".to_string(),
                    }),
                }),
            );
        }
    };

    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).min(100);

    match db_export::get_user_export_jobs(&pool, user_id, page, page_size).await {
        Ok(jobs) => {
            let jobs_json: Vec<serde_json::Value> = jobs
                .iter()
                .map(|job| {
                    let mut v = serde_json::json!({
                        "job_id": job.id.to_string(),
                        "conversation_id": job.conversation_id.to_string(),
                        "format": job.format.to_string(),
                        "status": job.status.to_string(),
                        "message_count": job.message_count,
                        "file_size": job.file_size,
                        "created_at": job.created_at.to_rfc3339(),
                    });
                    if let Some(ref completed_at) = job.completed_at {
                        v["completed_at"] = serde_json::json!(completed_at.to_rfc3339());
                    }
                    v
                })
                .collect();

            (
                StatusCode::OK,
                Json(ApiResponse::<serde_json::Value> {
                    success: true,
                    data: Some(serde_json::json!({
                        "jobs": jobs_json,
                        "page": page,
                        "page_size": page_size,
                    })),
                    error: None,
                }),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value> {
                success: false,
                data: None,
                error: Some(ApiError {
                    code: "DB_ERROR".to_string(),
                    message: format!("查询失败: {}", e),
                }),
            }),
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct ListExportQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}
