//! 存储后端抽象层
//!
//! 支持多种存储后端：
//! - 本地文件系统存储
//! - MinIO (S3兼容) 对象存储
//!
//! 通过环境变量 STORAGE_TYPE 选择存储后端：
//! - "local" (默认)：本地文件系统
//! - "minio"：MinIO 对象存储

use anyhow::Result;
use async_trait::async_trait;

/// 存储后端特征
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// 上传文件，返回存储路径
    async fn upload(&self, path: &str, data: &[u8], content_type: &str) -> Result<String>;
    /// 下载文件
    async fn download(&self, path: &str) -> Result<Vec<u8>>;
    /// 删除文件
    async fn delete(&self, path: &str) -> Result<()>;
    /// 检查文件是否存在
    async fn exists(&self, path: &str) -> Result<bool>;
    /// 获取文件的访问URL
    async fn get_url(&self, path: &str) -> Result<String>;
    /// 获取存储后端类型名称
    fn backend_type(&self) -> &str;
}

// ============================================================
// 本地文件系统存储后端
// ============================================================

pub struct LocalStorage {
    base_dir: std::path::PathBuf,
}

impl LocalStorage {
    pub fn new(base_dir: &str) -> Self {
        Self {
            base_dir: std::path::PathBuf::from(base_dir),
        }
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    async fn upload(&self, path: &str, data: &[u8], _content_type: &str) -> Result<String> {
        let full_path = self.base_dir.join(path);
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&full_path, data).await?;
        Ok(path.to_string())
    }

    async fn download(&self, path: &str) -> Result<Vec<u8>> {
        let full_path = self.base_dir.join(path);
        tokio::fs::read(&full_path).await.map_err(Into::into)
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let full_path = self.base_dir.join(path);
        if full_path.exists() {
            tokio::fs::remove_file(&full_path).await?;
        }
        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let full_path = self.base_dir.join(path);
        Ok(full_path.exists())
    }

    async fn get_url(&self, path: &str) -> Result<String> {
        // 本地存储返回API路径
        Ok(format!("/api/files/storage/{}", path))
    }

    fn backend_type(&self) -> &str {
        "local"
    }
}

// ============================================================
// MinIO/S3 兼容存储后端
// ============================================================

/// MinIO 存储后端配置
#[derive(Debug, Clone)]
pub struct MinioConfig {
    pub endpoint: String,      // 例如: localhost:9000
    pub bucket: String,        // 存储桶名称
    pub access_key: String,
    pub secret_key: String,
    pub region: String,
    pub use_ssl: bool,
}

impl MinioConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> Self {
        Self {
            endpoint: std::env::var("MINIO_ENDPOINT")
                .unwrap_or_else(|_| "localhost:9000".to_string()),
            bucket: std::env::var("MINIO_BUCKET")
                .unwrap_or_else(|_| "omnilink".to_string()),
            access_key: std::env::var("MINIO_ACCESS_KEY")
                .unwrap_or_default(),
            secret_key: std::env::var("MINIO_SECRET_KEY")
                .unwrap_or_default(),
            region: std::env::var("MINIO_REGION")
                .unwrap_or_else(|_| "us-east-1".to_string()),
            use_ssl: std::env::var("MINIO_USE_SSL")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
        }
    }
}

pub struct MinioStorage {
    config: MinioConfig,
    client: reqwest::Client,
}

impl MinioStorage {
    pub fn new(config: MinioConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        Ok(Self { config, client })
    }

    pub fn from_env() -> Result<Self> {
        let config = MinioConfig::from_env();
        Self::new(config)
    }

    /// 确保存储桶存在，不存在则自动创建
    pub async fn ensure_bucket(&self) -> Result<()> {
        let protocol = if self.config.use_ssl { "https" } else { "http" };
        let bucket_url = format!(
            "{}://{}/{}",
            protocol, self.config.endpoint, self.config.bucket
        );

        // Check if bucket exists
        let response = self.client
            .head(&bucket_url)
            .basic_auth(&self.config.access_key, Some(&self.config.secret_key))
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            // Create bucket
            let create_response = self.client
                .put(&bucket_url)
                .basic_auth(&self.config.access_key, Some(&self.config.secret_key))
                .send()
                .await?;

            if !create_response.status().is_success() {
                let status = create_response.status();
                let body = create_response.text().await.unwrap_or_default();
                return Err(anyhow::anyhow!(
                    "Failed to create MinIO bucket '{}' ({}): {}",
                    self.config.bucket, status, body
                ));
            }
            tracing::info!("Created MinIO bucket: {}", self.config.bucket);
        } else {
            tracing::info!("MinIO bucket '{}' already exists", self.config.bucket);
        }

        Ok(())
    }

    /// 构建对象URL
    fn object_url(&self, path: &str) -> String {
        let protocol = if self.config.use_ssl { "https" } else { "http" };
        format!(
            "{}://{}/{}/{}",
            protocol, self.config.endpoint, self.config.bucket, path
        )
    }

    /// 构建存储桶URL
    #[allow(dead_code)]
    fn bucket_url(&self) -> String {
        let protocol = if self.config.use_ssl { "https" } else { "http" };
        format!(
            "{}://{}/{}",
            protocol, self.config.endpoint, self.config.bucket
        )
    }
}

#[async_trait]
impl StorageBackend for MinioStorage {
    async fn upload(&self, path: &str, data: &[u8], content_type: &str) -> Result<String> {
        let url = self.object_url(path);

        let response = self.client
            .put(&url)
            .header("Content-Type", content_type)
            .basic_auth(&self.config.access_key, Some(&self.config.secret_key))
            .body(data.to_vec())
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "MinIO upload failed ({}): {}", status, body
            ));
        }

        Ok(path.to_string())
    }

    async fn download(&self, path: &str) -> Result<Vec<u8>> {
        let url = self.object_url(path);

        let response = self.client
            .get(&url)
            .basic_auth(&self.config.access_key, Some(&self.config.secret_key))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(anyhow::anyhow!("MinIO download failed: {}", status));
        }

        response.bytes().await
            .map(|b| b.to_vec())
            .map_err(|e| anyhow::anyhow!("Failed to read response: {}", e))
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let url = self.object_url(path);

        let response = self.client
            .delete(&url)
            .basic_auth(&self.config.access_key, Some(&self.config.secret_key))
            .send()
            .await?;

        // 404 is acceptable for delete
        if !response.status().is_success() && response.status() != reqwest::StatusCode::NOT_FOUND {
            let status = response.status();
            return Err(anyhow::anyhow!("MinIO delete failed: {}", status));
        }

        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let url = self.object_url(path);

        let response = self.client
            .head(&url)
            .basic_auth(&self.config.access_key, Some(&self.config.secret_key))
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    async fn get_url(&self, path: &str) -> Result<String> {
        // MinIO 返回对象的直接URL
        Ok(self.object_url(path))
    }

    fn backend_type(&self) -> &str {
        "minio"
    }
}

// ============================================================
// 存储后端工厂
// ============================================================

/// 根据环境变量创建存储后端实例
pub fn create_storage_backend(upload_dir: &str) -> Result<Box<dyn StorageBackend>> {
    let storage_type = std::env::var("STORAGE_TYPE")
        .unwrap_or_else(|_| "local".to_string());

    match storage_type.as_str() {
        "minio" | "s3" => {
            tracing::info!("使用 MinIO 对象存储后端");
            let backend = MinioStorage::from_env()?;
            Ok(Box::new(backend))
        }
        _ => {
            tracing::info!("使用本地文件存储后端: {}", upload_dir);
            let backend = LocalStorage::new(upload_dir);
            Ok(Box::new(backend))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_storage_backend_type() {
        let storage = LocalStorage::new("/tmp/test");
        assert_eq!(storage.backend_type(), "local");
    }

    #[tokio::test]
    async fn test_local_storage_upload_download() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = LocalStorage::new(temp_dir.path().to_str().unwrap());

        let data = b"hello world";
        let path = "test/file.txt";

        // Upload
        let result = storage.upload(path, data, "text/plain").await.unwrap();
        assert_eq!(result, path);

        // Check exists
        assert!(storage.exists(path).await.unwrap());

        // Download
        let downloaded = storage.download(path).await.unwrap();
        assert_eq!(downloaded, data);

        // Get URL
        let url = storage.get_url(path).await.unwrap();
        assert!(url.contains("test/file.txt"));

        // Delete
        storage.delete(path).await.unwrap();
        assert!(!storage.exists(path).await.unwrap());
    }

    #[test]
    fn test_minio_config_from_env_defaults() {
        // 清理环境变量
        std::env::remove_var("MINIO_ENDPOINT");
        std::env::remove_var("MINIO_BUCKET");
        std::env::remove_var("MINIO_ACCESS_KEY");
        std::env::remove_var("MINIO_SECRET_KEY");
        std::env::remove_var("MINIO_REGION");
        std::env::remove_var("MINIO_USE_SSL");

        let config = MinioConfig::from_env();
        assert_eq!(config.endpoint, "localhost:9000");
        assert_eq!(config.bucket, "omnilink");
        assert_eq!(config.region, "us-east-1");
        assert!(!config.use_ssl);
    }

    #[test]
    fn test_create_storage_backend_default() {
        std::env::remove_var("STORAGE_TYPE");
        let backend = create_storage_backend("/tmp/test").unwrap();
        assert_eq!(backend.backend_type(), "local");
    }
}
