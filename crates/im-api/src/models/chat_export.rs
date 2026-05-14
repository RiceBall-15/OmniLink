use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 导出任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExportStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl ExportStatus {
    pub fn to_string(&self) -> String {
        match self {
            ExportStatus::Pending => "pending".to_string(),
            ExportStatus::Processing => "processing".to_string(),
            ExportStatus::Completed => "completed".to_string(),
            ExportStatus::Failed => "failed".to_string(),
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => ExportStatus::Pending,
            "processing" => ExportStatus::Processing,
            "completed" => ExportStatus::Completed,
            "failed" => ExportStatus::Failed,
            _ => ExportStatus::Pending,
        }
    }
}

impl sqlx::Type<sqlx::Postgres> for ExportStatus {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <String as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Postgres> for ExportStatus {
    fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        Ok(ExportStatus::from_str(&s))
    }
}

impl sqlx::Encode<'_, sqlx::Postgres> for ExportStatus {
    fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        let s = self.to_string();
        <String as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&s, buf)
    }
}

/// 导出格式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExportFormat {
    Json,
    Csv,
    Txt,
}

impl ExportFormat {
    pub fn to_string(&self) -> String {
        match self {
            ExportFormat::Json => "json".to_string(),
            ExportFormat::Csv => "csv".to_string(),
            ExportFormat::Txt => "txt".to_string(),
        }
    }

    pub fn file_extension(&self) -> &str {
        match self {
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
            ExportFormat::Txt => "txt",
        }
    }

    pub fn content_type(&self) -> &str {
        match self {
            ExportFormat::Json => "application/json",
            ExportFormat::Csv => "text/csv",
            ExportFormat::Txt => "text/plain",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "json" => ExportFormat::Json,
            "csv" => ExportFormat::Csv,
            "txt" => ExportFormat::Txt,
            _ => ExportFormat::Json,
        }
    }
}

impl sqlx::Type<sqlx::Postgres> for ExportFormat {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <String as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Postgres> for ExportFormat {
    fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        Ok(ExportFormat::from_str(&s))
    }
}

impl sqlx::Encode<'_, sqlx::Postgres> for ExportFormat {
    fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        let s = self.to_string();
        <String as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&s, buf)
    }
}

/// 导出任务数据库实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExportJobEntity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub conversation_id: Uuid,
    pub format: ExportFormat,
    pub status: ExportStatus,
    pub message_count: i32,
    pub file_path: Option<String>,
    pub file_size: Option<i64>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl ExportJobEntity {
    pub fn is_completed(&self) -> bool {
        self.status == ExportStatus::Completed
    }

    pub fn is_failed(&self) -> bool {
        self.status == ExportStatus::Failed
    }
}
