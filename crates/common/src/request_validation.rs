//! API 请求体验证框架
//!
//! 提供统一的请求体验证中间件，支持：
//! - 基于规则的字段验证（长度、格式、范围、自定义）
//! - 自定义验证器注册
//! - 统一验证错误响应格式
//! - 与 Axum 中间件集成
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use common::request_validation::*;
//! use serde_json::json;
//!
//! // 定义验证 schema
//! let schema = ValidationSchema::new()
//!     .field(FieldValidation::new("username")
//!         .required()
//!         .min_length(3)
//!         .max_length(50)
//!         .pattern(r"^[a-zA-Z0-9_-]+$", "用户名只允许字母、数字、下划线和连字符"))
//!     .field(FieldValidation::new("email")
//!         .required()
//!         .email())
//!     .field(FieldValidation::new("age")
//!         .optional()
//!         .range(0.0, 200.0));
//!
//! // 验证请求体
//! let body = json!({"username": "test_user", "email": "test@example.com"});
//! let result = schema.validate(&body);
//! assert!(result.is_ok());
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

// ============================================================================
// 验证规则定义
// ============================================================================

/// 验证规则 trait
///
/// 实现此 trait 可创建自定义验证器
pub trait ValidationRule: Send + Sync {
    /// 验证字段值
    fn validate(&self, value: &Value, field_name: &str) -> Result<(), ValidationErrorDetail>;

    /// 规则描述（用于错误消息）
    fn description(&self) -> String;

    /// 克隆为 Box（支持 Clone）
    fn clone_box(&self) -> Box<dyn ValidationRule>;
}

/// 长度验证规则
#[derive(Debug, Clone)]
pub struct LengthRule {
    pub min: Option<usize>,
    pub max: Option<usize>,
}

impl ValidationRule for LengthRule {
    fn validate(&self, value: &Value, field_name: &str) -> Result<(), ValidationErrorDetail> {
        let len = match value {
            Value::String(s) => s.len(),
            Value::Array(a) => a.len(),
            _ => return Ok(()), // 非字符串/数组类型跳过长度检查
        };

        if let Some(min) = self.min {
            if len < min {
                return Err(ValidationErrorDetail {
                    field: field_name.to_string(),
                    message: format!("长度不能少于 {} 个字符，当前 {} 个", min, len),
                    code: "too_short".to_string(),
                    params: serde_json::json!({"min": min, "actual": len}),
                });
            }
        }

        if let Some(max) = self.max {
            if len > max {
                return Err(ValidationErrorDetail {
                    field: field_name.to_string(),
                    message: format!("长度不能超过 {} 个字符，当前 {} 个", max, len),
                    code: "too_long".to_string(),
                    params: serde_json::json!({"max": max, "actual": len}),
                });
            }
        }

        Ok(())
    }

    fn description(&self) -> String {
        match (self.min, self.max) {
            (Some(min), Some(max)) => format!("长度 {}-{}", min, max),
            (Some(min), None) => format!("最小长度 {}", min),
            (None, Some(max)) => format!("最大长度 {}", max),
            (None, None) => "无长度限制".to_string(),
        }
    }

    fn clone_box(&self) -> Box<dyn ValidationRule> {
        Box::new(self.clone())
    }
}

/// 范围验证规则（数值）
#[derive(Debug, Clone)]
pub struct RangeRule {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

impl ValidationRule for RangeRule {
    fn validate(&self, value: &Value, field_name: &str) -> Result<(), ValidationErrorDetail> {
        let num = match value {
            Value::Number(n) => n.as_f64().unwrap_or(0.0),
            _ => return Ok(()), // 非数值类型跳过范围检查
        };

        if let Some(min) = self.min {
            if num < min {
                return Err(ValidationErrorDetail {
                    field: field_name.to_string(),
                    message: format!("值不能小于 {}，当前 {}", min, num),
                    code: "too_small".to_string(),
                    params: serde_json::json!({"min": min, "actual": num}),
                });
            }
        }

        if let Some(max) = self.max {
            if num > max {
                return Err(ValidationErrorDetail {
                    field: field_name.to_string(),
                    message: format!("值不能大于 {}，当前 {}", max, num),
                    code: "too_large".to_string(),
                    params: serde_json::json!({"max": max, "actual": num}),
                });
            }
        }

        Ok(())
    }

    fn description(&self) -> String {
        match (self.min, self.max) {
            (Some(min), Some(max)) => format!("范围 {}-{}", min, max),
            (Some(min), None) => format!("最小值 {}", min),
            (None, Some(max)) => format!("最大值 {}", max),
            (None, None) => "无范围限制".to_string(),
        }
    }

    fn clone_box(&self) -> Box<dyn ValidationRule> {
        Box::new(self.clone())
    }
}

/// 正则表达式验证规则
#[derive(Debug, Clone)]
pub struct PatternRule {
    pub pattern: String,
    pub error_message: String,
}

impl ValidationRule for PatternRule {
    fn validate(&self, value: &Value, field_name: &str) -> Result<(), ValidationErrorDetail> {
        let s = match value {
            Value::String(s) => s.clone(),
            _ => return Ok(()),
        };

        let re = match regex::Regex::new(&self.pattern) {
            Ok(re) => re,
            Err(_) => return Ok(()), // 无效正则则跳过
        };

        if !re.is_match(&s) {
            return Err(ValidationErrorDetail {
                field: field_name.to_string(),
                message: self.error_message.clone(),
                code: "invalid_format".to_string(),
                params: serde_json::json!({"pattern": self.pattern}),
            });
        }

        Ok(())
    }

    fn description(&self) -> String {
        format!("匹配模式: {}", self.pattern)
    }

    fn clone_box(&self) -> Box<dyn ValidationRule> {
        Box::new(self.clone())
    }
}

/// 枚举值验证规则
#[derive(Debug, Clone)]
pub struct EnumRule {
    pub allowed_values: Vec<String>,
}

impl ValidationRule for EnumRule {
    fn validate(&self, value: &Value, field_name: &str) -> Result<(), ValidationErrorDetail> {
        let s = match value {
            Value::String(s) => s.clone(),
            _ => return Ok(()),
        };

        if !self.allowed_values.contains(&s) {
            return Err(ValidationErrorDetail {
                field: field_name.to_string(),
                message: format!(
                    "值 '{}' 不在允许范围内: [{}]",
                    s,
                    self.allowed_values.join(", ")
                ),
                code: "invalid_enum".to_string(),
                params: serde_json::json!({"allowed": self.allowed_values}),
            });
        }

        Ok(())
    }

    fn description(&self) -> String {
        format!("允许值: [{}]", self.allowed_values.join(", "))
    }

    fn clone_box(&self) -> Box<dyn ValidationRule> {
        Box::new(self.clone())
    }
}

/// 自定义闭包验证规则
#[derive(Clone)]
pub struct CustomRule {
    pub name: String,
    pub validator: Arc<dyn Fn(&Value) -> Result<(), String> + Send + Sync>,
}

impl std::fmt::Debug for CustomRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomRule")
            .field("name", &self.name)
            .finish()
    }
}

impl ValidationRule for CustomRule {
    fn validate(&self, value: &Value, field_name: &str) -> Result<(), ValidationErrorDetail> {
        match (self.validator)(value) {
            Ok(()) => Ok(()),
            Err(msg) => Err(ValidationErrorDetail {
                field: field_name.to_string(),
                message: msg,
                code: "custom_validation".to_string(),
                params: serde_json::json!({}),
            }),
        }
    }

    fn description(&self) -> String {
        format!("自定义规则: {}", self.name)
    }

    fn clone_box(&self) -> Box<dyn ValidationRule> {
        Box::new(self.clone())
    }
}

// ============================================================================
// 字段验证配置
// ============================================================================

/// 字段验证配置
///
/// 定义单个字段的验证规则
pub struct FieldValidation {
    /// 字段名称
    pub field_name: String,
    /// 是否必填
    pub required: bool,
    /// 验证规则列表
    pub rules: Vec<Box<dyn ValidationRule>>,
    /// 字段描述（用于错误消息）
    pub description: Option<String>,
}

impl Clone for FieldValidation {
    fn clone(&self) -> Self {
        Self {
            field_name: self.field_name.clone(),
            required: self.required,
            rules: self.rules.iter().map(|r| r.clone_box()).collect(),
            description: self.description.clone(),
        }
    }
}

impl FieldValidation {
    /// 创建新的字段验证配置
    pub fn new(field_name: &str) -> Self {
        Self {
            field_name: field_name.to_string(),
            required: false,
            rules: Vec::new(),
            description: None,
        }
    }

    /// 标记为必填
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// 标记为可选
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    /// 设置字段描述
    pub fn description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    /// 添加最小长度规则
    pub fn min_length(mut self, min: usize) -> Self {
        self.rules.push(Box::new(LengthRule {
            min: Some(min),
            max: None,
        }));
        self
    }

    /// 添加最大长度规则
    pub fn max_length(mut self, max: usize) -> Self {
        self.rules.push(Box::new(LengthRule {
            min: None,
            max: Some(max),
        }));
        self
    }

    /// 添加长度范围规则
    pub fn length_range(mut self, min: usize, max: usize) -> Self {
        self.rules.push(Box::new(LengthRule {
            min: Some(min),
            max: Some(max),
        }));
        self
    }

    /// 添加数值范围规则
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.rules.push(Box::new(RangeRule {
            min: Some(min),
            max: Some(max),
        }));
        self
    }

    /// 添加正则表达式规则
    pub fn pattern(mut self, pattern: &str, error_message: &str) -> Self {
        self.rules.push(Box::new(PatternRule {
            pattern: pattern.to_string(),
            error_message: error_message.to_string(),
        }));
        self
    }

    /// 添加邮箱格式验证
    pub fn email(self) -> Self {
        self.pattern(
            r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$",
            "邮箱格式不正确",
        )
    }

    /// 添加 UUID 格式验证
    pub fn uuid(self) -> Self {
        self.pattern(
            r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$",
            "UUID 格式不正确",
        )
    }

    /// 添加 URL 格式验证
    pub fn url(self) -> Self {
        self.pattern(
            r"^https?://[^\s/$.?#].[^\s]*$",
            "URL 格式不正确",
        )
    }

    /// 添加枚举值验证
    pub fn enum_values(mut self, values: Vec<&str>) -> Self {
        self.rules.push(Box::new(EnumRule {
            allowed_values: values.into_iter().map(|s| s.to_string()).collect(),
        }));
        self
    }

    /// 添加自定义验证规则
    pub fn custom<F>(mut self, name: &str, validator: F) -> Self
    where
        F: Fn(&Value) -> Result<(), String> + Send + Sync + 'static,
    {
        self.rules.push(Box::new(CustomRule {
            name: name.to_string(),
            validator: Arc::new(validator),
        }));
        self
    }
}

// ============================================================================
// 验证 Schema
// ============================================================================

/// 请求体验证 Schema
///
/// 定义整个请求体的验证规则
pub struct ValidationSchema {
    /// 字段验证配置映射
    fields: HashMap<String, FieldValidation>,
    /// 自定义全局验证器
    global_validators: Vec<Box<dyn ValidationRule>>,
}

impl Clone for ValidationSchema {
    fn clone(&self) -> Self {
        Self {
            fields: self.fields.clone(),
            global_validators: self.global_validators.iter().map(|r| r.clone_box()).collect(),
        }
    }
}

impl ValidationSchema {
    /// 创建新的空 Schema
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            global_validators: Vec::new(),
        }
    }

    /// 添加字段验证配置
    pub fn field(mut self, field: FieldValidation) -> Self {
        self.fields.insert(field.field_name.clone(), field);
        self
    }

    /// 添加全局验证规则
    pub fn global_rule(mut self, rule: Box<dyn ValidationRule>) -> Self {
        self.global_validators.push(rule);
        self
    }

    /// 验证 JSON 值
    pub fn validate(&self, value: &Value) -> Result<(), ValidationErrors> {
        let mut errors = Vec::new();

        // 验证每个字段
        for (field_name, field_config) in &self.fields {
            let field_value = value.get(field_name);

            // 检查必填字段
            if field_config.required {
                match field_value {
                    None | Some(Value::Null) => {
                        errors.push(ValidationErrorDetail {
                            field: field_name.clone(),
                            message: format!(
                                "{}是必填字段",
                                field_config.description.as_deref().unwrap_or(field_name)
                            ),
                            code: "required".to_string(),
                            params: serde_json::json!({}),
                        });
                        continue;
                    }
                    Some(Value::String(s)) if s.trim().is_empty() => {
                        errors.push(ValidationErrorDetail {
                            field: field_name.clone(),
                            message: format!(
                                "{}不能为空",
                                field_config.description.as_deref().unwrap_or(field_name)
                            ),
                            code: "required".to_string(),
                            params: serde_json::json!({}),
                        });
                        continue;
                    }
                    _ => {}
                }
            }

            // 如果字段存在且非空，执行验证规则
            if let Some(val) = field_value {
                if !val.is_null() {
                    for rule in &field_config.rules {
                        if let Err(e) = rule.validate(val, field_name) {
                            errors.push(e);
                        }
                    }
                }
            }
        }

        // 执行全局验证器
        for rule in &self.global_validators {
            if let Err(e) = rule.validate(value, "_global") {
                errors.push(e);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors { errors })
        }
    }

    /// 验证 serde_json::Value（便利方法）
    pub fn validate_json(&self, json_str: &str) -> Result<(), ValidationErrors> {
        let value: Value = serde_json::from_str(json_str).map_err(|e| ValidationErrors {
            errors: vec![ValidationErrorDetail {
                field: "_body".to_string(),
                message: format!("JSON 解析失败: {}", e),
                code: "invalid_json".to_string(),
                params: serde_json::json!({}),
            }],
        })?;
        self.validate(&value)
    }
}

impl Default for ValidationSchema {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 验证错误类型
// ============================================================================

/// 单个字段验证错误详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationErrorDetail {
    /// 错误字段名
    pub field: String,
    /// 错误消息
    pub message: String,
    /// 错误码
    pub code: String,
    /// 错误参数
    pub params: Value,
}

/// 验证错误集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationErrors {
    /// 所有验证错误
    pub errors: Vec<ValidationErrorDetail>,
}

impl std::fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let messages: Vec<String> = self.errors.iter().map(|e| e.message.clone()).collect();
        write!(f, "验证失败: {}", messages.join("; "))
    }
}

impl std::error::Error for ValidationErrors {}

/// 统一验证错误响应格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationErrorResponse {
    /// 错误状态
    pub success: bool,
    /// 错误消息
    pub message: String,
    /// 错误码
    pub code: String,
    /// 字段级错误详情
    pub errors: Vec<ValidationErrorDetail>,
}

impl ValidationErrors {
    /// 转换为 HTTP 响应格式
    pub fn to_response(&self) -> ValidationErrorResponse {
        let field_errors: Vec<String> = self
            .errors
            .iter()
            .map(|e| format!("{}: {}", e.field, e.message))
            .collect();

        ValidationErrorResponse {
            success: false,
            message: format!("请求验证失败: {}", field_errors.join("; ")),
            code: "VALIDATION_ERROR".to_string(),
            errors: self.errors.clone(),
        }
    }

    /// 转换为 Axum JSON 响应
    pub fn into_axum_response(self) -> axum::response::Response {
        let response = self.to_response();
        let body = serde_json::to_string(&response).unwrap_or_default();

        axum::response::Response::builder()
            .status(http::StatusCode::UNPROCESSABLE_ENTITY)
            .header("content-type", "application/json")
            .body(axum::body::Body::from(body))
            .unwrap()
    }
}

// ============================================================================
// 预构建验证 Schema 工厂
// ============================================================================

/// 预构建验证 Schema 工厂
///
/// 提供常用 API 端点的预定义验证规则
pub struct ValidationSchemas;

impl ValidationSchemas {
    /// 用户注册请求验证
    pub fn user_register() -> ValidationSchema {
        ValidationSchema::new()
            .field(
                FieldValidation::new("username")
                    .required()
                    .description("用户名")
                    .length_range(3, 50)
                    .pattern(r"^[a-zA-Z0-9_-]+$", "用户名只允许字母、数字、下划线和连字符"),
            )
            .field(
                FieldValidation::new("email")
                    .required()
                    .description("邮箱")
                    .email()
                    .max_length(255),
            )
            .field(
                FieldValidation::new("password")
                    .required()
                    .description("密码")
                    .min_length(8)
                    .max_length(128)
                    .custom("password_strength", |value| {
                        if let Some(s) = value.as_str() {
                            let has_upper = s.chars().any(|c| c.is_uppercase());
                            let has_lower = s.chars().any(|c| c.is_lowercase());
                            let has_digit = s.chars().any(|c| c.is_numeric());
                            if !has_upper || !has_lower || !has_digit {
                                return Err("密码必须包含大小写字母和数字".to_string());
                            }
                        }
                        Ok(())
                    }),
            )
    }

    /// 用户登录请求验证
    pub fn user_login() -> ValidationSchema {
        ValidationSchema::new()
            .field(
                FieldValidation::new("email")
                    .required()
                    .description("邮箱")
                    .email(),
            )
            .field(
                FieldValidation::new("password")
                    .required()
                    .description("密码")
                    .min_length(1),
            )
    }

    /// 发送消息请求验证
    pub fn send_message() -> ValidationSchema {
        ValidationSchema::new()
            .field(
                FieldValidation::new("conversation_id")
                    .required()
                    .description("会话ID")
                    .uuid(),
            )
            .field(
                FieldValidation::new("content")
                    .required()
                    .description("消息内容")
                    .length_range(1, 10000),
            )
            .field(
                FieldValidation::new("message_type")
                    .optional()
                    .description("消息类型")
                    .enum_values(vec!["text", "image", "file", "voice", "video", "system"]),
            )
    }

    /// 创建会话请求验证
    pub fn create_conversation() -> ValidationSchema {
        ValidationSchema::new()
            .field(
                FieldValidation::new("name")
                    .optional()
                    .description("会话名称")
                    .length_range(1, 100),
            )
            .field(
                FieldValidation::new("conversation_type")
                    .required()
                    .description("会话类型")
                    .enum_values(vec!["direct", "group"]),
            )
            .field(
                FieldValidation::new("member_ids")
                    .optional()
                    .description("成员ID列表")
                    .custom("non_empty_array", |value| {
                        if let Some(arr) = value.as_array() {
                            if arr.is_empty() {
                                return Err("成员列表不能为空".to_string());
                            }
                            if arr.len() > 500 {
                                return Err("成员数量不能超过500".to_string());
                            }
                        }
                        Ok(())
                    }),
            )
    }

    /// 更新用户资料请求验证
    pub fn update_profile() -> ValidationSchema {
        ValidationSchema::new()
            .field(
                FieldValidation::new("nickname")
                    .optional()
                    .description("昵称")
                    .length_range(1, 100),
            )
            .field(
                FieldValidation::new("bio")
                    .optional()
                    .description("个人简介")
                    .max_length(500),
            )
            .field(
                FieldValidation::new("avatar_url")
                    .optional()
                    .description("头像URL")
                    .url()
                    .max_length(2048),
            )
    }

    /// 管理员操作验证
    pub fn admin_action() -> ValidationSchema {
        ValidationSchema::new()
            .field(
                FieldValidation::new("action")
                    .required()
                    .description("操作类型")
                    .enum_values(vec!["ban", "unban", "delete", "warn", "mute"]),
            )
            .field(
                FieldValidation::new("reason")
                    .optional()
                    .description("原因")
                    .max_length(1000),
            )
            .field(
                FieldValidation::new("duration_hours")
                    .optional()
                    .description("时长（小时）")
                    .range(1.0, 8760.0),
            )
    }

    /// Webhook 创建验证
    pub fn create_webhook() -> ValidationSchema {
        ValidationSchema::new()
            .field(
                FieldValidation::new("url")
                    .required()
                    .description("Webhook URL")
                    .url()
                    .max_length(2048),
            )
            .field(
                FieldValidation::new("events")
                    .required()
                    .description("事件类型列表")
                    .custom("non_empty_array", |value| {
                        if let Some(arr) = value.as_array() {
                            if arr.is_empty() {
                                return Err("事件列表不能为空".to_string());
                            }
                        }
                        Ok(())
                    }),
            )
            .field(
                FieldValidation::new("secret")
                    .optional()
                    .description("密钥")
                    .length_range(16, 256),
            )
    }
}

// ============================================================================
// Axum 中间件集成
// ============================================================================

/// 请求体验证中间件
///
/// 用于在 Axum 路由中验证请求体
pub struct RequestBodyValidator {
    schema: ValidationSchema,
}

impl RequestBodyValidator {
    /// 创建新的请求体验证器
    pub fn new(schema: ValidationSchema) -> Self {
        Self { schema }
    }

    /// 验证请求体
    pub fn validate(&self, body: &Value) -> Result<(), ValidationErrors> {
        self.schema.validate(body)
    }
}

// ============================================================================
// 验证上下文扩展
// ============================================================================

/// 验证上下文，用于在请求处理链中传递验证结果
#[derive(Debug, Clone)]
pub struct ValidationContext {
    /// 是否通过验证
    pub is_valid: bool,
    /// 验证错误（如果有）
    pub errors: Option<ValidationErrors>,
    /// 已验证的数据
    pub validated_data: Option<Value>,
}

impl ValidationContext {
    /// 创建成功的验证上下文
    pub fn success(data: Value) -> Self {
        Self {
            is_valid: true,
            errors: None,
            validated_data: Some(data),
        }
    }

    /// 创建失败的验证上下文
    pub fn failure(errors: ValidationErrors) -> Self {
        Self {
            is_valid: false,
            errors: Some(errors),
            validated_data: None,
        }
    }
}

// ============================================================================
// 便利宏
// ============================================================================

/// 快速创建字段验证配置
///
/// # 示例
///
/// ```rust,no_run
/// use common::field_validation;
///
/// let field = field_validation!("username", required, length_range(3, 50));
/// ```
#[macro_export]
macro_rules! field_validation {
    ($name:expr, required $(, $method:ident($($arg:expr),*))*) => {{
        let mut field = $crate::request_validation::FieldValidation::new($name).required();
        $(field = field.$method($($arg),*);)*
        field
    }};
    ($name:expr $(, $method:ident($($arg:expr),*))*) => {{
        let mut field = $crate::request_validation::FieldValidation::new($name);
        $(field = field.$method($($arg),*);)*
        field
    }};
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_length_rule_min() {
        let rule = LengthRule {
            min: Some(3),
            max: None,
        };
        assert!(rule.validate(&json!("abc"), "test").is_ok());
        assert!(rule.validate(&json!("ab"), "test").is_err());
        assert!(rule.validate(&json!("abcd"), "test").is_ok());
    }

    #[test]
    fn test_length_rule_max() {
        let rule = LengthRule {
            min: None,
            max: Some(5),
        };
        assert!(rule.validate(&json!("abc"), "test").is_ok());
        assert!(rule.validate(&json!("abcdef"), "test").is_err());
    }

    #[test]
    fn test_length_rule_range() {
        let rule = LengthRule {
            min: Some(2),
            max: Some(5),
        };
        assert!(rule.validate(&json!("ab"), "test").is_ok());
        assert!(rule.validate(&json!("a"), "test").is_err());
        assert!(rule.validate(&json!("abcdef"), "test").is_err());
    }

    #[test]
    fn test_range_rule() {
        let rule = RangeRule {
            min: Some(0.0),
            max: Some(100.0),
        };
        assert!(rule.validate(&json!(50), "test").is_ok());
        assert!(rule.validate(&json!(-1), "test").is_err());
        assert!(rule.validate(&json!(101), "test").is_err());
    }

    #[test]
    fn test_pattern_rule() {
        let rule = PatternRule {
            pattern: r"^[a-z]+$".to_string(),
            error_message: "只允许小写字母".to_string(),
        };
        assert!(rule.validate(&json!("abc"), "test").is_ok());
        assert!(rule.validate(&json!("ABC"), "test").is_err());
        assert!(rule.validate(&json!("abc123"), "test").is_err());
    }

    #[test]
    fn test_enum_rule() {
        let rule = EnumRule {
            allowed_values: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        };
        assert!(rule.validate(&json!("a"), "test").is_ok());
        assert!(rule.validate(&json!("d"), "test").is_err());
    }

    #[test]
    fn test_email_validation() {
        let field = FieldValidation::new("email").required().email();
        let schema = ValidationSchema::new().field(field);

        assert!(schema.validate(&json!({"email": "test@example.com"})).is_ok());
        assert!(schema.validate(&json!({"email": "invalid"})).is_err());
        assert!(schema.validate(&json!({"email": ""})).is_err());
        assert!(schema.validate(&json!({})).is_err());
    }

    #[test]
    fn test_uuid_validation() {
        let field = FieldValidation::new("id").required().uuid();
        let schema = ValidationSchema::new().field(field);

        assert!(schema.validate(&json!({"id": "550e8400-e29b-41d4-a716-446655440000"})).is_ok());
        assert!(schema.validate(&json!({"id": "not-a-uuid"})).is_err());
    }

    #[test]
    fn test_required_field() {
        let field = FieldValidation::new("name").required();
        let schema = ValidationSchema::new().field(field);

        assert!(schema.validate(&json!({"name": "test"})).is_ok());
        assert!(schema.validate(&json!({})).is_err());
        assert!(schema.validate(&json!({"name": null})).is_err());
        assert!(schema.validate(&json!({"name": ""})).is_err());
    }

    #[test]
    fn test_optional_field() {
        let field = FieldValidation::new("bio").optional().max_length(500);
        let schema = ValidationSchema::new().field(field);

        assert!(schema.validate(&json!({"bio": "test bio"})).is_ok());
        assert!(schema.validate(&json!({})).is_ok());
        assert!(schema.validate(&json!({"bio": null})).is_ok());
    }

    #[test]
    fn test_custom_validator() {
        let field = FieldValidation::new("age").required().custom("positive", |value| {
            if let Some(n) = value.as_i64() {
                if n < 0 {
                    return Err("年龄不能为负数".to_string());
                }
            }
            Ok(())
        });
        let schema = ValidationSchema::new().field(field);

        assert!(schema.validate(&json!({"age": 25})).is_ok());
        assert!(schema.validate(&json!({"age": -5})).is_err());
    }

    #[test]
    fn test_multiple_fields() {
        let schema = ValidationSchema::new()
            .field(FieldValidation::new("username").required().length_range(3, 50))
            .field(FieldValidation::new("email").required().email())
            .field(FieldValidation::new("age").optional().range(0.0, 200.0));

        // 全部有效
        assert!(schema.validate(&json!({
            "username": "testuser",
            "email": "test@example.com",
            "age": 25
        })).is_ok());

        // 多个字段无效
        let result = schema.validate(&json!({
            "username": "ab",
            "email": "invalid",
            "age": -1
        }));
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.errors.len(), 3);
    }

    #[test]
    fn test_validation_error_response() {
        let schema = ValidationSchema::new()
            .field(FieldValidation::new("name").required());

        let result = schema.validate(&json!({}));
        assert!(result.is_err());

        let response = result.unwrap_err().to_response();
        assert!(!response.success);
        assert_eq!(response.code, "VALIDATION_ERROR");
        assert!(!response.errors.is_empty());
    }

    #[test]
    fn test_predefined_user_register_schema() {
        let schema = ValidationSchemas::user_register();

        // 有效注册
        assert!(schema.validate(&json!({
            "username": "test_user",
            "email": "test@example.com",
            "password": "StrongPass1"
        })).is_ok());

        // 密码太弱
        assert!(schema.validate(&json!({
            "username": "test_user",
            "email": "test@example.com",
            "password": "weakpass"
        })).is_err());

        // 用户名太短
        assert!(schema.validate(&json!({
            "username": "ab",
            "email": "test@example.com",
            "password": "StrongPass1"
        })).is_err());
    }

    #[test]
    fn test_predefined_send_message_schema() {
        let schema = ValidationSchemas::send_message();

        // 有效消息
        assert!(schema.validate(&json!({
            "conversation_id": "550e8400-e29b-41d4-a716-446655440000",
            "content": "Hello!"
        })).is_ok());

        // 无效 UUID
        assert!(schema.validate(&json!({
            "conversation_id": "not-a-uuid",
            "content": "Hello!"
        })).is_err());

        // 内容为空
        assert!(schema.validate(&json!({
            "conversation_id": "550e8400-e29b-41d4-a716-446655440000",
            "content": ""
        })).is_err());
    }

    #[test]
    fn test_field_validation_builder_chaining() {
        let field = FieldValidation::new("test")
            .required()
            .description("测试字段")
            .min_length(1)
            .max_length(100)
            .pattern(r"^[a-z]+$", "只允许小写字母");

        assert_eq!(field.field_name, "test");
        assert!(field.required);
        assert!(field.description.is_some());
        assert_eq!(field.rules.len(), 3); // min_length, max_length, pattern
    }

    #[test]
    fn test_array_validation() {
        let schema = ValidationSchema::new()
            .field(
                FieldValidation::new("items")
                    .required()
                    .custom("non_empty", |value| {
                        if let Some(arr) = value.as_array() {
                            if arr.is_empty() {
                                return Err("列表不能为空".to_string());
                            }
                        }
                        Ok(())
                    }),
            );

        assert!(schema.validate(&json!({"items": [1, 2, 3]})).is_ok());
        assert!(schema.validate(&json!({"items": []})).is_err());
    }

    #[test]
    fn test_url_validation() {
        let field = FieldValidation::new("url").required().url();
        let schema = ValidationSchema::new().field(field);

        assert!(schema.validate(&json!({"url": "https://example.com"})).is_ok());
        assert!(schema.validate(&json!({"url": "http://example.com/path"})).is_ok());
        assert!(schema.validate(&json!({"url": "not-a-url"})).is_err());
    }

    #[test]
    fn test_enum_validation() {
        let field = FieldValidation::new("type")
            .required()
            .enum_values(vec!["text", "image", "file"]);
        let schema = ValidationSchema::new().field(field);

        assert!(schema.validate(&json!({"type": "text"})).is_ok());
        assert!(schema.validate(&json!({"type": "video"})).is_err());
    }
}
