use axum::{
    extract::{Path, Query, State, Multipart},
    http::StatusCode,
    Json,
    response::{Response, IntoResponse},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use common::auth::Claims;
use crate::models::*;
use crate::services::FileService;

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
    State(state): State<AppState>,
    claims: Claims,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<UploadResponse>>, StatusCode> {
    while let Some(field) = multipart.next_field().await.unwrap() {
        let filename = field.file_name().unwrap_or("file").to_string();
        let file_size = field.bytes().await.unwrap().len() as i64;
        let data = field.bytes().await.unwrap();
        let mime_type = field.content_type().unwrap_or("application/octet-stream").to_string();

        match state
            .file_service
            .upload_file(
                claims.user_id,
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
    State(state): State<AppState>,
    claims: Claims,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<BatchUploadResponse>>, StatusCode> {
    let mut files = Vec::new();
    let mut failed = Vec::new();

    while let Some(field) = multipart.next_field().await.unwrap() {
        let filename = field.file_name().unwrap_or("file").to_string();
        let file_size = field.bytes().await.unwrap().len() as i64;
        let data = field.bytes().await.unwrap();
        let mime_type = field.content_type().unwrap_or("application/octet-stream").to_string();

        match state
            .file_service
            .upload_file(
                claims.user_id,
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
    State(state): State<AppState>,
    Path(file_id): Path<Uuid>,
) -> Result<Response, StatusCode> {
    match state.file_service.download_file(file_id).await {
        Ok((file_info, data)) => {
            let headers = [
                ("Content-Type", file_info.mime_type.as_str()),
                ("Content-Disposition", &format!("attachment; filename="{}"", file_info.original_name)),
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
    State(state): State<AppState>,
    claims: Claims,
    Path(file_id): Path<Uuid>,
) -> Result<Json<ApiResponse<bool>>, StatusCode> {
    match state
        .file_service
        .delete_file(file_id, claims.user_id)
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
    State(state): State<AppState>,
    claims: Claims,
    Query(params): Query<FileListParams>,
) -> Result<Json<ApiResponse<FileListResponse>>, StatusCode> {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);

    match state
        .file_service
        .get_files(claims.user_id, params.file_type, page, page_size)
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
    State(state): State<AppState>,
    claims: Claims,
    Path(file_id): Path<Uuid>,
    Json(req): Json<UpdateFileRequest>,
) -> Result<Json<ApiResponse<FileInfo>>, StatusCode> {
    let updates = crate::repository::FileUpdate {
        original_name: req.original_name,
        is_public: req.is_public,
    };

    match state
        .file_service
        .update_file(file_id, claims.user_id, updates)
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
    State(state): State<AppState>,
    Path(file_id): Path<Uuid>,
) -> Result<Response, StatusCode> {
    match state.file_service.download_file(file_id).await {
        Ok((file_info, data)) => {
            if let Some(thumb_path) = file_info.thumbnail_path {
                // 读取缩略图
                match state.file_service._read_file(&thumb_path).await {
                    Ok(thumb_data) => {
                        let headers = [
                            ("Content-Type", "image/jpeg"),
                            ("Cache-Control", "public, max-age=31536000"),
                        ];
                        return Ok((headers, thumb_data).into_response());
                    }
                    Err(e) => {
                        tracing::error!("Failed to read thumbnail: {:?}", e);
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                }
            } else {
                // 返回原图
                let headers = [
                    ("Content-Type", file_info.mime_type.as_str()),
                    ("Cache-Control", "public, max-age=86400"),
                ];
                return Ok((headers, data).into_response());
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
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<ApiResponse<StorageStats>>, StatusCode> {
    match state.file_service.get_storage_stats(claims.user_id).await {
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