use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use validator::Validate;

/// 用户数据模型
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct User {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "username")]
    pub username: String,
    #[serde(rename = "email")]
    pub email: String,
    #[serde(rename = "avatar", skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(rename = "nickname", skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    #[serde(rename = "bio", skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(rename = "statusMessage", skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

/// 数据库中的用户实体（包含密码）
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserEntity {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub avatar: Option<String>,
    pub nickname: Option<String>,
    pub bio: Option<String>,
    pub status_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserEntity {
    /// 转换为 API 响应的 User 格式
    pub fn to_user(&self) -> User {
        User {
            id: self.id.to_string(),
            username: self.username.clone(),
            email: self.email.clone(),
            avatar: self.avatar.clone(),
            nickname: self.nickname.clone(),
            bio: self.bio.clone(),
            status_message: self.status_message.clone(),
            created_at: self.created_at.to_rfc3339(),
            updated_at: self.updated_at.to_rfc3339(),
        }
    }
}

/// 用户注册请求
#[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 20, message = "用户名长度必须在 3-20 个字符之间"))]
    pub username: String,
    #[validate(email(message = "邮箱格式不正确"))]
    pub email: String,
    #[validate(length(min = 8, message = "密码至少需要 8 个字符"))]
    pub password: String,
}

/// 用户登录请求
#[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
pub struct LoginRequest {
    #[validate(email(message = "邮箱格式不正确"))]
    pub email: String,
    #[validate(length(min = 1, message = "密码不能为空"))]
    pub password: String,
}

/// 登录响应
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct LoginResponse {
    pub token: String,
    pub user: User,
}

/// 用户更新请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserRequest {
    #[validate(length(min = 3, max = 20, message = "用户名长度必须在 3-20 个字符之间"))]
    pub username: Option<String>,
    #[validate(email(message = "邮箱格式不正确"))]
    pub email: Option<String>,
    pub avatar: Option<String>,
}

/// 用户资料更新请求（扩展字段）
#[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
pub struct UpdateUserProfileRequest {
    #[validate(length(min = 1, max = 50, message = "昵称长度必须在 1-50 个字符之间"))]
    pub nickname: Option<String>,
    #[validate(length(max = 500, message = "个人简介不能超过 500 个字符"))]
    pub bio: Option<String>,
    #[validate(length(max = 100, message = "状态消息不能超过 100 个字符"))]
    pub status_message: Option<String>,
    pub avatar: Option<String>,
}

/// JWT Claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // 用户 ID
    pub email: String,
    pub exp: usize,   // 过期时间
    pub iat: usize,   // 签发时间
}

/// API 统一响应格式
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

/// 标准化错误码枚举
///
/// 定义所有 API 错误的统一错误码，格式为 `EXXXX`：
/// - E1xxx: 认证/授权错误
/// - E2xxx: 资源错误
/// - E3xxx: 验证/请求错误
/// - E4xxx: 业务逻辑错误
/// - E5xxx: 服务器内部错误
#[derive(Debug, Clone, Copy)]
pub enum ErrorCode {
    // === 认证/授权错误 (E1xxx) ===
    /// 未提供认证 token
    Unauthorized,
    /// token 已过期
    TokenExpired,
    /// token 无效
    InvalidToken,
    /// 无权限访问
    Forbidden,

    // === 资源错误 (E2xxx) ===
    /// 资源不存在
    NotFound,
    /// 资源已存在（冲突）
    Conflict,
    /// 资源已被删除
    Gone,

    // === 验证/请求错误 (E3xxx) ===
    /// 请求参数验证失败
    ValidationFailed,
    /// 请求体 JSON 解析失败
    InvalidJson,
    /// 请求方法不允许
    MethodNotAllowed,
    /// 请求过于频繁
    RateLimited,
    /// 请求体过大
    PayloadTooLarge,

    // === 业务逻辑错误 (E4xxx) ===
    /// 用户名已存在
    UsernameTaken,
    /// 邮箱已注册
    EmailTaken,
    /// 密码错误
    WrongPassword,
    /// 账号已禁用
    AccountDisabled,
    /// 消息发送失败
    MessageSendFailed,
    /// 消息不可编辑
    MessageNotEditable,
    /// 消息不可撤回
    MessageNotRecallable,
    /// 会话不存在
    ConversationNotFound,
    /// 文件上传失败
    FileUploadFailed,
    /// 文件类型不支持
    UnsupportedFileType,
    /// 操作频率限制
    OperationRateLimited,

    // === 服务器内部错误 (E5xxx) ===
    /// 内部服务器错误
    InternalError,
    /// 数据库错误
    DatabaseError,
    /// 外部服务错误
    ExternalServiceError,
    /// 配置错误
    ConfigurationError,
}

impl ErrorCode {
    /// 获取错误码字符串
    pub fn code_str(&self) -> &'static str {
        match self {
            // 认证/授权错误
            ErrorCode::Unauthorized => "E1001",
            ErrorCode::TokenExpired => "E1002",
            ErrorCode::InvalidToken => "E1003",
            ErrorCode::Forbidden => "E1004",

            // 资源错误
            ErrorCode::NotFound => "E2001",
            ErrorCode::Conflict => "E2002",
            ErrorCode::Gone => "E2003",

            // 验证/请求错误
            ErrorCode::ValidationFailed => "E3001",
            ErrorCode::InvalidJson => "E3002",
            ErrorCode::MethodNotAllowed => "E3003",
            ErrorCode::RateLimited => "E3004",
            ErrorCode::PayloadTooLarge => "E3005",

            // 业务逻辑错误
            ErrorCode::UsernameTaken => "E4001",
            ErrorCode::EmailTaken => "E4002",
            ErrorCode::WrongPassword => "E4003",
            ErrorCode::AccountDisabled => "E4004",
            ErrorCode::MessageSendFailed => "E4005",
            ErrorCode::MessageNotEditable => "E4006",
            ErrorCode::MessageNotRecallable => "E4007",
            ErrorCode::ConversationNotFound => "E4008",
            ErrorCode::FileUploadFailed => "E4009",
            ErrorCode::UnsupportedFileType => "E4010",
            ErrorCode::OperationRateLimited => "E4011",

            // 服务器内部错误
            ErrorCode::InternalError => "E5001",
            ErrorCode::DatabaseError => "E5002",
            ErrorCode::ExternalServiceError => "E5003",
            ErrorCode::ConfigurationError => "E5004",
        }
    }

    /// 获取错误类型
    pub fn error_type(&self) -> &'static str {
        match self {
            ErrorCode::Unauthorized
            | ErrorCode::TokenExpired
            | ErrorCode::InvalidToken
            | ErrorCode::Forbidden => "auth",

            ErrorCode::NotFound
            | ErrorCode::Conflict
            | ErrorCode::Gone => "resource",

            ErrorCode::ValidationFailed
            | ErrorCode::InvalidJson
            | ErrorCode::MethodNotAllowed
            | ErrorCode::RateLimited
            | ErrorCode::PayloadTooLarge => "validation",

            ErrorCode::UsernameTaken
            | ErrorCode::EmailTaken
            | ErrorCode::WrongPassword
            | ErrorCode::AccountDisabled
            | ErrorCode::MessageSendFailed
            | ErrorCode::MessageNotEditable
            | ErrorCode::MessageNotRecallable
            | ErrorCode::ConversationNotFound
            | ErrorCode::FileUploadFailed
            | ErrorCode::UnsupportedFileType
            | ErrorCode::OperationRateLimited => "business",

            ErrorCode::InternalError
            | ErrorCode::DatabaseError
            | ErrorCode::ExternalServiceError
            | ErrorCode::ConfigurationError => "internal",
        }
    }

    /// 获取默认错误消息（中文）
    pub fn default_message(&self) -> &'static str {
        match self {
            ErrorCode::Unauthorized => "未提供认证凭据",
            ErrorCode::TokenExpired => "认证 token 已过期",
            ErrorCode::InvalidToken => "无效的认证 token",
            ErrorCode::Forbidden => "无权限访问该资源",
            ErrorCode::NotFound => "请求的资源不存在",
            ErrorCode::Conflict => "资源已存在",
            ErrorCode::Gone => "资源已被删除",
            ErrorCode::ValidationFailed => "请求参数验证失败",
            ErrorCode::InvalidJson => "请求体 JSON 格式错误",
            ErrorCode::MethodNotAllowed => "不支持的请求方法",
            ErrorCode::RateLimited => "请求过于频繁，请稍后重试",
            ErrorCode::PayloadTooLarge => "请求体过大",
            ErrorCode::UsernameTaken => "用户名已被占用",
            ErrorCode::EmailTaken => "邮箱已注册",
            ErrorCode::WrongPassword => "密码错误",
            ErrorCode::AccountDisabled => "账号已被禁用",
            ErrorCode::MessageSendFailed => "消息发送失败",
            ErrorCode::MessageNotEditable => "该消息不可编辑",
            ErrorCode::MessageNotRecallable => "该消息不可撤回",
            ErrorCode::ConversationNotFound => "会话不存在",
            ErrorCode::FileUploadFailed => "文件上传失败",
            ErrorCode::UnsupportedFileType => "不支持的文件类型",
            ErrorCode::OperationRateLimited => "操作过于频繁，请稍后重试",
            ErrorCode::InternalError => "内部服务器错误",
            ErrorCode::DatabaseError => "数据库操作失败",
            ErrorCode::ExternalServiceError => "外部服务调用失败",
            ErrorCode::ConfigurationError => "服务器配置错误",
        }
    }

    /// 获取对应的 HTTP 状态码
    pub fn status_code(&self) -> axum::http::StatusCode {
        use axum::http::StatusCode;
        match self {
            ErrorCode::Unauthorized
            | ErrorCode::TokenExpired
            | ErrorCode::InvalidToken => StatusCode::UNAUTHORIZED,
            ErrorCode::Forbidden => StatusCode::FORBIDDEN,
            ErrorCode::NotFound
            | ErrorCode::ConversationNotFound => StatusCode::NOT_FOUND,
            ErrorCode::Conflict
            | ErrorCode::UsernameTaken
            | ErrorCode::EmailTaken => StatusCode::CONFLICT,
            ErrorCode::Gone => StatusCode::GONE,
            ErrorCode::ValidationFailed
            | ErrorCode::InvalidJson
            | ErrorCode::WrongPassword
            | ErrorCode::MessageNotEditable
            | ErrorCode::MessageNotRecallable
            | ErrorCode::UnsupportedFileType => StatusCode::BAD_REQUEST,
            ErrorCode::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            ErrorCode::RateLimited
            | ErrorCode::OperationRateLimited => StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            ErrorCode::AccountDisabled => StatusCode::FORBIDDEN,
            ErrorCode::MessageSendFailed
            | ErrorCode::FileUploadFailed
            | ErrorCode::InternalError
            | ErrorCode::DatabaseError
            | ErrorCode::ExternalServiceError
            | ErrorCode::ConfigurationError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// 创建 ApiError 实例（使用默认消息）
    pub fn to_api_error(&self) -> ApiError {
        ApiError {
            code: self.code_str().to_string(),
            message: self.default_message().to_string(),
        }
    }

    /// 创建 ApiError 实例（使用自定义消息）
    pub fn to_api_error_with_message(&self, message: impl Into<String>) -> ApiError {
        ApiError {
            code: self.code_str().to_string(),
            message: message.into(),
        }
    }
}

impl<T> ApiResponse<T> {
    /// 成功响应
    pub fn success(data: T) -> Self {
        ApiResponse {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// 错误响应
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.into(),
                message: message.into(),
            }),
        }
    }

    /// 使用标准化错误码创建错误响应
    pub fn error_with_code(error_code: ErrorCode) -> Self {
        ApiResponse {
            success: false,
            data: None,
            error: Some(error_code.to_api_error()),
        }
    }

    /// 使用标准化错误码和自定义消息创建错误响应
    pub fn error_with_code_and_message(
        error_code: ErrorCode,
        message: impl Into<String>,
    ) -> Self {
        ApiResponse {
            success: false,
            data: None,
            error: Some(error_code.to_api_error_with_message(message)),
        }
    }
}

/// 用户创建参数（用于数据库插入）
pub struct CreateUserParams {
    pub username: String,
    pub email: String,
    pub password_hash: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    // === ApiResponse 测试 ===

    #[test]
    fn test_api_response_success() {
        let resp = ApiResponse::success("hello");
        assert!(resp.success);
        assert_eq!(resp.data, Some("hello"));
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let resp: ApiResponse<()> = ApiResponse::error("ERR_CODE", "error message");
        assert!(!resp.success);
        assert!(resp.data.is_none());
        assert!(resp.error.is_some());
        let err = resp.error.unwrap();
        assert_eq!(err.code, "ERR_CODE");
        assert_eq!(err.message, "error message");
    }

    #[test]
    fn test_api_response_success_serialization() {
        let resp = ApiResponse::success(serde_json::json!({"key": "value"}));
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("key"));
        assert!(!json.contains("error")); // skip_serializing_if = None
    }

    #[test]
    fn test_api_response_error_serialization() {
        let resp: ApiResponse<()> = ApiResponse::error("CODE", "msg");
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("CODE"));
        assert!(json.contains("msg"));
        assert!(!json.contains("data")); // skip_serializing_if = None
    }

    // === User 测试 ===

    #[test]
    fn test_user_serialization() {
        let user = User {
            id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            username: "alice".to_string(),
            email: "alice@example.com".to_string(),
            avatar: None,
            nickname: None,
            bio: None,
            status_message: None,
            created_at: "2026-05-13T00:00:00Z".to_string(),
            updated_at: "2026-05-13T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("\"username\""));
        assert!(json.contains("alice"));
        assert!(!json.contains("\"avatar\"")); // skip_serializing_if = None
    }

    #[test]
    fn test_user_with_avatar_serialization() {
        let user = User {
            id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            username: "bob".to_string(),
            email: "bob@example.com".to_string(),
            avatar: Some("https://example.com/avatar.png".to_string()),
            nickname: Some("Bob".to_string()),
            bio: Some("A developer".to_string()),
            status_message: Some("Available".to_string()),
            created_at: "2026-05-13T00:00:00Z".to_string(),
            updated_at: "2026-05-13T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("avatar"));
    }

    // === RegisterRequest 验证测试 ===

    #[test]
    fn test_register_request_valid() {
        let req = RegisterRequest {
            username: "alice".to_string(),
            email: "alice@example.com".to_string(),
            password: "password123".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_register_request_short_username() {
        let req = RegisterRequest {
            username: "ab".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_register_request_invalid_email() {
        let req = RegisterRequest {
            username: "alice".to_string(),
            email: "not-an-email".to_string(),
            password: "password123".to_string(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_register_request_short_password() {
        let req = RegisterRequest {
            username: "alice".to_string(),
            email: "test@example.com".to_string(),
            password: "short".to_string(),
        };
        assert!(req.validate().is_err());
    }

    // === LoginRequest 验证测试 ===

    #[test]
    fn test_login_request_valid() {
        let req = LoginRequest {
            email: "test@example.com".to_string(),
            password: "anypassword".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_login_request_empty_password() {
        let req = LoginRequest {
            email: "test@example.com".to_string(),
            password: "".to_string(),
        };
        assert!(req.validate().is_err());
    }

    // === LoginResponse 测试 ===

    #[test]
    fn test_login_response_serialization() {
        let user = User {
            id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            username: "alice".to_string(),
            email: "alice@example.com".to_string(),
            avatar: None,
            nickname: None,
            bio: None,
            status_message: None,
            created_at: "2026-05-13T00:00:00Z".to_string(),
            updated_at: "2026-05-13T00:00:00Z".to_string(),
        };

        let resp = LoginResponse {
            token: "jwt_token_here".to_string(),
            user,
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("jwt_token_here"));
        assert!(json.contains("alice"));
    }

    // === Claims 测试 ===

    #[test]
    fn test_claims_serialization() {
        let claims = Claims {
            sub: "user123".to_string(),
            email: "test@example.com".to_string(),
            exp: 1700000000,
            iat: 1699996400,
        };

        let json = serde_json::to_string(&claims).unwrap();
        assert!(json.contains("user123"));
        assert!(json.contains("test@example.com"));
    }

    #[test]
    fn test_claims_deserialization() {
        let json = r#"{
            "sub": "user456",
            "email": "bob@example.com",
            "exp": 1700000000,
            "iat": 1699996400
        }"#;

        let claims: Claims = serde_json::from_str(json).unwrap();
        assert_eq!(claims.sub, "user456");
        assert_eq!(claims.email, "bob@example.com");
    }

    // === UpdateUserRequest 测试 ===

    #[test]
    fn test_update_user_request_partial() {
        let req = UpdateUserRequest {
            username: Some("newname".to_string()),
            email: None,
            avatar: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_update_user_request_empty() {
        let req = UpdateUserRequest {
            username: None,
            email: None,
            avatar: None,
        };
        assert!(req.validate().is_ok()); // all optional, no validation needed
    }
}

// ==================== 用户屏蔽模型 ====================

/// 屏蔽用户请求
#[derive(Debug, Clone, Deserialize, Validate, utoipa::ToSchema)]
pub struct BlockUserRequest {
    /// 被屏蔽的用户ID
    #[serde(rename = "blockedUserId")]
    pub blocked_user_id: String,
}

/// 用户屏蔽记录
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BlockRecord {
    /// 屏蔽记录ID
    #[serde(rename = "id")]
    pub id: String,
    /// 屏蔽者用户ID
    #[serde(rename = "blockerId")]
    pub blocker_id: String,
    /// 被屏蔽者用户ID
    #[serde(rename = "blockedId")]
    pub blocked_id: String,
    /// 被屏蔽者用户名（冗余字段，方便查询）
    #[serde(rename = "blockedUsername", skip_serializing_if = "Option::is_none")]
    pub blocked_username: Option<String>,
    /// 被屏蔽者头像（冗余字段）
    #[serde(rename = "blockedAvatar", skip_serializing_if = "Option::is_none")]
    pub blocked_avatar: Option<String>,
    /// 屏蔽时间
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

/// 屏蔽列表响应
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct BlockListResponse {
    #[serde(rename = "blocks")]
    pub blocks: Vec<BlockRecord>,
    #[serde(rename = "total")]
    pub total: i64,
}

/// 检查是否被屏蔽的响应
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct BlockStatusResponse {
    /// 是否被该用户屏蔽
    #[serde(rename = "isBlocked")]
    pub is_blocked: bool,
    /// 是否屏蔽了该用户
    #[serde(rename = "hasBlocked")]
    pub has_blocked: bool,
}

/// 联系人实体（数据库模型）
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ContactEntity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub contact_id: Uuid,
    pub nickname: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 联系人 API 响应
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Contact {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "contactId")]
    pub contact_id: String,
    #[serde(rename = "nickname", skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

impl ContactEntity {
    pub fn to_contact(&self) -> Contact {
        Contact {
            id: self.id.to_string(),
            user_id: self.user_id.to_string(),
            contact_id: self.contact_id.to_string(),
            nickname: self.nickname.clone(),
            created_at: self.created_at.to_rfc3339(),
            updated_at: self.updated_at.to_rfc3339(),
        }
    }
}

/// 添加联系人请求
#[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
pub struct AddContactRequest {
    /// 联系人用户ID
    #[serde(rename = "contactId")]
    pub contact_id: String,
    /// 联系人备注名（可选）
    #[serde(rename = "nickname", skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 100, message = "备注名长度不能超过100字符"))]
    pub nickname: Option<String>,
}

/// 更新联系人备注请求
#[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
pub struct UpdateContactRequest {
    /// 新的备注名
    #[serde(rename = "nickname")]
    #[validate(length(max = 100, message = "备注名长度不能超过100字符"))]
    pub nickname: String,
}

/// 联系人列表响应
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ContactListResponse {
    #[serde(rename = "contacts")]
    pub contacts: Vec<Contact>,
    #[serde(rename = "total")]
    pub total: i64,
}

/// 用户搜索结果
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct UserSearchResult {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "username")]
    pub username: String,
    #[serde(rename = "nickname", skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    #[serde(rename = "avatar", skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(rename = "bio", skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(rename = "isContact")]
    pub is_contact: bool,
}
