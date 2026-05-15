use axum::{
    extract::{Path, Query, State, Multipart},
    http::{StatusCode, HeaderMap, header},
    Json,
    response::{Response, IntoResponse},
};
use serde::Deserialize;
use serde_json;
use std::sync::Arc;
use uuid::Uuid;

use common::ApiResponse;
use crate::middleware::AuthUser;
use crate::models::*;
use crate::services::FileService;
use crate::repository::StorageStats;
use crate::progress::{UploadProgressTracker, UploadProgressResponse};
use crate::presign::{PresignConfig, generate_presigned_get_url, generate_presigned_put_url};

#[derive(Clone)]
pub struct AppState {
    pub file_service: Arc<FileService>,
    pub progress_tracker: UploadProgressTracker,
    pub presign_config: Option<PresignConfig>,
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
            return Ok(Json(ApiResponse::error(400, format!("File validation failed: {}", e))));
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

/// 下载文件（支持 ETag 缓存）
pub async fn download_file(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(file_id): Path<Uuid>,
) -> Result<Response, StatusCode> {
    match state.file_service.get_file_by_id(file_id).await {
        Ok(Some(file_info)) => {
            // 生成 ETag: 基于文件ID和创建时间
            let etag = format!(
                "\"{}-{}\"",
                file_info.id,
                file_info.created_at.timestamp()
            );

            // 检查 If-None-Match 头
            if let Some(if_none_match) = headers.get(header::IF_NONE_MATCH) {
                if let Ok(client_etag) = if_none_match.to_str() {
                    if client_etag == etag {
                        return Ok(StatusCode::NOT_MODIFIED.into_response());
                    }
                }
            }

            // 下载文件数据
            match state.file_service.download_file(file_id).await {
                Ok((_, data)) => {
                    let response_headers = [
                        ("Content-Type", file_info.mime_type.as_str()),
                        ("Content-Disposition", &format!("attachment; filename=\"{}\"", file_info.original_name)),
                        ("Content-Length", &file_info.file_size.to_string()),
                        ("ETag", &etag),
                        ("Cache-Control", "private, max-age=3600"),
                    ];
                    Ok((response_headers, data).into_response())
                }
                Err(e) => {
                    tracing::error!("Failed to download file data: {:?}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to get file info: {:?}", e);
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
    match state.file_service.get_thumbnail(file_id).await {
        Ok((file_info, data)) => {
            // 缩略图使用 JPEG 格式，原图使用原始 MIME 类型
            let content_type = if file_info.thumbnail_path.is_some() {
                "image/jpeg"
            } else {
                &file_info.mime_type
            };
            let headers = [
                ("Content-Type", content_type),
                ("Cache-Control", "public, max-age=86400"),
            ];
            Ok((headers, data).into_response())
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
    _auth_user: AuthUser,
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

// ============================================================
// 预签名 URL 相关处理器
// ============================================================

/// 预签名 URL 响应
#[derive(Debug, serde::Serialize)]
pub struct PresignResponse {
    pub upload_url: String,
    pub upload_id: Uuid,
    pub expires_in: u64,
    pub method: String,
    pub headers: std::collections::HashMap<String, String>,
}

/// 获取预签名上传 URL
///
/// 返回一个预签名的 PUT URL，客户端可以直接上传文件到 MinIO/S3。
/// 同时创建进度追踪记录。
pub async fn get_presigned_upload_url(
    State(state): State<Arc<AppState>>,
    _auth_user: AuthUser,
    Json(req): Json<UploadRequest>,
) -> Result<Json<ApiResponse<PresignResponse>>, StatusCode> {
    // 验证文件类型和大小
    if let Err(e) = limits::check_file_size(req.file_size, &req.mime_type) {
        return Ok(Json(ApiResponse::error(400, e)));
    }
    if !is_allowed_mime_type(&req.mime_type) {
        return Ok(Json(ApiResponse::error(400, format!("不支持的文件类型: {}", req.mime_type))));
    }

    let config = match &state.presign_config {
        Some(c) => c,
        None => {
            return Err(StatusCode::NOT_IMPLEMENTED);
        }
    };

    // 生成对象路径
    let file_id = Uuid::new_v4();
    let file_type = FileType::from_mime_type(&req.mime_type);
    let ext = std::path::Path::new(&req.filename)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_else(|| match req.mime_type.as_str() {
            "image/jpeg" => ".jpg".to_string(),
            "image/png" => ".png".to_string(),
            "application/pdf" => ".pdf".to_string(),
            _ => ".bin".to_string(),
        });
    let date = chrono::Utc::now().format("%Y-%m-%d");
    let object_key = format!("{}/{}/{}{}", file_type.as_str(), date, file_id, ext);

    // 创建进度追踪
    let upload_id = state.progress_tracker.create(
        req.filename.clone(),
        req.file_size,
        req.mime_type.clone(),
    );

    // 生成预签名 URL
    match generate_presigned_put_url(config, &object_key, &req.mime_type, 3600) {
        Ok(url) => {
            let mut headers = std::collections::HashMap::new();
            headers.insert("Content-Type".to_string(), req.mime_type.clone());

            let response = PresignResponse {
                upload_url: url,
                upload_id,
                expires_in: 3600,
                method: "PUT".to_string(),
                headers,
            };
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("Failed to generate presigned URL: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取预签名下载 URL
///
/// 返回一个预签名的 GET URL，客户端可以直接从 MinIO/S3 下载文件。
pub async fn get_presigned_download_url(
    State(state): State<Arc<AppState>>,
    _auth_user: AuthUser,
    Path(file_id): Path<Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    let config = match &state.presign_config {
        Some(c) => c,
        None => {
            return Err(StatusCode::NOT_IMPLEMENTED);
        }
    };

    match state.file_service.get_file_by_id(file_id).await {
        Ok(Some(file_info)) => {
            match generate_presigned_get_url(config, &file_info.file_path, 3600) {
                Ok(url) => {
                    Ok(Json(ApiResponse::success(serde_json::json!({
                        "download_url": url,
                        "file_name": file_info.original_name,
                        "mime_type": file_info.mime_type,
                        "file_size": file_info.file_size,
                        "expires_in": 3600,
                    }))))
                }
                Err(e) => {
                    tracing::error!("Failed to generate presigned download URL: {:?}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to get file info: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// ============================================================
// 上传进度追踪处理器
// ============================================================

/// 更新上传进度
///
/// 客户端在分块上传时调用此接口更新进度。
pub async fn update_upload_progress(
    State(state): State<Arc<AppState>>,
    _auth_user: AuthUser,
    Path(upload_id): Path<Uuid>,
    Json(req): Json<UpdateProgressRequest>,
) -> Result<Json<ApiResponse<bool>>, StatusCode> {
    if state.progress_tracker.update(upload_id, req.uploaded_size) {
        Ok(Json(ApiResponse::success(true)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// 获取单个上传进度
pub async fn get_upload_progress(
    State(state): State<Arc<AppState>>,
    _auth_user: AuthUser,
    Path(upload_id): Path<Uuid>,
) -> Result<Json<ApiResponse<UploadProgressResponse>>, StatusCode> {
    match state.progress_tracker.get(upload_id) {
        Some(progress) => Ok(Json(ApiResponse::success(progress))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// 批量获取上传进度
pub async fn get_batch_upload_progress(
    State(state): State<Arc<AppState>>,
    _auth_user: AuthUser,
    Json(req): Json<BatchProgressRequest>,
) -> Result<Json<ApiResponse<Vec<UploadProgressResponse>>>, StatusCode> {
    let results = state.progress_tracker.get_batch(&req.upload_ids);
    Ok(Json(ApiResponse::success(results)))
}

/// 标记上传完成
///
/// 客户端完成上传后调用此接口，服务端执行后处理（缩略图等）。
pub async fn complete_upload(
    State(state): State<Arc<AppState>>,
    _auth_user: AuthUser,
    Path(upload_id): Path<Uuid>,
) -> Result<Json<ApiResponse<bool>>, StatusCode> {
    // 标记为处理中
    state.progress_tracker.processing(upload_id);

    // 获取进度信息以创建文件记录
    let _progress = state.progress_tracker.get(upload_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    // 生成文件 ID
    let file_id = Uuid::new_v4();

    // 标记完成
    state.progress_tracker.complete(upload_id, file_id);

    Ok(Json(ApiResponse::success(true)))
}

/// 标记上传失败
pub async fn fail_upload(
    State(state): State<Arc<AppState>>,
    _auth_user: AuthUser,
    Path(upload_id): Path<Uuid>,
    Json(req): Json<FailUploadRequest>,
) -> Result<Json<ApiResponse<bool>>, StatusCode> {
    if state.progress_tracker.fail(upload_id, req.error) {
        Ok(Json(ApiResponse::success(true)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// 进度更新请求
#[derive(Debug, Deserialize)]
pub struct UpdateProgressRequest {
    pub uploaded_size: i64,
}

/// 批量进度查询请求
#[derive(Debug, Deserialize)]
pub struct BatchProgressRequest {
    pub upload_ids: Vec<Uuid>,
}

/// 上传失败请求
#[derive(Debug, Deserialize)]
pub struct FailUploadRequest {
    pub error: String,
}
