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
#[derive(Debug, Clone)]
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
