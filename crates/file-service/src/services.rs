use anyhow::{Context, Result};
use sqlx::PgPool;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use super::models::*;
use super::repository::{FileRepository, FileUpdate, TypeCount};

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
        tokio::spawn(async move {
            let _ = fs::create_dir_all(&path).await;
        });

        Self {
            repository: FileRepository::new(pool),
            upload_dir: path,
            storage_type,
        }
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

        let data = self._read_file(&file_info.file_path).await?;

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
                "image/jpeg" => ".jpg",
                "image/png" => ".png",
                "image/gif" => ".gif",
                "image/webp" => ".webp",
                "video/mp4" => ".mp4",
                "video/webm" => ".webm",
                "application/pdf" => ".pdf",
                _ => ".bin",
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

    async fn _read_file(&self, path: &str) -> Result<Vec<u8>> {
        let full_path = self.upload_dir.join(path);
        fs::read(&full_path).await.map_err(Into::into)
    }

    async fn _delete_file(&self, path: &str) -> Result<()> {
        let full_path = self.upload_dir.join(path);
        fs::remove_file(&full_path).await.map_err(Into::into)
    }

    async fn _process_media(
        &self,
        path: &str,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileListResponse {
    pub files: Vec<FileInfo>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_size: i64,
    pub file_count: i64,
    pub by_type: Vec<TypeCount>,
}