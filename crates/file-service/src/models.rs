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
    pub format: Option<String>,     // 'mp4', 'webm'
    pub quality: Option<String>,    // 'low', 'medium', 'high'
    pub resolution: Option<String>, // '720p', '1080p'
    pub thumbnail: Option<bool>,
}

/// 文件分享记录
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FileShare {
    pub id: Uuid,
    pub file_id: Uuid,
    pub created_by: Uuid,
    pub share_token: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub max_downloads: Option<i32>,
    pub download_count: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// 创建分享请求
#[derive(Debug, Deserialize)]
pub struct CreateShareRequest {
    pub expires_in_hours: Option<i64>,  // 过期时间（小时），None表示永不过期
    pub max_downloads: Option<i32>,     // 最大下载次数，None表示不限制
}

/// 分享信息响应
#[derive(Debug, Serialize)]
pub struct ShareInfoResponse {
    pub share_id: Uuid,
    pub file_id: Uuid,
    pub file_name: String,
    pub file_size: i64,
    pub mime_type: String,
    pub created_by: Uuid,
    pub expires_at: Option<String>,
    pub max_downloads: Option<i32>,
    pub download_count: i32,
    pub is_expired: bool,
    pub is_download_limit_reached: bool,
    pub share_url: String,
}

/// 文件类型枚举
#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    Image,
    Video,
    Document,
    Audio,
    Other,
}

impl FileType {
    pub fn from_mime_type(mime: &str) -> Self {
        if mime.starts_with("image/") {
            FileType::Image
        } else if mime.starts_with("video/") {
            FileType::Video
        } else if mime.starts_with("audio/") {
            FileType::Audio
        } else if mime.starts_with("application/pdf")
            || mime.starts_with("application/msword")
            || mime.starts_with("text/")
        {
            FileType::Document
        } else {
            FileType::Other
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            FileType::Image => "image",
            FileType::Video => "video",
            FileType::Audio => "audio",
            FileType::Document => "document",
            FileType::Other => "other",
        }
    }
}

/// 文件大小限制常量
pub mod limits {
    pub const MAX_FILE_SIZE: i64 = 100 * 1024 * 1024; // 100MB
    pub const MAX_IMAGE_SIZE: i64 = 10 * 1024 * 1024; // 10MB
    pub const MAX_VIDEO_SIZE: i64 = 50 * 1024 * 1024; // 50MB
    pub const MAX_DOCUMENT_SIZE: i64 = 20 * 1024 * 1024; // 20MB

    pub fn check_file_size(file_size: i64, mime_type: &str) -> Result<(), String> {
        let max_size = if mime_type.starts_with("image/") {
            MAX_IMAGE_SIZE
        } else if mime_type.starts_with("video/") {
            MAX_VIDEO_SIZE
        } else if mime_type.starts_with("application/pdf") || mime_type.starts_with("text/") {
            MAX_DOCUMENT_SIZE
        } else {
            MAX_FILE_SIZE
        };

        if file_size > max_size {
            Err(format!(
                "File size {} exceeds maximum allowed size {} for type {}",
                file_size, max_size, mime_type
            ))
        } else if file_size <= 0 {
            Err("File size must be positive".to_string())
        } else {
            Ok(())
        }
    }
}

/// 允许的MIME类型
pub const ALLOWED_MIME_TYPES: &[&str] = &[
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
    "video/mp4",
    "video/webm",
    "audio/mpeg",
    "audio/wav",
    "application/pdf",
    "application/msword",
    "text/plain",
];

pub fn is_allowed_mime_type(mime_type: &str) -> bool {
    ALLOWED_MIME_TYPES.contains(&mime_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    // === FileType 测试 ===

    #[test]
    fn test_file_type_from_image_mime() {
        assert_eq!(FileType::from_mime_type("image/jpeg"), FileType::Image);
        assert_eq!(FileType::from_mime_type("image/png"), FileType::Image);
        assert_eq!(FileType::from_mime_type("image/gif"), FileType::Image);
        assert_eq!(FileType::from_mime_type("image/webp"), FileType::Image);
    }

    #[test]
    fn test_file_type_from_video_mime() {
        assert_eq!(FileType::from_mime_type("video/mp4"), FileType::Video);
        assert_eq!(FileType::from_mime_type("video/webm"), FileType::Video);
    }

    #[test]
    fn test_file_type_from_audio_mime() {
        assert_eq!(FileType::from_mime_type("audio/mpeg"), FileType::Audio);
        assert_eq!(FileType::from_mime_type("audio/wav"), FileType::Audio);
    }

    #[test]
    fn test_file_type_from_document_mime() {
        assert_eq!(FileType::from_mime_type("application/pdf"), FileType::Document);
        assert_eq!(FileType::from_mime_type("text/plain"), FileType::Document);
        assert_eq!(
            FileType::from_mime_type("application/msword"),
            FileType::Document
        );
    }

    #[test]
    fn test_file_type_from_unknown_mime() {
        assert_eq!(FileType::from_mime_type("application/octet-stream"), FileType::Other);
        assert_eq!(FileType::from_mime_type("unknown/type"), FileType::Other);
    }

    #[test]
    fn test_file_type_as_str() {
        assert_eq!(FileType::Image.as_str(), "image");
        assert_eq!(FileType::Video.as_str(), "video");
        assert_eq!(FileType::Audio.as_str(), "audio");
        assert_eq!(FileType::Document.as_str(), "document");
        assert_eq!(FileType::Other.as_str(), "other");
    }

    // === 文件大小限制测试 ===

    #[test]
    fn test_file_size_within_limits() {
        assert!(limits::check_file_size(1024, "image/jpeg").is_ok());
        assert!(limits::check_file_size(5 * 1024 * 1024, "video/mp4").is_ok());
        assert!(limits::check_file_size(1024, "application/pdf").is_ok());
    }

    #[test]
    fn test_file_size_exceeds_image_limit() {
        let result = limits::check_file_size(11 * 1024 * 1024, "image/jpeg");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceeds maximum"));
    }

    #[test]
    fn test_file_size_exceeds_video_limit() {
        let result = limits::check_file_size(51 * 1024 * 1024, "video/mp4");
        assert!(result.is_err());
    }

    #[test]
    fn test_file_size_exceeds_document_limit() {
        let result = limits::check_file_size(21 * 1024 * 1024, "application/pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_file_size_zero_or_negative() {
        assert!(limits::check_file_size(0, "image/jpeg").is_err());
        assert!(limits::check_file_size(-1, "image/jpeg").is_err());
    }

    // === MIME类型验证测试 ===

    #[test]
    fn test_allowed_mime_types() {
        assert!(is_allowed_mime_type("image/jpeg"));
        assert!(is_allowed_mime_type("image/png"));
        assert!(is_allowed_mime_type("video/mp4"));
        assert!(is_allowed_mime_type("application/pdf"));
        assert!(is_allowed_mime_type("text/plain"));
    }

    #[test]
    fn test_disallowed_mime_types() {
        assert!(!is_allowed_mime_type("application/javascript"));
        assert!(!is_allowed_mime_type("application/x-executable"));
        assert!(!is_allowed_mime_type("text/html"));
        assert!(!is_allowed_mime_type("unknown/type"));
    }

    // === 模型序列化测试 ===

    #[test]
    fn test_upload_request_deserialization() {
        let json = r#"{
            "filename": "test.jpg",
            "file_size": 1024,
            "mime_type": "image/jpeg",
            "is_public": true
        }"#;

        let request: UploadRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.filename, "test.jpg");
        assert_eq!(request.file_size, 1024);
        assert_eq!(request.mime_type, "image/jpeg");
        assert_eq!(request.is_public, Some(true));
    }

    #[test]
    fn test_upload_request_optional_fields() {
        let json = r#"{
            "filename": "test.pdf",
            "file_size": 2048,
            "mime_type": "application/pdf"
        }"#;

        let request: UploadRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.filename, "test.pdf");
        assert!(request.is_public.is_none());
    }

    #[test]
    fn test_file_list_params_deserialization() {
        let json = r#"{
            "file_type": "image",
            "page": 1,
            "page_size": 20
        }"#;

        let params: FileListParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.file_type, Some("image".to_string()));
        assert_eq!(params.page, Some(1));
        assert_eq!(params.page_size, Some(20));
    }

    #[test]
    fn test_file_list_params_defaults() {
        let json = r#"{}"#;
        let params: FileListParams = serde_json::from_str(json).unwrap();
        assert!(params.file_type.is_none());
        assert!(params.page.is_none());
        assert!(params.page_size.is_none());
    }

    #[test]
    fn test_image_process_params_deserialization() {
        let json = r#"{
            "width": 800,
            "height": 600,
            "quality": 85,
            "format": "webp"
        }"#;

        let params: ImageProcessParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.width, Some(800));
        assert_eq!(params.height, Some(600));
        assert_eq!(params.quality, Some(85));
        assert_eq!(params.format, Some("webp".to_string()));
    }

    #[test]
    fn test_batch_upload_request_deserialization() {
        let json = r#"{
            "files": [
                {"filename": "a.jpg", "file_size": 100, "mime_type": "image/jpeg"},
                {"filename": "b.pdf", "file_size": 200, "mime_type": "application/pdf"}
            ]
        }"#;

        let request: BatchUploadRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.files.len(), 2);
        assert_eq!(request.files[0].filename, "a.jpg");
        assert_eq!(request.files[1].filename, "b.pdf");
    }

    #[test]
    fn test_file_list_response_serialization() {
        let response = FileListResponse {
            files: vec![],
            total: 0,
            page: 1,
            page_size: 20,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total\":0"));
        assert!(json.contains("\"page\":1"));
        assert!(json.contains("\"page_size\":20"));
    }
}