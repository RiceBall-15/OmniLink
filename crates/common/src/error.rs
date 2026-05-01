use thiserror::Error;

/// 应用错误类型
/// 
/// 这个枚举定义了整个应用中可能遇到的所有错误类型
/// 使用thiserror库实现自动的错误转换和格式化
#[derive(Debug, Error)]
pub enum AppError {
    /// 数据库相关错误
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Redis相关错误
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    /// MongoDB相关错误
    #[error("MongoDB error: {0}")]
    MongoDb(#[from] mongodb::error::Error),

    /// IO操作错误
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// 序列化/反序列化错误
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// 认证错误 - 用户身份验证失败
    #[error("Authentication error: {0}")]
    Auth(String),

    /// 授权错误 - 权限不足
    #[error("Authorization error: {0}")]
    Authorization(String),

    /// 资源未找到错误
    #[error("Not found: {0}")]
    NotFound(String),

    /// 数据验证错误
    #[error("Validation error: {0}")]
    Validation(String),

    /// 请求频率限制错误
    #[error("Rate limited: {0}")]
    RateLimited(String),

    /// 内部服务器错误
    #[error("Internal server error: {0}")]
    Internal(String),

    /// 错误的请求
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// HTTP客户端错误
    #[error("HTTP client error: {0}")]
    Http(String),

    /// WebSocket错误
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// 文件操作错误
    #[error("File error: {0}")]
    File(String),

    /// 配置错误
    #[error("Configuration error: {0}")]
    Config(String),

    /// Token过期错误
    #[error("Token expired")]
    TokenExpired,

    /// Token无效错误
    #[error("Invalid token: {0}")]
    InvalidToken(String),

    /// 用户已存在错误
    #[error("User already exists: {0}")]
    UserAlreadyExists(String),

    /// 设备未找到错误
    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    /// 消息未找到错误
    #[error("Message not found: {0}")]
    MessageNotFound(String),

    /// 会话未找到错误
    #[error("Conversation not found: {0}")]
    ConversationNotFound(String),

    /// 文件未找到错误
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// 配置未找到错误
    #[error("Config not found: {0}")]
    ConfigNotFound(String),

    /// 服务不可用错误
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    /// 超时错误
    #[error("Timeout: {0}")]
    Timeout(String),
}

/// 应用结果类型别名
/// 
/// 简化了返回Result<T, AppError>的写法
pub type Result<T> = std::result::Result<T, AppError>;

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
        }
    }
}