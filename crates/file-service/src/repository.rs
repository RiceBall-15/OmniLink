use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use super::models::*;

pub struct FileRepository {
    pool: PgPool,
}

impl FileRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 创建文件记录
    pub async fn create_file(&self, file: FileInfo) -> Result<FileInfo> {
        let new_file = sqlx::query_as::<_, FileInfo>(
            r#"
            INSERT INTO files (id, user_id, filename, original_name, file_path, file_size,
                              mime_type, file_type, width, height, duration, thumbnail_path,
                              storage_type, is_public, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
            "#
        )
        .bind(file.id)
        .bind(file.user_id)
        .bind(&file.filename)
        .bind(&file.original_name)
        .bind(&file.file_path)
        .bind(file.file_size)
        .bind(&file.mime_type)
        .bind(&file.file_type)
        .bind(file.width)
        .bind(file.height)
        .bind(file.duration)
        .bind(&file.thumbnail_path)
        .bind(&file.storage_type)
        .bind(file.is_public)
        .bind(file.created_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(new_file)
    }

    /// 获取文件信息
    pub async fn get_file(&self, file_id: Uuid) -> Result<Option<FileInfo>> {
        let file = sqlx::query_as::<_, FileInfo>(
            r#"
            SELECT * FROM files WHERE id = $1
            "#
        )
        .bind(file_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(file)
    }

    /// 获取用户文件列表
    pub async fn get_user_files(
        &self,
        user_id: Uuid,
        file_type: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<FileInfo>, i64)> {
        let offset = (page - 1) * page_size;

        let query = if let Some(ft) = file_type {
            sqlx::query_as::<_, FileInfo>(
                r#"
                SELECT * FROM files
                WHERE user_id = $1 AND file_type = $2
                ORDER BY created_at DESC
                LIMIT $3 OFFSET $4
                "#
            )
            .bind(user_id)
            .bind(ft)
            .bind(page_size)
            .bind(offset)
        } else {
            sqlx::query_as::<_, FileInfo>(
                r#"
                SELECT * FROM files
                WHERE user_id = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#
            )
            .bind(user_id)
            .bind(page_size)
            .bind(offset)
        };

        let files = query.fetch_all(&self.pool).await?;

        // 查询总数
        let total = if let Some(ft) = file_type {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*) FROM files
                WHERE user_id = $1 AND file_type = $2
                "#
            )
            .bind(user_id)
            .bind(ft)
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*) FROM files WHERE user_id = $1
                "#
            )
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?
        };

        Ok((files, total))
    }

    /// 删除文件
    pub async fn delete_file(&self, file_id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM files WHERE id = $1
            "#
        )
        .bind(file_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 批量删除文件
    pub async fn delete_files(&self, file_ids: &[Uuid]) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM files WHERE id = ANY($1)
            "#
        )
        .bind(file_ids)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// 更新文件信息
    pub async fn update_file(&self, file_id: Uuid, updates: &FileUpdate) -> Result<Option<FileInfo>> {
        let query = sqlx::query_as::<_, FileInfo>(
            r#"
            UPDATE files
            SET
                original_name = COALESCE($2, original_name),
                is_public = COALESCE($3, is_public)
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(file_id)
        .bind(&updates.original_name)
        .bind(updates.is_public)
        .fetch_optional(&self.pool)
        .await?;

        Ok(query)
    }

    /// 获取文件使用统计
    pub async fn get_storage_stats(&self, user_id: Uuid) -> Result<StorageStats> {
        let total_size = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COALESCE(SUM(file_size), 0) FROM files WHERE user_id = $1
            "#
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let count_by_type = sqlx::query_as::<_, TypeCount>(
            r#"
            SELECT
                file_type,
                COUNT(*) as count,
                COALESCE(SUM(file_size), 0) as total_size
            FROM files
            WHERE user_id = $1
            GROUP BY file_type
            "#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(StorageStats {
            total_size,
            file_count: count_by_type.iter().map(|t| t.count).sum(),
            by_type: count_by_type,
        })
    }

    /// 清理临时文件
    pub async fn cleanup_temp_files(&self, days: i64) -> Result<u64> {
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(days);

        let result = sqlx::query(
            r#"
            DELETE FROM files
            WHERE created_at < $1 AND storage_type = 'temp'
            "#
        )
        .bind(cutoff_date)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

/// 文件更新请求
#[derive(Debug, Clone)]
pub struct FileUpdate {
    pub original_name: Option<String>,
    pub is_public: Option<bool>,
}

/// 存储统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_size: i64,
    pub file_count: i64,
    pub by_type: Vec<TypeCount>,
}

/// 按类型统计
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TypeCount {
    pub file_type: String,
    pub count: i64,
    pub total_size: i64,
}