use anyhow::{Context, Result};
use sqlx::PgPool;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;


use super::models::*;
use super::repository::{FileRepository, FileUpdate};

/// 允许的文件类型和最大大小
const ALLOWED_TYPES: &[(&str, u64)] = &[
    ("image/jpeg", 20 * 1024 * 1024),     // 20MB
    ("image/png", 20 * 1024 * 1024),
    ("image/gif", 10 * 1024 * 1024),
    ("image/webp", 20 * 1024 * 1024),
    ("video/mp4", 500 * 1024 * 1024),      // 500MB
    ("video/webm", 500 * 1024 * 1024),
    ("audio/mpeg", 50 * 1024 * 1024),      // 50MB
    ("audio/ogg", 50 * 1024 * 1024),
    ("application/pdf", 100 * 1024 * 1024), // 100MB
    ("application/zip", 200 * 1024 * 1024),
    ("text/plain", 10 * 1024 * 1024),
];

/// 默认最大文件大小 (10MB)
const DEFAULT_MAX_SIZE: u64 = 10 * 1024 * 1024;

pub struct FileService {
    repository: FileRepository,
    upload_dir: PathBuf,
    storage_type: String,
}

impl FileService {
    pub fn new(pool: PgPool) -> Self {
        let upload_dir = std::env::var("UPLOAD_DIR")
            .unwrap_or_else(|_| "./uploads".to_string());

        let storage_type = std::env::var("STORAGE_TYPE")
            .unwrap_or_else(|_| "local".to_string());

        // 确保上传目录存在
        let path = PathBuf::from(&upload_dir);
        let path_clone = path.clone();
        tokio::spawn(async move {
            let _ = fs::create_dir_all(&path_clone).await;
        });

        Self {
            repository: FileRepository::new(pool),
            upload_dir: path,
            storage_type,
        }
    }

    /// 验证文件类型和大小
    pub fn validate_file(&self, mime_type: &str, file_size: i64) -> Result<()> {
        // 检查 MIME 类型是否允许
        let max_size = ALLOWED_TYPES
            .iter()
            .find(|(t, _)| *t == mime_type)
            .map(|(_, s)| *s)
            .unwrap_or(DEFAULT_MAX_SIZE);

        if file_size as u64 > max_size {
            return Err(anyhow::anyhow!(
                "File size {} exceeds maximum allowed size {} for type {}",
                file_size,
                max_size,
                mime_type
            ));
        }

        if file_size <= 0 {
            return Err(anyhow::anyhow!("File is empty"));
        }

        Ok(())
    }

    /// 上传文件
    pub async fn upload_file(
        &self,
        user_id: Uuid,
        filename: String,
        file_size: i64,
        mime_type: String,
        data: Vec<u8>,
        is_public: bool,
    ) -> Result<FileInfo> {
        // 验证文件
        self.validate_file(&mime_type, file_size)?;

        // 生成唯一文件名
        let file_id = Uuid::new_v4();
        let ext = self._get_extension(&filename, &mime_type);
        let stored_filename = format!("{}{}", file_id, ext);

        // 确定文件类型
        let file_type = self._get_file_type(&mime_type);

        // 生成文件路径
        let file_path = self._generate_file_path(&file_type, &stored_filename);

        // 保存文件
        self._save_file(&file_path, &data).await?;

        // 处理图片/视频
        let (width, height, duration, thumbnail_path) =
            self._process_media(&file_path, &file_type).await?;

        // 创建文件记录
        let file_info = FileInfo {
            id: file_id,
            user_id,
            filename: stored_filename,
            original_name: filename,
            file_path: file_path.clone(),
            file_size,
            mime_type,
            file_type,
            width,
            height,
            duration,
            thumbnail_path,
            storage_type: self.storage_type.clone(),
            is_public,
            created_at: chrono::Utc::now(),
        };

        self.repository.create_file(file_info).await
    }

    /// 下载文件
    pub async fn download_file(&self, file_id: Uuid) -> Result<(FileInfo, Vec<u8>)> {
        let file_info = self
            .repository
            .get_file(file_id)
            .await?
            .context("File not found")?;

        let data = self.read_file(&file_info.file_path).await?;

        Ok((file_info, data))
    }

    /// 删除文件
    pub async fn delete_file(&self, file_id: Uuid, user_id: Uuid) -> Result<bool> {
        let file_info = self.repository.get_file(file_id).await?;

        if let Some(info) = file_info {
            // 验证权限
            if info.user_id != user_id && !info.is_public {
                return Err(anyhow::anyhow!("Permission denied"));
            }

            // 删除物理文件
            let _ = self._delete_file(&info.file_path).await;

            // 删除缩略图
            if let Some(thumb_path) = info.thumbnail_path {
                let _ = self._delete_file(&thumb_path).await;
            }

            // 删除数据库记录
            self.repository.delete_file(file_id).await
        } else {
            Ok(false)
        }
    }

    /// 获取文件列表
    pub async fn get_files(
        &self,
        user_id: Uuid,
        file_type: Option<String>,
        page: i64,
        page_size: i64,
    ) -> Result<FileListResponse> {
        let (files, total) = self
            .repository
            .get_user_files(user_id, file_type.as_deref(), page, page_size)
            .await?;

        Ok(FileListResponse {
            files,
            total,
            page,
            page_size,
        })
    }

    /// 更新文件信息
    pub async fn update_file(
        &self,
        file_id: Uuid,
        user_id: Uuid,
        updates: FileUpdate,
    ) -> Result<Option<FileInfo>> {
        let file_info = self.repository.get_file(file_id).await?;

        if let Some(info) = file_info {
            // 验证权限
            if info.user_id != user_id {
                return Err(anyhow::anyhow!("Permission denied"));
            }

            self.repository.update_file(file_id, &updates).await
        } else {
            Ok(None)
        }
    }

    /// 获取存储统计
    pub async fn get_storage_stats(&self, user_id: Uuid) -> Result<super::repository::StorageStats> {
        self.repository.get_storage_stats(user_id).await
    }

    /// 生成文件URL
    pub fn generate_file_url(&self, file_id: Uuid) -> String {
        format!("/api/files/{}/download", file_id)
    }

    /// 生成缩略图URL
    pub fn generate_thumbnail_url(&self, file_id: Uuid) -> String {
        format!("/api/files/{}/thumbnail", file_id)
    }

    // 内部方法

    fn _get_extension(&self, filename: &str, mime_type: &str) -> String {
        if let Some(ext) = PathBuf::from(filename).extension() {
            format!(".{}", ext.to_string_lossy())
        } else {
            match mime_type {
                "image/jpeg" => ".jpg".to_string(),
                "image/png" => ".png".to_string(),
                "image/gif" => ".gif".to_string(),
                "image/webp" => ".webp".to_string(),
                "video/mp4" => ".mp4".to_string(),
                "video/webm" => ".webm".to_string(),
                "application/pdf" => ".pdf".to_string(),
                _ => ".bin".to_string(),
            }
        }
    }

    fn _get_file_type(&self, mime_type: &str) -> String {
        if mime_type.starts_with("image/") {
            "image".to_string()
        } else if mime_type.starts_with("video/") {
            "video".to_string()
        } else if mime_type.starts_with("audio/") {
            "audio".to_string()
        } else if mime_type.contains("pdf") {
            "document".to_string()
        } else {
            "other".to_string()
        }
    }

    fn _generate_file_path(&self, file_type: &str, filename: &str) -> String {
        let date = chrono::Utc::now().format("%Y-%m-%d");
        format!("{}/{}/{}", file_type, date, filename)
    }

    async fn _save_file(&self, path: &str, data: &[u8]) -> Result<()> {
        let full_path = self.upload_dir.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut file = fs::File::create(&full_path).await?;
        file.write_all(data).await?;
        Ok(())
    }

    pub async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let full_path = self.upload_dir.join(path);
        fs::read(&full_path).await.map_err(Into::into)
    }

    async fn _delete_file(&self, path: &str) -> Result<()> {
        let full_path = self.upload_dir.join(path);
        fs::remove_file(&full_path).await.map_err(Into::into)
    }

    async fn _process_media(
        &self,
        _path: &str,
        file_type: &str,
    ) -> Result<(Option<i32>, Option<i32>, Option<i32>, Option<String>)> {
        match file_type {
            "image" => {
                // TODO: 实现图片处理（使用image crate）
                Ok((Some(0), Some(0), None, None))
            }
            "video" => {
                // TODO: 实现视频处理（使用FFmpeg）
                Ok((None, None, Some(0), None))
            }
            _ => Ok((None, None, None, None)),
        }
    }

    // === 文件分享相关 ===

    /// 创建文件分享链接
    pub async fn create_share(
        &self,
        file_id: Uuid,
        user_id: Uuid,
        expires_in_hours: Option<i64>,
        max_downloads: Option<i32>,
    ) -> Result<(FileShare, String)> {
        // 验证文件存在且属于用户
        let file_info = self.repository.get_file(file_id).await?
            .ok_or_else(|| anyhow::anyhow!("File not found"))?;

        if file_info.user_id != user_id {
            return Err(anyhow::anyhow!("Not authorized to share this file"));
        }

        // 生成分享 token（短 URL 友好）
        let share_token = self.generate_share_token();

        // 计算过期时间
        let expires_at = expires_in_hours.map(|hours| {
            chrono::Utc::now() + chrono::Duration::hours(hours)
        });

        let share = self.repository.create_share(
            file_id,
            user_id,
            share_token.clone(),
            expires_at,
            max_downloads,
        ).await?;

        let share_url = format!("/api/files/share/{}", share_token);
        Ok((share, share_url))
    }

    /// 通过分享链接下载文件
    pub async fn download_shared_file(
        &self,
        share_token: &str,
    ) -> Result<(FileInfo, Vec<u8>, FileShare)> {
        // 获取分享记录
        let share = self.repository.get_share_by_token(share_token).await?
            .ok_or_else(|| anyhow::anyhow!("Share link not found or inactive"))?;

        // 检查是否过期
        if let Some(expires_at) = share.expires_at {
            if chrono::Utc::now() > expires_at {
                return Err(anyhow::anyhow!("Share link has expired"));
            }
        }

        // 检查下载次数限制
        if let Some(max) = share.max_downloads {
            if share.download_count >= max {
                return Err(anyhow::anyhow!("Download limit reached"));
            }
        }

        // 获取文件
        let file_info = self.repository.get_file(share.file_id).await?
            .ok_or_else(|| anyhow::anyhow!("File not found"))?;

        // 下载文件内容
        let data = self.read_file(&file_info.file_path).await?;

        // 增加下载次数
        self.repository.increment_download_count(share.id).await?;

        Ok((file_info, data, share))
    }

    /// 获取分享信息
    pub async fn get_share_info(&self, share_token: &str) -> Result<ShareInfoResponse> {
        let share = self.repository.get_share_by_token(share_token).await?
            .ok_or_else(|| anyhow::anyhow!("Share link not found"))?;

        let file_info = self.repository.get_file(share.file_id).await?
            .ok_or_else(|| anyhow::anyhow!("File not found"))?;

        let is_expired = share.expires_at
            .map(|exp| chrono::Utc::now() > exp)
            .unwrap_or(false);

        let is_download_limit_reached = share.max_downloads
            .map(|max| share.download_count >= max)
            .unwrap_or(false);

        Ok(ShareInfoResponse {
            share_id: share.id,
            file_id: share.file_id,
            file_name: file_info.original_name,
            file_size: file_info.file_size,
            mime_type: file_info.mime_type,
            created_by: share.created_by,
            expires_at: share.expires_at.map(|t| t.to_rfc3339()),
            max_downloads: share.max_downloads,
            download_count: share.download_count,
            is_expired,
            is_download_limit_reached,
            share_url: format!("/api/files/share/{}", share.share_token),
        })
    }

    /// 删除分享链接
    pub async fn delete_share(&self, share_id: Uuid, user_id: Uuid) -> Result<bool> {
        self.repository.deactivate_share(share_id, user_id).await
    }

    /// 获取文件的所有分享
    pub async fn get_file_shares(&self, file_id: Uuid, user_id: Uuid) -> Result<Vec<FileShare>> {
        // 验证文件属于用户
        let file_info = self.repository.get_file(file_id).await?
            .ok_or_else(|| anyhow::anyhow!("File not found"))?;

        if file_info.user_id != user_id {
            return Err(anyhow::anyhow!("Not authorized"));
        }

        self.repository.get_file_shares(file_id).await
    }

    /// 生成分享 token
    fn generate_share_token(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        // 使用 UUID 的前 12 字符作为短 token
        let uuid = Uuid::new_v4().to_string();
        let token: String = uuid.chars().take(12).collect();
        token
    }
}



