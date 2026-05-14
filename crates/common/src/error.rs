use thiserror::Error;
use std::fmt;

/// 错误上下文信息
///
/// 用于追踪错误的来源和位置，方便调试
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// 错误发生的文件
    pub file: &'static str,
    /// 错误发生的行号
    pub line: u32,
    /// 错误发生的函数
    pub function: &'static str,
    /// 附加的上下文信息
    pub message: Option<String>,
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}:{}:{}]", self.file, self.line, self.function)?;
        if let Some(msg) = &self.message {
            write!(f, " {}", msg)?;
        }
        Ok(())
    }
}

/// 创建错误上下文的宏
///
/// 用法：
/// ```rust
/// let err = AppError::NotFound("user".to_string()).with_context(error_context!("user lookup"));
/// ```
#[macro_export]
macro_rules! error_context {
    ($msg:expr) => {
        $crate::error::ErrorContext {
            file: file!(),
            line: line!(),
            function: module_path!(),
            message: Some($msg.to_string()),
        }
    };
    () => {
        $crate::error::ErrorContext {
            file: file!(),
            line: line!(),
            function: module_path!(),
            message: None,
        }
    };
}

/// 应用错误类型
///
/// 这个枚举定义了整个应用中可能遇到的所有错误类型
/// 使用thiserror库实现自动的错误转换和格式化
#[derive(Debug, Error)]
pub enum AppError {
    /// 数据库相关错误
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),

    /// Redis相关错误
    #[error("缓存服务错误: {0}")]
    Redis(#[from] redis::RedisError),

    /// MongoDB相关错误
    #[error("数据库错误: {0}")]
    MongoDb(#[from] mongodb::error::Error),

    /// IO操作错误
    #[error("系统IO错误: {0}")]
    Io(#[from] std::io::Error),

    /// 序列化/反序列化错误
    #[error("数据格式错误: {0}")]
    Serialization(#[from] serde_json::Error),

    /// 认证错误 - 用户身份验证失败
    #[error("认证失败: {0}")]
    Auth(String),

    /// 授权错误 - 权限不足
    #[error("权限不足: {0}")]
    Authorization(String),

    /// 资源未找到错误
    #[error("资源未找到: {0}")]
    NotFound(String),

    /// 数据验证错误
    #[error("数据验证失败: {0}")]
    Validation(String),

    /// 请求频率限制错误
    #[error("请求过于频繁，请稍后再试")]
    RateLimited(String),

    /// 内部服务器错误
    #[error("服务器内部错误: {0}")]
    Internal(String),

    /// 错误的请求
    #[error("请求参数错误: {0}")]
    BadRequest(String),

    /// HTTP客户端错误
    #[error("外部服务调用失败: {0}")]
    Http(String),

    /// WebSocket错误
    #[error("WebSocket连接错误: {0}")]
    WebSocket(String),

    /// 文件操作错误
    #[error("文件操作失败: {0}")]
    File(String),

    /// 配置错误
    #[error("配置错误: {0}")]
    Config(String),

    /// Token过期错误
    #[error("登录已过期，请重新登录")]
    TokenExpired,

    /// Token无效错误
    #[error("无效的认证令牌: {0}")]
    InvalidToken(String),

    /// 用户已存在错误
    #[error("用户已存在: {0}")]
    UserAlreadyExists(String),

    /// 设备未找到错误
    #[error("设备未找到: {0}")]
    DeviceNotFound(String),

    /// 消息未找到错误
    #[error("消息未找到: {0}")]
    MessageNotFound(String),

    /// 会话未找到错误
    #[error("会话未找到: {0}")]
    ConversationNotFound(String),

    /// 文件未找到错误
    #[error("文件未找到: {0}")]
    FileNotFound(String),

    /// 配置未找到错误
    #[error("配置项未找到: {0}")]
    ConfigNotFound(String),

    /// 服务不可用错误
    #[error("服务暂时不可用: {0}")]
    ServiceUnavailable(String),

    /// 超时错误
    #[error("请求超时: {0}")]
    Timeout(String),

    /// 带上下文的错误（包装其他错误并添加上下文信息）
    #[error("{context}: {source}")]
    WithContext {
        source: Box<AppError>,
        context: ErrorContext,
    },
}

/// 错误码枚举
///
/// 为每种错误类型定义唯一的错误码，方便客户端识别和处理
/// 格式：E + 2位分类码 + 3位序号
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ErrorCode {
    // 认证相关 (E1xxx)
    /// 认证失败
    AuthFailed = 1001,
    /// Token 过期
    TokenExpired = 1002,
    /// Token 无效
    InvalidToken = 1003,
    /// 权限不足
    InsufficientPermission = 1004,
    /// 用户已存在
    UserAlreadyExists = 1005,

    // 资源相关 (E2xxx)
    /// 资源未找到
    ResourceNotFound = 2001,
    /// 用户未找到
    UserNotFound = 2002,
    /// 消息未找到
    MessageNotFound = 2003,
    /// 会话未找到
    ConversationNotFound = 2004,
    /// 文件未找到
    FileNotFound = 2005,
    /// 配置未找到
    ConfigNotFound = 2006,
    /// 设备未找到
    DeviceNotFound = 2007,

    // 请求相关 (E3xxx)
    /// 请求参数无效
    InvalidRequest = 3001,
    /// 请求体过大
    RequestTooLarge = 3002,
    /// 请求频率超限
    RateLimited = 3003,
    /// 验证失败
    ValidationFailed = 3004,

    // 服务器相关 (E5xxx)
    /// 内部服务器错误
    InternalError = 5001,
    /// 数据库错误
    DatabaseError = 5002,
    /// 缓存错误
    CacheError = 5003,
    /// 外部服务错误
    ExternalServiceError = 5004,
    /// 服务不可用
    ServiceUnavailable = 5005,
    /// 超时
    Timeout = 5006,
    /// 文件操作错误
    FileError = 5007,
    /// 配置错误
    ConfigError = 5008,
}

impl ErrorCode {
    /// 获取错误码的数值
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    /// 获取错误码的描述
    pub fn description(&self) -> &'static str {
        match self {
            ErrorCode::AuthFailed => "认证失败",
            ErrorCode::TokenExpired => "Token已过期",
            ErrorCode::InvalidToken => "Token无效",
            ErrorCode::InsufficientPermission => "权限不足",
            ErrorCode::UserAlreadyExists => "用户已存在",
            ErrorCode::ResourceNotFound => "资源未找到",
            ErrorCode::UserNotFound => "用户未找到",
            ErrorCode::MessageNotFound => "消息未找到",
            ErrorCode::ConversationNotFound => "会话未找到",
            ErrorCode::FileNotFound => "文件未找到",
            ErrorCode::ConfigNotFound => "配置未找到",
            ErrorCode::DeviceNotFound => "设备未找到",
            ErrorCode::InvalidRequest => "请求参数无效",
            ErrorCode::RequestTooLarge => "请求体过大",
            ErrorCode::RateLimited => "请求频率超限",
            ErrorCode::ValidationFailed => "验证失败",
            ErrorCode::InternalError => "内部服务器错误",
            ErrorCode::DatabaseError => "数据库错误",
            ErrorCode::CacheError => "缓存错误",
            ErrorCode::ExternalServiceError => "外部服务错误",
            ErrorCode::ServiceUnavailable => "服务不可用",
            ErrorCode::Timeout => "请求超时",
            ErrorCode::FileError => "文件操作错误",
            ErrorCode::ConfigError => "配置错误",
        }
    }

    /// 获取用户友好的错误消息
    pub fn user_message(&self) -> &'static str {
        match self {
            ErrorCode::AuthFailed => "登录失败，请检查用户名和密码",
            ErrorCode::TokenExpired => "登录已过期，请重新登录",
            ErrorCode::InvalidToken => "认证令牌无效，请重新登录",
            ErrorCode::InsufficientPermission => "您没有权限执行此操作",
            ErrorCode::UserAlreadyExists => "该用户名已被注册",
            ErrorCode::ResourceNotFound => "请求的资源不存在",
            ErrorCode::UserNotFound => "用户不存在",
            ErrorCode::MessageNotFound => "消息不存在或已被删除",
            ErrorCode::ConversationNotFound => "会话不存在或已被删除",
            ErrorCode::FileNotFound => "文件不存在或已被删除",
            ErrorCode::ConfigNotFound => "配置项不存在",
            ErrorCode::DeviceNotFound => "设备不存在",
            ErrorCode::InvalidRequest => "请求参数无效，请检查输入",
            ErrorCode::RequestTooLarge => "请求数据过大",
            ErrorCode::RateLimited => "请求过于频繁，请稍后再试",
            ErrorCode::ValidationFailed => "数据验证失败，请检查输入",
            ErrorCode::InternalError => "服务器内部错误，请稍后再试",
            ErrorCode::DatabaseError => "数据服务异常，请稍后再试",
            ErrorCode::CacheError => "缓存服务异常，请稍后再试",
            ErrorCode::ExternalServiceError => "外部服务异常，请稍后再试",
            ErrorCode::ServiceUnavailable => "服务暂时不可用，请稍后再试",
            ErrorCode::Timeout => "请求超时，请稍后再试",
            ErrorCode::FileError => "文件操作失败，请稍后再试",
            ErrorCode::ConfigError => "系统配置错误",
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "E{}", self.as_i32())
    }
}

impl AppError {
    /// 获取对应的错误码
    pub fn error_code(&self) -> ErrorCode {
        match self {
            AppError::Database(_) => ErrorCode::DatabaseError,
            AppError::Redis(_) => ErrorCode::CacheError,
            AppError::MongoDb(_) => ErrorCode::DatabaseError,
            AppError::Io(_) => ErrorCode::InternalError,
            AppError::Serialization(_) => ErrorCode::InternalError,
            AppError::Auth(_) => ErrorCode::AuthFailed,
            AppError::Authorization(_) => ErrorCode::InsufficientPermission,
            AppError::NotFound(_) => ErrorCode::ResourceNotFound,
            AppError::Validation(_) => ErrorCode::ValidationFailed,
            AppError::RateLimited(_) => ErrorCode::RateLimited,
            AppError::Internal(_) => ErrorCode::InternalError,
            AppError::BadRequest(_) => ErrorCode::InvalidRequest,
            AppError::Http(_) => ErrorCode::ExternalServiceError,
            AppError::WebSocket(_) => ErrorCode::InternalError,
            AppError::File(_) => ErrorCode::FileError,
            AppError::Config(_) => ErrorCode::ConfigError,
            AppError::TokenExpired => ErrorCode::TokenExpired,
            AppError::InvalidToken(_) => ErrorCode::InvalidToken,
            AppError::UserAlreadyExists(_) => ErrorCode::UserAlreadyExists,
            AppError::DeviceNotFound(_) => ErrorCode::DeviceNotFound,
            AppError::MessageNotFound(_) => ErrorCode::MessageNotFound,
            AppError::ConversationNotFound(_) => ErrorCode::ConversationNotFound,
            AppError::FileNotFound(_) => ErrorCode::FileNotFound,
            AppError::ConfigNotFound(_) => ErrorCode::ConfigNotFound,
            AppError::ServiceUnavailable(_) => ErrorCode::ServiceUnavailable,
            AppError::Timeout(_) => ErrorCode::Timeout,
            AppError::WithContext { source, .. } => source.error_code(),
        }
    }
}

/// 应用结果类型别名
///
/// 简化了返回Result<T, AppError>的写法
pub type Result<T> = std::result::Result<T, AppError>;

/// 将 anyhow::Error 转换为 AppError
/// 用于 DatabaseManager 等返回 anyhow::Error 的地方
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}

/// 将AppError转换为HTTP状态码
impl AppError {
    /// 获取对应的HTTP状态码
    pub fn status_code(&self) -> http::StatusCode {
        match self {
            AppError::Database(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Redis(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            AppError::MongoDb(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Io(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Serialization(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Auth(_) => http::StatusCode::UNAUTHORIZED,
            AppError::Authorization(_) => http::StatusCode::FORBIDDEN,
            AppError::NotFound(_) => http::StatusCode::NOT_FOUND,
            AppError::Validation(_) => http::StatusCode::BAD_REQUEST,
            AppError::RateLimited(_) => http::StatusCode::TOO_MANY_REQUESTS,
            AppError::Internal(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            AppError::BadRequest(_) => http::StatusCode::BAD_REQUEST,
            AppError::Http(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            AppError::WebSocket(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            AppError::File(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Config(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            AppError::TokenExpired => http::StatusCode::UNAUTHORIZED,
            AppError::InvalidToken(_) => http::StatusCode::UNAUTHORIZED,
            AppError::UserAlreadyExists(_) => http::StatusCode::CONFLICT,
            AppError::DeviceNotFound(_) => http::StatusCode::NOT_FOUND,
            AppError::MessageNotFound(_) => http::StatusCode::NOT_FOUND,
            AppError::ConversationNotFound(_) => http::StatusCode::NOT_FOUND,
            AppError::FileNotFound(_) => http::StatusCode::NOT_FOUND,
            AppError::ConfigNotFound(_) => http::StatusCode::NOT_FOUND,
            AppError::ServiceUnavailable(_) => http::StatusCode::SERVICE_UNAVAILABLE,
            AppError::Timeout(_) => http::StatusCode::GATEWAY_TIMEOUT,
            AppError::WithContext { source, .. } => source.status_code(),
        }
    }

    /// 添加错误上下文
    ///
    /// 用于追踪错误的来源和位置
    ///
    /// # Examples
    ///
    /// ```rust
    /// use common::error::*;
    ///
    /// let err = AppError::NotFound("user".to_string())
    ///     .with_context(error_context!("failed to find user"));
    /// ```
    pub fn with_context(self, context: ErrorContext) -> Self {
        AppError::WithContext {
            source: Box::new(self),
            context,
        }
    }

    /// 获取错误的根因（最内层的错误）
    pub fn root_cause(&self) -> &AppError {
        match self {
            AppError::WithContext { source, .. } => source.root_cause(),
            _ => self,
        }
    }

    /// 获取完整的错误链信息
    pub fn error_chain(&self) -> Vec<String> {
        let mut chain = Vec::new();
        self.collect_chain(&mut chain);
        chain
    }

    fn collect_chain(&self, chain: &mut Vec<String>) {
        chain.push(self.to_string());
        if let AppError::WithContext { source, context } = self {
            chain.push(format!("  caused by: {}", context));
            source.collect_chain(chain);
        }
    }

    /// 判断是否为客户端错误（4xx）
    pub fn is_client_error(&self) -> bool {
        let code = self.status_code().as_u16();
        code >= 400 && code < 500
    }

    /// 判断是否为服务器错误（5xx）
    pub fn is_server_error(&self) -> bool {
        let code = self.status_code().as_u16();
        code >= 500 && code < 600
    }

    /// 获取用户友好的错误消息
    pub fn user_message(&self) -> String {
        match self {
            AppError::Auth(_) => "登录失败，请检查用户名和密码".to_string(),
            AppError::TokenExpired => "登录已过期，请重新登录".to_string(),
            AppError::InvalidToken(_) => "认证令牌无效，请重新登录".to_string(),
            AppError::Authorization(_) => "您没有权限执行此操作".to_string(),
            AppError::UserAlreadyExists(_) => "该用户名已被注册".to_string(),
            AppError::NotFound(_) => "请求的资源不存在".to_string(),
            AppError::Validation(msg) => format!("数据验证失败: {}", msg),
            AppError::RateLimited(_) => "请求过于频繁，请稍后再试".to_string(),
            AppError::BadRequest(msg) => format!("请求参数错误: {}", msg),
            AppError::Timeout(_) => "请求超时，请稍后再试".to_string(),
            AppError::ServiceUnavailable(_) => "服务暂时不可用，请稍后再试".to_string(),
            AppError::WithContext { source, .. } => source.user_message(),
            _ => "操作失败，请稍后再试".to_string(),
        }
    }
}

/// 实现 axum 的 IntoResponse trait，使 AppError 可以直接作为 axum handler 的返回类型
impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = self.status_code();
        let error_code = self.error_code();
        let user_msg = self.user_message();
        let body = axum::Json(serde_json::json!({
            "success": false,
            "error": {
                "code": error_code.as_i32(),
                "code_str": error_code.to_string(),
                "type": error_code.description(),
                "message": user_msg,
                "user_message": error_code.user_message(),
            },
            "data": null,
            "timestamp": chrono::Utc::now().timestamp(),
        }));
        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_messages() {
        let err = AppError::Auth("invalid".to_string());
        assert!(err.to_string().contains("认证失败"));

        let err = AppError::NotFound("resource".to_string());
        assert!(err.to_string().contains("资源未找到"));

        let err = AppError::Validation("field".to_string());
        assert!(err.to_string().contains("数据验证失败"));

        let err = AppError::TokenExpired;
        assert!(err.to_string().contains("登录已过期"));
    }

    #[test]
    fn test_status_codes_auth_errors() {
        assert_eq!(AppError::Auth("x".into()).status_code(), http::StatusCode::UNAUTHORIZED);
        assert_eq!(AppError::TokenExpired.status_code(), http::StatusCode::UNAUTHORIZED);
        assert_eq!(AppError::InvalidToken("x".into()).status_code(), http::StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_status_codes_not_found_errors() {
        assert_eq!(AppError::NotFound("x".into()).status_code(), http::StatusCode::NOT_FOUND);
        assert_eq!(AppError::DeviceNotFound("x".into()).status_code(), http::StatusCode::NOT_FOUND);
        assert_eq!(AppError::MessageNotFound("x".into()).status_code(), http::StatusCode::NOT_FOUND);
        assert_eq!(AppError::ConversationNotFound("x".into()).status_code(), http::StatusCode::NOT_FOUND);
        assert_eq!(AppError::FileNotFound("x".into()).status_code(), http::StatusCode::NOT_FOUND);
        assert_eq!(AppError::ConfigNotFound("x".into()).status_code(), http::StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_status_codes_client_errors() {
        assert_eq!(AppError::Validation("x".into()).status_code(), http::StatusCode::BAD_REQUEST);
        assert_eq!(AppError::BadRequest("x".into()).status_code(), http::StatusCode::BAD_REQUEST);
        assert_eq!(AppError::RateLimited("x".into()).status_code(), http::StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(AppError::Authorization("x".into()).status_code(), http::StatusCode::FORBIDDEN);
        assert_eq!(AppError::UserAlreadyExists("x".into()).status_code(), http::StatusCode::CONFLICT);
    }

    #[test]
    fn test_status_codes_server_errors() {
        assert_eq!(AppError::Internal("x".into()).status_code(), http::StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(AppError::ServiceUnavailable("x".into()).status_code(), http::StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(AppError::Timeout("x".into()).status_code(), http::StatusCode::GATEWAY_TIMEOUT);
    }

    #[test]
    fn test_from_anyhow_error() {
        let anyhow_err = anyhow::anyhow!("test error");
        let app_err: AppError = anyhow_err.into();
        match app_err {
            AppError::Internal(msg) => assert_eq!(msg, "test error"),
            _ => panic!("Expected Internal error"),
        }
    }

    #[test]
    fn test_error_code_mapping() {
        assert_eq!(AppError::Auth("x".into()).error_code(), ErrorCode::AuthFailed);
        assert_eq!(AppError::TokenExpired.error_code(), ErrorCode::TokenExpired);
        assert_eq!(AppError::InvalidToken("x".into()).error_code(), ErrorCode::InvalidToken);
        assert_eq!(AppError::Authorization("x".into()).error_code(), ErrorCode::InsufficientPermission);
        assert_eq!(AppError::UserAlreadyExists("x".into()).error_code(), ErrorCode::UserAlreadyExists);
        assert_eq!(AppError::NotFound("x".into()).error_code(), ErrorCode::ResourceNotFound);
        assert_eq!(AppError::Validation("x".into()).error_code(), ErrorCode::ValidationFailed);
        assert_eq!(AppError::RateLimited("x".into()).error_code(), ErrorCode::RateLimited);
        assert_eq!(AppError::Internal("x".into()).error_code(), ErrorCode::InternalError);
        assert_eq!(AppError::Database(sqlx::Error::RowNotFound).error_code(), ErrorCode::DatabaseError);
        assert_eq!(AppError::BadRequest("x".into()).error_code(), ErrorCode::InvalidRequest);
        assert_eq!(AppError::ServiceUnavailable("x".into()).error_code(), ErrorCode::ServiceUnavailable);
        assert_eq!(AppError::Timeout("x".into()).error_code(), ErrorCode::Timeout);
    }

    #[test]
    fn test_error_code_display() {
        assert_eq!(ErrorCode::AuthFailed.to_string(), "E1001");
        assert_eq!(ErrorCode::TokenExpired.to_string(), "E1002");
        assert_eq!(ErrorCode::ResourceNotFound.to_string(), "E2001");
        assert_eq!(ErrorCode::InvalidRequest.to_string(), "E3001");
        assert_eq!(ErrorCode::InternalError.to_string(), "E5001");
    }

    #[test]
    fn test_error_code_as_i32() {
        assert_eq!(ErrorCode::AuthFailed.as_i32(), 1001);
        assert_eq!(ErrorCode::ResourceNotFound.as_i32(), 2001);
        assert_eq!(ErrorCode::InvalidRequest.as_i32(), 3001);
        assert_eq!(ErrorCode::InternalError.as_i32(), 5001);
    }

    #[test]
    fn test_error_code_description() {
        assert_eq!(ErrorCode::AuthFailed.description(), "认证失败");
        assert_eq!(ErrorCode::TokenExpired.description(), "Token已过期");
        assert_eq!(ErrorCode::ResourceNotFound.description(), "资源未找到");
        assert_eq!(ErrorCode::InvalidRequest.description(), "请求参数无效");
        assert_eq!(ErrorCode::InternalError.description(), "内部服务器错误");
    }

    #[test]
    fn test_error_code_user_message() {
        assert_eq!(ErrorCode::AuthFailed.user_message(), "登录失败，请检查用户名和密码");
        assert_eq!(ErrorCode::TokenExpired.user_message(), "登录已过期，请重新登录");
        assert_eq!(ErrorCode::ResourceNotFound.user_message(), "请求的资源不存在");
        assert_eq!(ErrorCode::RateLimited.user_message(), "请求过于频繁，请稍后再试");
    }

    #[test]
    fn test_error_code_equality() {
        assert_eq!(ErrorCode::AuthFailed, ErrorCode::AuthFailed);
        assert_ne!(ErrorCode::AuthFailed, ErrorCode::TokenExpired);
    }

    #[test]
    fn test_error_code_serialization() {
        let code = ErrorCode::AuthFailed;
        let json = serde_json::to_string(&code).unwrap();
        assert_eq!(json, "\"AuthFailed\"");

        let deserialized: ErrorCode = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ErrorCode::AuthFailed);
    }

    #[test]
    fn test_error_context() {
        let ctx = ErrorContext {
            file: "test.rs",
            line: 42,
            function: "test_func",
            message: Some("test message".to_string()),
        };
        assert!(ctx.to_string().contains("test.rs:42:test_func"));
        assert!(ctx.to_string().contains("test message"));
    }

    #[test]
    fn test_error_with_context() {
        let err = AppError::NotFound("user".to_string())
            .with_context(ErrorContext {
                file: "test.rs",
                line: 42,
                function: "test_func",
                message: Some("user lookup failed".to_string()),
            });
        
        assert!(err.to_string().contains("user lookup failed"));
        assert!(err.to_string().contains("资源未找到"));
    }

    #[test]
    fn test_error_root_cause() {
        let inner = AppError::NotFound("user".to_string());
        let err = inner.with_context(ErrorContext {
            file: "test.rs",
            line: 42,
            function: "test_func",
            message: Some("lookup failed".to_string()),
        });
        
        let root = err.root_cause();
        assert!(root.to_string().contains("资源未找到"));
    }

    #[test]
    fn test_error_chain() {
        let err = AppError::NotFound("user".to_string())
            .with_context(ErrorContext {
                file: "test.rs",
                line: 42,
                function: "test_func",
                message: Some("lookup failed".to_string()),
            });
        
        let chain = err.error_chain();
        assert!(chain.len() >= 2);
        assert!(chain[0].contains("资源未找到"));
    }

    #[test]
    fn test_is_client_error() {
        assert!(AppError::Auth("x".into()).is_client_error());
        assert!(AppError::NotFound("x".into()).is_client_error());
        assert!(AppError::Validation("x".into()).is_client_error());
        assert!(!AppError::Internal("x".into()).is_client_error());
    }

    #[test]
    fn test_is_server_error() {
        assert!(AppError::Internal("x".into()).is_server_error());
        assert!(AppError::ServiceUnavailable("x".into()).is_server_error());
        assert!(!AppError::Auth("x".into()).is_server_error());
    }

    #[test]
    fn test_user_message() {
        let err = AppError::Auth("invalid".to_string());
        assert_eq!(err.user_message(), "登录失败，请检查用户名和密码");

        let err = AppError::TokenExpired;
        assert_eq!(err.user_message(), "登录已过期，请重新登录");

        let err = AppError::Validation("email".to_string());
        assert_eq!(err.user_message(), "数据验证失败: email");
    }
}
