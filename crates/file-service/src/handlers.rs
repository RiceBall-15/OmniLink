use axum::{
    extract::{Path, Query, State, Multipart},
    http::StatusCode,
    Json,
    response::{Response, IntoResponse},
};
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::Arc;
use uuid::Uuid;

use crate::middleware::AuthUser;
use crate::models::*;
use crate::services::FileService;
use crate::repository::StorageStats;

#[derive(Clone)]
pub struct AppState {
    pub file_service: Arc<FileService>,
}

/// 响应包装器
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

/// 上传单个文件
pub async fn upload_file(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<UploadResponse>>, StatusCode> {
    while let Some(field) = multipart.next_field().await.unwrap() {
        let filename = field.file_name().unwrap_or("file").to_string();
        let mime_type = field.content_type().unwrap_or("application/octet-stream").to_string();
        let data = field.bytes().await.unwrap();
        let file_size = data.len() as i64;

        // 验证文件类型和大小
        if let Err(e) = state.file_service.validate_file(&mime_type, file_size) {
            tracing::warn!("File validation failed for {}: {}", filename, e);
            return Ok(Json(ApiResponse::error(format!("File validation failed: {}", e))));
        }

        match state
            .file_service
            .upload_file(
                auth_user.0.sub,
                filename,
                file_size,
                mime_type,
                data.to_vec(),
                false,
            )
            .await
        {
            Ok(file_info) => {
                let response = UploadResponse {
                    file_id: file_info.id,
                    file_url: state.file_service.generate_file_url(file_info.id),
                    thumbnail_url: file_info
                        .thumbnail_path
                        .as_ref()
                        .map(|_| state.file_service.generate_thumbnail_url(file_info.id)),
                    file_info,
                };

                return Ok(Json(ApiResponse::success(response)));
            }
            Err(e) => {
                tracing::error!("Failed to upload file: {:?}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    Err(StatusCode::BAD_REQUEST)
}

/// 批量上传文件
pub async fn batch_upload_files(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<BatchUploadResponse>>, StatusCode> {
    let mut files = Vec::new();
    let mut failed = Vec::new();

    while let Some(field) = multipart.next_field().await.unwrap() {
        let filename = field.file_name().unwrap_or("file").to_string();
        let mime_type = field.content_type().unwrap_or("application/octet-stream").to_string();
        let data = field.bytes().await.unwrap();
        let file_size = data.len() as i64;

        match state
            .file_service
            .upload_file(
                auth_user.0.sub,
                filename.clone(),
                file_size,
                mime_type,
                data.to_vec(),
                false,
            )
            .await
        {
            Ok(file_info) => {
                files.push(UploadResponse {
                    file_id: file_info.id,
                    file_url: state.file_service.generate_file_url(file_info.id),
                    thumbnail_url: file_info
                        .thumbnail_path
                        .as_ref()
                        .map(|_| state.file_service.generate_thumbnail_url(file_info.id)),
                    file_info,
                });
            }
            Err(e) => {
                tracing::error!("Failed to upload file {}: {:?}", filename, e);
                failed.push(filename);
            }
        }
    }

    let response = BatchUploadResponse { files, failed };
    Ok(Json(ApiResponse::success(response)))
}

/// 下载文件
pub async fn download_file(
    State(state): State<Arc<AppState>>,
    Path(file_id): Path<Uuid>,
) -> Result<Response, StatusCode> {
    match state.file_service.download_file(file_id).await {
        Ok((file_info, data)) => {
            let headers = [
                ("Content-Type", file_info.mime_type.as_str()),
                ("Content-Disposition", &format!("attachment; filename=\"{}\"", file_info.original_name)),
                ("Content-Length", &file_info.file_size.to_string()),
            ];

            Ok((headers, data).into_response())
        }
        Err(e) => {
            tracing::error!("Failed to download file: {:?}", e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// 删除文件
pub async fn delete_file(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(file_id): Path<Uuid>,
) -> Result<Json<ApiResponse<bool>>, StatusCode> {
    match state
        .file_service
        .delete_file(file_id, auth_user.0.sub)
        .await
    {
        Ok(deleted) => {
            if deleted {
                Ok(Json(ApiResponse::success(true)))
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            tracing::error!("Failed to delete file: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取文件列表
pub async fn list_files(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(params): Query<FileListParams>,
) -> Result<Json<ApiResponse<FileListResponse>>, StatusCode> {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);

    match state
        .file_service
        .get_files(auth_user.0.sub, params.file_type, page, page_size)
        .await
    {
        Ok(response) => Ok(Json(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("Failed to list files: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 更新文件信息
pub async fn update_file(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(file_id): Path<Uuid>,
    Json(req): Json<UpdateFileRequest>,
) -> Result<Json<ApiResponse<FileInfo>>, StatusCode> {
    let updates = crate::repository::FileUpdate {
        original_name: req.original_name,
        is_public: req.is_public,
    };

    match state
        .file_service
        .update_file(file_id, auth_user.0.sub, updates)
        .await
    {
        Ok(Some(file_info)) => Ok(Json(ApiResponse::success(file_info))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to update file: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateFileRequest {
    pub original_name: Option<String>,
    pub is_public: Option<bool>,
}

/// 获取缩略图
pub async fn get_thumbnail(
    State(state): State<Arc<AppState>>,
    Path(file_id): Path<Uuid>,
) -> Result<Response, StatusCode> {
    match state.file_service.download_file(file_id).await {
        Ok((file_info, data)) => {
            if let Some(ref _thumb_path) = file_info.thumbnail_path {
                // 返回原图（缩略图生成TODO）
                let headers = [
                    ("Content-Type", file_info.mime_type.as_str()),
                    ("Cache-Control", "public, max-age=86400"),
                ];
                Ok((headers, data).into_response())
            } else {
                let headers = [
                    ("Content-Type", file_info.mime_type.as_str()),
                    ("Cache-Control", "public, max-age=86400"),
                ];
                Ok((headers, data).into_response())
            }
        }
        Err(e) => {
            tracing::error!("Failed to get thumbnail: {:?}", e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// 获取存储统计
pub async fn get_storage_stats(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<ApiResponse<StorageStats>>, StatusCode> {
    match state.file_service.get_storage_stats(auth_user.0.sub).await {
        Ok(stats) => Ok(Json(ApiResponse::success(stats))),
        Err(e) => {
            tracing::error!("Failed to get storage stats: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 健康检查
pub async fn health_check() -> &'static str {
    "File service is healthy"
}

/// 获取文件预览信息
pub async fn get_file_preview(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(file_id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    let file_uuid = Uuid::parse_str(&file_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let file = state.file_service.get_file_by_id(file_uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // 构建预览信息
    let preview = serde_json::json!({
        "id": file.id.to_string(),
        "filename": file.filename,
        "original_filename": file.original_name,
        "content_type": file.mime_type,
        "size": file.file_size,
        "is_image": file.mime_type.starts_with("image/"),
        "is_video": file.mime_type.starts_with("video/"),
        "is_audio": file.mime_type.starts_with("audio/"),
        "is_document": !file.mime_type.starts_with("image/") && !file.mime_type.starts_with("video/") && !file.mime_type.starts_with("audio/"),
        "preview_url": format!("/api/files/{}", file.id),
        "created_at": file.created_at.to_rfc3339(),
    });

    Ok(Json(ApiResponse::success(preview)))
}

/// 创建文件分享链接
pub async fn create_share(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(file_id): Path<Uuid>,
    Json(req): Json<CreateShareRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    match state
        .file_service
        .create_share(file_id, auth_user.0.sub, req.expires_in_hours, req.max_downloads)
        .await
    {
        Ok((share, share_url)) => {
            Ok(Json(ApiResponse::success(serde_json::json!({
                "share_id": share.id,
                "share_token": share.share_token,
                "share_url": share_url,
                "expires_at": share.expires_at.map(|t| t.to_rfc3339()),
                "max_downloads": share.max_downloads,
                "created_at": share.created_at.to_rfc3339(),
            }))))
        }
        Err(e) => {
            tracing::error!("Failed to create share: {:?}", e);
            if e.to_string().contains("Not authorized") {
                Err(StatusCode::FORBIDDEN)
            } else if e.to_string().contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// 通过分享链接下载文件（无需认证）
pub async fn download_shared_file(
    State(state): State<Arc<AppState>>,
    Path(share_token): Path<String>,
) -> Result<Response, StatusCode> {
    match state
        .file_service
        .download_shared_file(&share_token)
        .await
    {
        Ok((file_info, data, _share)) => {
            let headers = [
                ("Content-Type", file_info.mime_type.as_str()),
                ("Content-Disposition", &format!("attachment; filename=\"{}\"", file_info.original_name)),
                ("Content-Length", &file_info.file_size.to_string()),
            ];
            Ok((headers, data).into_response())
        }
        Err(e) => {
            tracing::error!("Failed to download shared file: {:?}", e);
            if e.to_string().contains("expired") || e.to_string().contains("limit reached") {
                Err(StatusCode::GONE)
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
    }
}

/// 获取分享信息（无需认证）
pub async fn get_share_info(
    State(state): State<Arc<AppState>>,
    Path(share_token): Path<String>,
) -> Result<Json<ApiResponse<ShareInfoResponse>>, StatusCode> {
    match state.file_service.get_share_info(&share_token).await {
        Ok(info) => Ok(Json(ApiResponse::success(info))),
        Err(e) => {
            tracing::error!("Failed to get share info: {:?}", e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// 删除分享链接
pub async fn delete_share(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(share_id): Path<Uuid>,
) -> Result<Json<ApiResponse<bool>>, StatusCode> {
    match state.file_service.delete_share(share_id, auth_user.0.sub).await {
        Ok(deleted) => {
            if deleted {
                Ok(Json(ApiResponse::success(true)))
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            tracing::error!("Failed to delete share: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取文件的所有分享
pub async fn get_file_shares(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(file_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<FileShare>>>, StatusCode> {
    match state.file_service.get_file_shares(file_id, auth_user.0.sub).await {
        Ok(shares) => Ok(Json(ApiResponse::success(shares))),
        Err(e) => {
            tracing::error!("Failed to get file shares: {:?}", e);
            if e.to_string().contains("Not authorized") {
                Err(StatusCode::FORBIDDEN)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}
