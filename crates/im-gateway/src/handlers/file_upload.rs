//! 文件上传 API 处理器
//!
//! 提供文件上传、下载、删除、列表等端点。
//! 支持的文件类型：图片、文档、音频、视频。
//! 使用本地文件存储（可扩展为 MinIO）。

use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use crate::middleware::AuthUser;
use common::ApiResponse;

// ─── 配置 ───────────────────────────────────────────────────

/// 文件上传配置
#[derive(Debug, Clone)]
pub struct FileUploadConfig {
    /// 上传根目录
    pub upload_dir: PathBuf,
    /// 最大文件大小（字节）
    pub max_file_size: usize,
    /// 允许的 MIME 类型前缀
    pub allowed_types: Vec<String>,
    /// URL 前缀
    pub url_prefix: String,
}

impl Default for FileUploadConfig {
    fn default() -> Self {
        Self {
            upload_dir: PathBuf::from("./uploads"),
            max_file_size: 50 * 1024 * 1024, // 50MB
            allowed_types: vec![
                "image/".to_string(),
                "video/".to_string(),
                "audio/".to_string(),
                "application/pdf".to_string(),
                "application/msword".to_string(),
                "application/vnd.openxmlformats".to_string(),
                "text/".to_string(),
            ],
            url_prefix: "/api/files".to_string(),
        }
    }
}

// ─── 文件元数据 ──────────────────────────────────────────────

/// 文件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub file_id: Uuid,
    pub original_name: String,
    pub stored_name: String,
    pub mime_type: String,
    pub size: u64,
    pub uploader_id: Uuid,
    pub created_at: i64,
    pub url: String,
}

/// 文件上传响应
#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub file_id: Uuid,
    pub url: String,
    pub original_name: String,
    pub mime_type: String,
    pub size: u64,
}

/// 文件列表查询参数
#[derive(Debug, Deserialize)]
pub struct FileListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub mime_type_prefix: Option<String>,
}

/// 文件列表响应
#[derive(Debug, Serialize)]
pub struct FileListResponse {
    pub files: Vec<FileMetadata>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

// ─── 文件服务 ───────────────────────────────────────────────

/// 文件上传服务
pub struct FileService {
    config: FileUploadConfig,
    /// 文件元数据存储（内存版，生产环境应用数据库）
    metadata: tokio::sync::RwLock<HashMap<Uuid, FileMetadata>>,
}

impl FileService {
    pub fn new(config: FileUploadConfig) -> Self {
        Self {
            config,
            metadata: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    /// 初始化上传目录
    pub async fn init(&self) -> std::io::Result<()> {
        tokio::fs::create_dir_all(&self.config.upload_dir).await?;
        tracing::info!("File upload directory: {:?}", self.config.upload_dir);
        Ok(())
    }

    /// 验证文件类型
    fn validate_mime_type(&self, mime_type: &str) -> bool {
        self.config
            .allowed_types
            .iter()
            .any(|allowed| mime_type.starts_with(allowed))
    }

    /// 生成存储文件名
    fn generate_stored_name(original_name: &str) -> String {
        let ext = std::path::Path::new(original_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin");
        format!("{}.{}", Uuid::new_v4(), ext)
    }

    /// 上传文件
    pub async fn upload(
        &self,
        file_name: String,
        mime_type: String,
        data: Vec<u8>,
        uploader_id: Uuid,
    ) -> Result<UploadResponse, String> {
        // 验证文件大小
        if data.len() > self.config.max_file_size {
            return Err(format!(
                "File too large: {} bytes (max: {} bytes)",
                data.len(),
                self.config.max_file_size
            ));
        }

        // 验证文件类型
        if !self.validate_mime_type(&mime_type) {
            return Err(format!("File type not allowed: {}", mime_type));
        }

        // 生成存储路径
        let stored_name = Self::generate_stored_name(&file_name);
        let date_dir = Utc::now().format("%Y/%m/%d").to_string();
        let file_path = self.config.upload_dir.join(&date_dir).join(&stored_name);

        // 确保目录存在
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        // 写入文件
        tokio::fs::write(&file_path, &data)
            .await
            .map_err(|e| format!("Failed to write file: {}", e))?;

        // 生成元数据
        let file_id = Uuid::new_v4();
        let url = format!("{}/{}/{}", self.config.url_prefix, date_dir, stored_name);
        let original_name = file_name.clone();
        let mime = mime_type.clone();
        let size = data.len() as u64;

        let metadata = FileMetadata {
            file_id,
            original_name: file_name,
            stored_name: stored_name.clone(),
            mime_type,
            size,
            uploader_id,
            created_at: Utc::now().timestamp(),
            url: url.clone(),
        };

        // 保存元数据
        {
            let mut meta = self.metadata.write().await;
            meta.insert(file_id, metadata);
        }

        tracing::info!(
            "File uploaded: {} ({} bytes) -> {:?}",
            file_id,
            data.len(),
            file_path
        );

        Ok(UploadResponse {
            file_id,
            url,
            original_name,
            mime_type: mime,
            size,
        })
    }

    /// 获取文件元数据
    pub async fn get_metadata(&self, file_id: &Uuid) -> Option<FileMetadata> {
        let meta = self.metadata.read().await;
        meta.get(file_id).cloned()
    }

    /// 获取文件内容路径
    pub async fn get_file_path(&self, file_id: &Uuid) -> Option<PathBuf> {
        let meta = self.metadata.read().await;
        meta.get(file_id).map(|m| {
            let date_dir = chrono::NaiveDateTime::from_timestamp_opt(m.created_at, 0)
                .map(|dt| dt.format("%Y/%m/%d").to_string())
                .unwrap_or_else(|| "unknown".to_string());
            self.config.upload_dir.join(&date_dir).join(&m.stored_name)
        })
    }

    /// 删除文件
    pub async fn delete(&self, file_id: &Uuid, user_id: &Uuid) -> Result<(), String> {
        let metadata = {
            let meta = self.metadata.read().await;
            meta.get(file_id).cloned()
        };

        let metadata = metadata.ok_or("File not found")?;

        // 验证所有权
        if metadata.uploader_id != *user_id {
            return Err("Not authorized to delete this file".to_string());
        }

        // 删除物理文件
        if let Some(path) = self.get_file_path(file_id).await {
            if path.exists() {
                tokio::fs::remove_file(&path)
                    .await
                    .map_err(|e| format!("Failed to delete file: {}", e))?;
            }
        }

        // 删除元数据
        {
            let mut meta = self.metadata.write().await;
            meta.remove(file_id);
        }

        tracing::info!("File deleted: {}", file_id);
        Ok(())
    }

    /// 列出用户的文件
    pub async fn list_user_files(
        &self,
        user_id: &Uuid,
        page: u32,
        page_size: u32,
        mime_type_prefix: Option<&str>,
    ) -> FileListResponse {
        let meta = self.metadata.read().await;
        let mut files: Vec<FileMetadata> = meta
            .values()
            .filter(|m| m.uploader_id == *user_id)
            .filter(|m| {
                mime_type_prefix
                    .map(|prefix| m.mime_type.starts_with(prefix))
                    .unwrap_or(true)
            })
            .cloned()
            .collect();

        files.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let total = files.len() as u64;
        let start = (page * page_size) as usize;
        let end = (start + page_size as usize).min(files.len());
        let paged = if start < files.len() {
            files[start..end].to_vec()
        } else {
            vec![]
        };

        FileListResponse {
            files: paged,
            total,
            page,
            page_size,
        }
    }
}

// ─── HTTP 处理器 ─────────────────────────────────────────────

/// 上传文件
pub async fn upload_file(
    State(file_service): State<Arc<FileService>>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    let mut file_name = String::new();
    let mut mime_type = String::new();
    let mut data = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" || file_name.is_empty() {
            file_name = field
                .file_name()
                .unwrap_or("unknown")
                .to_string();
            mime_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();
            data = field
                .bytes()
                .await
                .map_err(|_| StatusCode::BAD_REQUEST)?
                .to_vec();
        }
    }

    if data.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    match file_service.upload(file_name, mime_type, data, auth.0.sub).await {
        Ok(response) => Ok((StatusCode::CREATED, Json(ApiResponse::success(response)))),
        Err(e) => {
            tracing::error!("File upload failed: {}", e);
            if e.contains("too large") {
                Err(StatusCode::PAYLOAD_TOO_LARGE)
            } else if e.contains("not allowed") {
                Err(StatusCode::UNSUPPORTED_MEDIA_TYPE)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// 下载/查看文件
pub async fn download_file(
    State(file_service): State<Arc<FileService>>,
    Path(file_id): Path<Uuid>,
) -> Result<Response, StatusCode> {
    let metadata = file_service
        .get_metadata(&file_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let file_path = file_service
        .get_file_path(&file_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    if !file_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    let content = tokio::fs::read(&file_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, &metadata.mime_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{}\"", metadata.original_name),
        )
        .header(header::CONTENT_LENGTH, content.len())
        .body(Body::from(content))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// 删除文件
pub async fn delete_file(
    State(file_service): State<Arc<FileService>>,
    Path(file_id): Path<Uuid>,
    auth: AuthUser,
) -> Result<impl IntoResponse, StatusCode> {
    match file_service.delete(&file_id, &auth.0.sub).await {
        Ok(()) => Ok(Json(ApiResponse::<()>::success_with_message(
            "File deleted".to_string(),
        ))),
        Err(e) => {
            if e.contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else if e.contains("Not authorized") {
                Err(StatusCode::FORBIDDEN)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// 列出用户文件
pub async fn list_files(
    State(file_service): State<Arc<FileService>>,
    auth: AuthUser,
    Query(query): Query<FileListQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let page = query.page.unwrap_or(0);
    let page_size = query.page_size.unwrap_or(20).min(100);

    let response = file_service
        .list_user_files(
            &auth.0.sub,
            page,
            page_size,
            query.mime_type_prefix.as_deref(),
        )
        .await;

    Ok(Json(ApiResponse::success(response)))
}

/// 获取文件元数据
pub async fn get_file_info(
    State(file_service): State<Arc<FileService>>,
    Path(file_id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    match file_service.get_metadata(&file_id).await {
        Some(metadata) => Ok(Json(ApiResponse::success(metadata))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

// ─── 辅助函数 ───────────────────────────────────────────────

use common::ApiResponse as ApiResp;

/// 创建文件上传路由
pub fn file_routes(file_service: Arc<FileService>) -> axum::Router {
    use axum::routing::{delete, get, post};

    axum::Router::new()
        .route("/upload", post(upload_file))
        .route("/list", get(list_files))
        .route("/{file_id}", get(download_file).delete(delete_file))
        .route("/{file_id}/info", get(get_file_info))
        .with_state(file_service)
}

// ─── 测试 ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> FileUploadConfig {
        FileUploadConfig {
            upload_dir: PathBuf::from("/tmp/omnilink-test-uploads"),
            max_file_size: 1024 * 1024, // 1MB
            allowed_types: vec!["image/".to_string(), "application/pdf".to_string()],
            url_prefix: "/api/files".to_string(),
        }
    }

    #[tokio::test]
    async fn test_file_service_upload() {
        let service = FileService::new(test_config());
        service.init().await.unwrap();

        let user_id = Uuid::new_v4();
        let data = b"Hello, World!";

        let result = service
            .upload(
                "test.txt".to_string(),
                "image/png".to_string(),
                data.to_vec(),
                user_id,
            )
            .await;

        assert!(result.is_ok());
        let resp = result.unwrap();
        assert_eq!(resp.size, 13);
        assert!(resp.url.starts_with("/api/files/"));
    }

    #[tokio::test]
    async fn test_file_type_validation() {
        let service = FileService::new(test_config());
        service.init().await.unwrap();

        let user_id = Uuid::new_v4();

        // 不允许的类型
        let result = service
            .upload(
                "test.exe".to_string(),
                "application/x-executable".to_string(),
                b"binary".to_vec(),
                user_id,
            )
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not allowed"));
    }

    #[tokio::test]
    async fn test_file_size_limit() {
        let service = FileService::new(test_config());
        service.init().await.unwrap();

        let user_id = Uuid::new_v4();
        let big_data = vec![0u8; 2 * 1024 * 1024]; // 2MB

        let result = service
            .upload(
                "big.png".to_string(),
                "image/png".to_string(),
                big_data,
                user_id,
            )
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too large"));
    }

    #[tokio::test]
    async fn test_file_list_and_delete() {
        let service = FileService::new(test_config());
        service.init().await.unwrap();

        let user_id = Uuid::new_v4();

        // 上传两个文件
        let r1 = service
            .upload(
                "a.png".to_string(),
                "image/png".to_string(),
                b"data a".to_vec(),
                user_id,
            )
            .await
            .unwrap();

        let _r2 = service
            .upload(
                "b.pdf".to_string(),
                "application/pdf".to_string(),
                b"data b".to_vec(),
                user_id,
            )
            .await
            .unwrap();

        // 列出文件
        let list = service.list_user_files(&user_id, 0, 10, None).await;
        assert_eq!(list.total, 2);
        assert_eq!(list.files.len(), 2);

        // 按类型过滤
        let filtered = service
            .list_user_files(&user_id, 0, 10, Some("image/"))
            .await;
        assert_eq!(filtered.total, 1);

        // 删除文件
        let result = service.delete(&r1.file_id, &user_id).await;
        assert!(result.is_ok());

        // 删除后只剩 1 个
        let list = service.list_user_files(&user_id, 0, 10, None).await;
        assert_eq!(list.total, 1);
    }

    #[tokio::test]
    async fn test_file_delete_unauthorized() {
        let service = FileService::new(test_config());
        service.init().await.unwrap();

        let user_id = Uuid::new_v4();
        let other_user = Uuid::new_v4();

        let resp = service
            .upload(
                "test.png".to_string(),
                "image/png".to_string(),
                b"data".to_vec(),
                user_id,
            )
            .await
            .unwrap();

        let result = service.delete(&resp.file_id, &other_user).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Not authorized"));
    }

    #[test]
    fn test_stored_name_generation() {
        let name1 = FileService::generate_stored_name("photo.jpg");
        let name2 = FileService::generate_stored_name("photo.jpg");
        assert_ne!(name1, name2);
        assert!(name1.ends_with(".jpg"));
    }
}
