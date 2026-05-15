//! 文件上传进度追踪器
//!
//! 使用内存存储追踪文件上传进度，支持：
//! - 分块上传进度记录
//! - 进度查询
//! - 自动清理过期记录

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

/// 上传进度状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UploadStatus {
    /// 上传中
    Uploading,
    /// 处理中（生成缩略图等）
    Processing,
    /// 完成
    Completed,
    /// 失败
    Failed(String),
}

/// 单个文件的上传进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadProgress {
    pub upload_id: Uuid,
    pub file_name: String,
    pub total_size: i64,
    pub uploaded_size: i64,
    pub status: UploadStatus,
    pub mime_type: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// 上传完成后的文件 ID
    pub file_id: Option<Uuid>,
    /// 错误信息
    pub error: Option<String>,
}

impl UploadProgress {
    pub fn new(upload_id: Uuid, file_name: String, total_size: i64, mime_type: String) -> Self {
        let now = Utc::now();
        Self {
            upload_id,
            file_name,
            total_size,
            uploaded_size: 0,
            status: UploadStatus::Uploading,
            mime_type,
            created_at: now,
            updated_at: now,
            file_id: None,
            error: None,
        }
    }

    /// 计算进度百分比 (0-100)
    pub fn percentage(&self) -> f64 {
        if self.total_size <= 0 {
            return 0.0;
        }
        ((self.uploaded_size as f64 / self.total_size as f64) * 100.0).min(100.0)
    }

    /// 更新已上传大小
    pub fn update_progress(&mut self, uploaded: i64) {
        self.uploaded_size = uploaded;
        self.updated_at = Utc::now();
    }

    /// 标记为完成
    pub fn mark_completed(&mut self, file_id: Uuid) {
        self.uploaded_size = self.total_size;
        self.status = UploadStatus::Completed;
        self.file_id = Some(file_id);
        self.updated_at = Utc::now();
    }

    /// 标记为处理中
    pub fn mark_processing(&mut self) {
        self.status = UploadStatus::Processing;
        self.updated_at = Utc::now();
    }

    /// 标记为失败
    pub fn mark_failed(&mut self, error: String) {
        self.status = UploadStatus::Failed(error.clone());
        self.error = Some(error);
        self.updated_at = Utc::now();
    }
}

/// 上传进度响应
#[derive(Debug, Serialize)]
pub struct UploadProgressResponse {
    pub upload_id: Uuid,
    pub file_name: String,
    pub total_size: i64,
    pub uploaded_size: i64,
    pub percentage: f64,
    pub status: UploadStatus,
    pub mime_type: String,
    pub file_id: Option<Uuid>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&UploadProgress> for UploadProgressResponse {
    fn from(p: &UploadProgress) -> Self {
        Self {
            upload_id: p.upload_id,
            file_name: p.file_name.clone(),
            total_size: p.total_size,
            uploaded_size: p.uploaded_size,
            percentage: p.percentage(),
            status: p.status.clone(),
            mime_type: p.mime_type.clone(),
            file_id: p.file_id,
            error: p.error.clone(),
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

/// 上传进度追踪器
///
/// 线程安全的内存进度追踪器，使用 DashMap 实现。
/// 支持并发访问，适合多线程异步环境。
pub struct UploadProgressTracker {
    /// upload_id -> UploadProgress
    uploads: Arc<RwLock<HashMap<Uuid, UploadProgress>>>,
    /// 最大保留记录数
    max_entries: usize,
}

impl UploadProgressTracker {
    pub fn new(max_entries: usize) -> Self {
        Self {
            uploads: Arc::new(RwLock::new(HashMap::new())),
            max_entries,
        }
    }

    /// 创建新的上传进度记录
    pub fn create(
        &self,
        file_name: String,
        total_size: i64,
        mime_type: String,
    ) -> Uuid {
        let upload_id = Uuid::new_v4();
        let progress = UploadProgress::new(upload_id, file_name, total_size, mime_type);

        let mut uploads = self.uploads.write();

        // 清理过期记录
        if uploads.len() >= self.max_entries {
            self.cleanup_old_entries(&mut uploads);
        }

        uploads.insert(upload_id, progress);
        upload_id
    }

    /// 更新上传进度
    pub fn update(&self, upload_id: Uuid, uploaded_size: i64) -> bool {
        let mut uploads = self.uploads.write();
        if let Some(progress) = uploads.get_mut(&upload_id) {
            progress.update_progress(uploaded_size);
            true
        } else {
            false
        }
    }

    /// 标记上传完成
    pub fn complete(&self, upload_id: Uuid, file_id: Uuid) -> bool {
        let mut uploads = self.uploads.write();
        if let Some(progress) = uploads.get_mut(&upload_id) {
            progress.mark_completed(file_id);
            true
        } else {
            false
        }
    }

    /// 标记上传处理中
    pub fn processing(&self, upload_id: Uuid) -> bool {
        let mut uploads = self.uploads.write();
        if let Some(progress) = uploads.get_mut(&upload_id) {
            progress.mark_processing();
            true
        } else {
            false
        }
    }

    /// 标记上传失败
    pub fn fail(&self, upload_id: Uuid, error: String) -> bool {
        let mut uploads = self.uploads.write();
        if let Some(progress) = uploads.get_mut(&upload_id) {
            progress.mark_failed(error);
            true
        } else {
            false
        }
    }

    /// 获取上传进度
    pub fn get(&self, upload_id: Uuid) -> Option<UploadProgressResponse> {
        let uploads = self.uploads.read();
        uploads.get(&upload_id).map(UploadProgressResponse::from)
    }

    /// 获取用户的所有上传进度（通过 upload_id 列表）
    pub fn get_batch(&self, upload_ids: &[Uuid]) -> Vec<UploadProgressResponse> {
        let uploads = self.uploads.read();
        upload_ids
            .iter()
            .filter_map(|id| uploads.get(id).map(UploadProgressResponse::from))
            .collect()
    }

    /// 清理过期记录（已完成/失败且超过 1 小时的记录）
    fn cleanup_old_entries(&self, uploads: &mut HashMap<Uuid, UploadProgress>) {
        let cutoff = Utc::now() - chrono::Duration::hours(1);
        let to_remove: Vec<Uuid> = uploads
            .iter()
            .filter(|(_, p)| {
                matches!(p.status, UploadStatus::Completed | UploadStatus::Failed(_))
                    && p.updated_at < cutoff
            })
            .map(|(id, _)| *id)
            .collect();

        for id in &to_remove {
            uploads.remove(id);
        }

        // 如果还是太多，删除最旧的记录
        if uploads.len() >= self.max_entries {
            let mut entries: Vec<(Uuid, DateTime<Utc>)> = uploads
                .iter()
                .map(|(id, p)| (*id, p.updated_at))
                .collect();
            entries.sort_by_key(|(_, t)| *t);

            let remove_count = uploads.len() - self.max_entries / 2;
            for (id, _) in entries.iter().take(remove_count) {
                uploads.remove(id);
            }
        }
    }
}

impl Clone for UploadProgressTracker {
    fn clone(&self) -> Self {
        Self {
            uploads: Arc::clone(&self.uploads),
            max_entries: self.max_entries,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_get_progress() {
        let tracker = UploadProgressTracker::new(100);
        let upload_id = tracker.create("test.jpg".to_string(), 1024, "image/jpeg".to_string());

        let progress = tracker.get(upload_id).unwrap();
        assert_eq!(progress.file_name, "test.jpg");
        assert_eq!(progress.total_size, 1024);
        assert_eq!(progress.uploaded_size, 0);
        assert!((progress.percentage - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_update_progress() {
        let tracker = UploadProgressTracker::new(100);
        let upload_id = tracker.create("test.jpg".to_string(), 1024, "image/jpeg".to_string());

        assert!(tracker.update(upload_id, 512));
        let progress = tracker.get(upload_id).unwrap();
        assert_eq!(progress.uploaded_size, 512);
        assert!((progress.percentage - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_complete_progress() {
        let tracker = UploadProgressTracker::new(100);
        let upload_id = tracker.create("test.jpg".to_string(), 1024, "image/jpeg".to_string());
        let file_id = Uuid::new_v4();

        assert!(tracker.complete(upload_id, file_id));
        let progress = tracker.get(upload_id).unwrap();
        assert_eq!(progress.uploaded_size, 1024);
        assert!((progress.percentage - 100.0).abs() < 0.01);
        assert_eq!(progress.file_id, Some(file_id));
    }

    #[test]
    fn test_fail_progress() {
        let tracker = UploadProgressTracker::new(100);
        let upload_id = tracker.create("test.jpg".to_string(), 1024, "image/jpeg".to_string());

        assert!(tracker.fail(upload_id, "Network error".to_string()));
        let progress = tracker.get(upload_id).unwrap();
        assert!(matches!(progress.status, UploadStatus::Failed(_)));
        assert_eq!(progress.error, Some("Network error".to_string()));
    }

    #[test]
    fn test_get_nonexistent() {
        let tracker = UploadProgressTracker::new(100);
        assert!(tracker.get(Uuid::new_v4()).is_none());
    }

    #[test]
    fn test_batch_get() {
        let tracker = UploadProgressTracker::new(100);
        let id1 = tracker.create("a.jpg".to_string(), 100, "image/jpeg".to_string());
        let id2 = tracker.create("b.pdf".to_string(), 200, "application/pdf".to_string());

        let results = tracker.get_batch(&[id1, id2, Uuid::new_v4()]);
        assert_eq!(results.len(), 2);
    }
}
