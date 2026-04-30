use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 文件信息
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FileInfo {
    pub id: Uuid,
    pub user_id: Uuid,
    pub filename: String,
    pub original_name: String,
    pub file_path: String,
    pub file_size: i64,
    pub mime_type: String,
    pub file_type: String, // 'image', 'video', 'document', 'other'
    pub width: Option<i32>,  // 图片宽度
    pub height: Option<i32>, // 图片高度
    pub duration: Option<i32>, // 视频时长(秒)
    pub thumbnail_path: Option<String>, // 缩略图路径
    pub storage_type: String, // 'local', 'minio', 's3'
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
}

/// 文件上传请求
#[derive(Debug, Deserialize)]
pub struct UploadRequest {
    pub filename: String,
    pub file_size: i64,
    pub mime_type: String,
    pub is_public: Option<bool>,
}

/// 文件上传响应
#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub file_id: Uuid,
    pub file_url: String,
    pub thumbnail_url: Option<String>,
    pub file_info: FileInfo,
}

/// 批量上传请求
#[derive(Debug, Deserialize)]
pub struct BatchUploadRequest {
    pub files: Vec<UploadRequest>,
}

/// 批量上传响应
#[derive(Debug, Serialize)]
pub struct BatchUploadResponse {
    pub files: Vec<UploadResponse>,
    pub failed: Vec<String>,
}

/// 文件下载请求
#[derive(Debug, Deserialize)]
pub struct DownloadRequest {
    pub file_id: Uuid,
}

/// 文件删除请求
#[derive(Debug, Deserialize)]
pub struct DeleteRequest {
    pub file_id: Uuid,
}

/// 文件列表查询参数
#[derive(Debug, Deserialize)]
pub struct FileListParams {
    pub file_type: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 文件列表响应
#[derive(Debug, Serialize)]
pub struct FileListResponse {
    pub files: Vec<FileInfo>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

/// 图片处理参数
#[derive(Debug, Deserialize)]
pub struct ImageProcessParams {
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub quality: Option<i32>, // 1-100
    pub format: Option<String>, // 'jpg', 'png', 'webp'
}

/// 视频处理参数
#[derive(Debug, Deserialize)]
pub struct VideoProcessParams {
    pub format: Option<String>, // 'mp4', 'webm'
    pub quality: Option<String>, // 'low', 'medium', 'high'
    pub resolution: Option<String>, // '720p', '1080p'
    pub thumbnail: Option<bool>,
}