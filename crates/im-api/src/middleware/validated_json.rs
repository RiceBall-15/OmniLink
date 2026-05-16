//! 验证 JSON 请求体提取器
//!
//! 提供 `ValidatedJson` 提取器和验证辅助函数，
//! 使用 `common::request_validation::ValidationSchema` 进行验证。

use axum::{
    async_trait,
    extract::{rejection::JsonRejection, FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use common::request_validation::{ValidationErrors, ValidationSchema};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// 验证 JSON 提取器
///
/// 包装 `axum::Json`，在提取请求体后自动使用指定的 `ValidationSchema` 进行验证。
/// 验证失败时返回 422 Unprocessable Entity 响应。
#[derive(Debug, Clone)]
pub struct ValidatedJson<T>(pub T);

/// 验证 JSON 拒绝类型
#[derive(Debug)]
pub enum ValidatedJsonRejection {
    /// JSON 解析失败
    JsonRejection(JsonRejection),
    /// 验证失败
    ValidationRejection(ValidationErrors),
}

impl IntoResponse for ValidatedJsonRejection {
    fn into_response(self) -> Response {
        match self {
            ValidatedJsonRejection::JsonRejection(rejection) => {
                let body = serde_json::json!({
                    "success": false,
                    "message": format!("JSON 解析失败: {}", rejection.body_text()),
                    "code": "INVALID_JSON",
                });
                (StatusCode::BAD_REQUEST, Json(body)).into_response()
            }
            ValidatedJsonRejection::ValidationRejection(errors) => {
                errors.into_axum_response()
            }
        }
    }
}

#[async_trait]
impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ValidatedJsonRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(ValidatedJsonRejection::JsonRejection)?;
        Ok(ValidatedJson(value))
    }
}

/// 验证 Schema 容器，用于 Axum State
///
/// 使用 Builder 模式构建，不依赖 Clone
#[derive(Clone)]
pub struct ValidationState {
    pub schemas: Arc<HashMap<String, ValidationSchema>>,
}

impl ValidationState {
    /// 使用预构建的 schemas 创建
    pub fn new(schemas: HashMap<String, ValidationSchema>) -> Self {
        Self {
            schemas: Arc::new(schemas),
        }
    }

    pub fn get(&self, name: &str) -> Option<&ValidationSchema> {
        self.schemas.get(name)
    }
}

/// ValidationState Builder（不需要 ValidationSchema 实现 Clone）
pub struct ValidationStateBuilder {
    schemas: HashMap<String, ValidationSchema>,
}

impl ValidationStateBuilder {
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    pub fn with_schema(mut self, name: &str, schema: ValidationSchema) -> Self {
        self.schemas.insert(name.to_string(), schema);
        self
    }

    pub fn build(self) -> ValidationState {
        ValidationState::new(self.schemas)
    }
}

impl Default for ValidationStateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// 辅助函数：验证 JSON Value 与给定 Schema
pub fn validate_body(body: &Value, schema: &ValidationSchema) -> Result<(), ValidationErrors> {
    schema.validate(body)
}

/// 辅助函数：创建验证失败响应
pub fn validation_error_response(errors: ValidationErrors) -> Response {
    errors.into_axum_response()
}

/// 辅助函数：验证请求体并返回错误或继续
///
/// 在 handler 中使用：
/// ```rust,no_run
/// use im_api::middleware::validated_json::validate_or_reject;
/// use common::request_validation::ValidationSchemas;
///
/// async fn handler(Json(body): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, Response> {
///     validate_or_reject(&body, &ValidationSchemas::user_register())?;
///     Ok(Json(serde_json::json!({"ok": true})))
/// }
/// ```
pub fn validate_or_reject(body: &Value, schema: &ValidationSchema) -> Result<(), Response> {
    schema.validate(body).map_err(|errors| errors.into_axum_response())
}

/// 辅助函数：验证已反序列化的请求体，返回 ApiError 元组
///
/// 在返回 `(StatusCode, Json<Value>)` 的 handler 中使用：
///
/// ```rust,no_run
/// use im_api::middleware::validated_json::validate_request;
/// use common::request_validation::ValidationSchemas;
///
/// async fn register(Json(req): Json<RegisterRequest>) -> (StatusCode, Json<Value>) {
///     if let Some(err) = validate_request(&req, &ValidationSchemas::user_register()) {
///         return err;
///     }
///     // ... 业务逻辑
/// }
/// ```
pub fn validate_request<T: serde::Serialize>(
    data: &T,
    schema: &ValidationSchema,
) -> Option<(StatusCode, Json<Value>)> {
    match schema.validate_serializable(data) {
        Ok(()) => None,
        Err(errors) => {
            let response = errors.to_response();
            Some((
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({
                    "success": false,
                    "message": response.message,
                    "code": response.code,
                    "errors": response.errors,
                })),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::request_validation::ValidationSchemas;
    use serde_json::json;

    #[test]
    fn test_validate_body_success() {
        let schema = ValidationSchemas::user_register();
        let body = json!({
            "username": "test_user",
            "email": "test@example.com",
            "password": "StrongPass1"
        });
        assert!(validate_body(&body, &schema).is_ok());
    }

    #[test]
    fn test_validate_body_failure() {
        let schema = ValidationSchemas::user_register();
        let body = json!({
            "username": "ab",
            "email": "invalid",
            "password": "weak"
        });
        let result = validate_body(&body, &schema);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.errors.is_empty());
    }

    #[test]
    fn test_validate_or_reject_success() {
        let schema = ValidationSchemas::send_message();
        let body = json!({
            "conversation_id": "550e8400-e29b-41d4-a716-446655440000",
            "content": "Hello!"
        });
        assert!(validate_or_reject(&body, &schema).is_ok());
    }

    #[test]
    fn test_validate_or_reject_failure() {
        let schema = ValidationSchemas::send_message();
        let body = json!({
            "conversation_id": "not-a-uuid",
            "content": ""
        });
        assert!(validate_or_reject(&body, &schema).is_err());
    }

    #[test]
    fn test_validation_state_builder() {
        let state = ValidationStateBuilder::new()
            .with_schema("register", ValidationSchemas::user_register())
            .with_schema("login", ValidationSchemas::user_login())
            .build();

        assert!(state.get("register").is_some());
        assert!(state.get("login").is_some());
        assert!(state.get("nonexistent").is_none());
    }

    #[test]
    fn test_validate_request_success() {
        let schema = ValidationSchemas::user_register();
        let req = json!({
            "username": "test_user",
            "email": "test@example.com",
            "password": "StrongPass1"
        });
        assert!(validate_request(&req, &schema).is_none());
    }

    #[test]
    fn test_validate_request_failure() {
        let schema = ValidationSchemas::user_register();
        let req = json!({
            "username": "ab",
            "email": "invalid",
            "password": "weak"
        });
        let result = validate_request(&req, &schema);
        assert!(result.is_some());
        let (status, body) = result.unwrap();
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        let body_val: Value = serde_json::from_str(&body.0.to_string()).unwrap();
        assert_eq!(body_val["success"], false);
        assert_eq!(body_val["code"], "VALIDATION_ERROR");
    }
}
