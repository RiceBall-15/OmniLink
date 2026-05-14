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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format_to_string() {
        assert_eq!(ExportFormat::Json.to_string(), "json");
        assert_eq!(ExportFormat::Csv.to_string(), "csv");
        assert_eq!(ExportFormat::Txt.to_string(), "txt");
    }

    #[test]
    fn test_export_format_from_str() {
        assert_eq!(ExportFormat::from_str("json"), ExportFormat::Json);
        assert_eq!(ExportFormat::from_str("csv"), ExportFormat::Csv);
        assert_eq!(ExportFormat::from_str("txt"), ExportFormat::Txt);
        // 默认为 Json
        assert_eq!(ExportFormat::from_str("unknown"), ExportFormat::Json);
    }

    #[test]
    fn test_export_format_file_extension() {
        assert_eq!(ExportFormat::Json.file_extension(), "json");
        assert_eq!(ExportFormat::Csv.file_extension(), "csv");
        assert_eq!(ExportFormat::Txt.file_extension(), "txt");
    }

    #[test]
    fn test_export_format_content_type() {
        assert_eq!(ExportFormat::Json.content_type(), "application/json");
        assert_eq!(ExportFormat::Csv.content_type(), "text/csv");
        assert_eq!(ExportFormat::Txt.content_type(), "text/plain");
    }

    #[test]
    fn test_export_status_to_string() {
        assert_eq!(ExportStatus::Pending.to_string(), "pending");
        assert_eq!(ExportStatus::Processing.to_string(), "processing");
        assert_eq!(ExportStatus::Completed.to_string(), "completed");
        assert_eq!(ExportStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_export_status_from_str() {
        assert_eq!(ExportStatus::from_str("pending"), ExportStatus::Pending);
        assert_eq!(ExportStatus::from_str("processing"), ExportStatus::Processing);
        assert_eq!(ExportStatus::from_str("completed"), ExportStatus::Completed);
        assert_eq!(ExportStatus::from_str("failed"), ExportStatus::Failed);
        // 默认为 Pending
        assert_eq!(ExportStatus::from_str("unknown"), ExportStatus::Pending);
    }

    #[test]
    fn test_export_job_entity_is_completed() {
        let job = ExportJobEntity {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            format: ExportFormat::Json,
            status: ExportStatus::Completed,
            message_count: 10,
            file_path: Some("/tmp/test.json".to_string()),
            file_size: Some(1024),
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: Some(Utc::now()),
        };
        assert!(job.is_completed());
        assert!(!job.is_failed());
    }

    #[test]
    fn test_export_job_entity_is_failed() {
        let job = ExportJobEntity {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            format: ExportFormat::Csv,
            status: ExportStatus::Failed,
            message_count: 0,
            file_path: None,
            file_size: None,
            error_message: Some("导出失败".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: Some(Utc::now()),
        };
        assert!(!job.is_completed());
        assert!(job.is_failed());
    }

    #[test]
    fn test_export_job_entity_pending() {
        let job = ExportJobEntity {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            format: ExportFormat::Txt,
            status: ExportStatus::Pending,
            message_count: 0,
            file_path: None,
            file_size: None,
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
        };
        assert!(!job.is_completed());
        assert!(!job.is_failed());
    }
}
