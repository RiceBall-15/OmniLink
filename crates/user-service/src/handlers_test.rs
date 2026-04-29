#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_request_validation() {
        use crate::models::RegisterRequest;
        use validator::Validate;

        // 测试有效请求
        let req = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            avatar_url: None,
            bio: None,
        };
        assert!(req.validate().is_ok());

        // 测试无效用户名（太短）
        let req = RegisterRequest {
            username: "ab".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            avatar_url: None,
            bio: None,
        };
        assert!(req.validate().is_err());

        // 测试无效邮箱
        let req = RegisterRequest {
            username: "testuser".to_string(),
            email: "invalid-email".to_string(),
            password: "password123".to_string(),
            avatar_url: None,
            bio: None,
        };
        assert!(req.validate().is_err());

        // 测试无效密码（太短）
        let req = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "short".to_string(),
            avatar_url: None,
            bio: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_login_request_validation() {
        use crate::models::LoginRequest;
        use validator::Validate;

        // 测试有效请求
        let req = LoginRequest {
            email_or_username: "test@example.com".to_string(),
            password: "password123".to_string(),
            device_id: "device-123".to_string(),
            device_name: None,
        };
        assert!(req.validate().is_ok());

        // 测试空的邮箱或用户名
        let req = LoginRequest {
            email_or_username: "".to_string(),
            password: "password123".to_string(),
            device_id: "device-123".to_string(),
            device_name: None,
        };
        assert!(req.validate().is_err());

        // 测试空的密码
        let req = LoginRequest {
            email_or_username: "test@example.com".to_string(),
            password: "".to_string(),
            device_id: "device-123".to_string(),
            device_name: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_update_profile_request_validation() {
        use crate::models::UpdateProfileRequest;
        use validator::Validate;

        // 测试有效请求（所有字段可选）
        let req = UpdateProfileRequest {
            username: None,
            email: None,
            avatar_url: None,
            bio: None,
        };
        assert!(req.validate().is_ok());

        // 测试有效请求（有字段）
        let req = UpdateProfileRequest {
            username: Some("newusername".to_string()),
            email: Some("new@example.com".to_string()),
            avatar_url: Some("http://example.com/avatar.jpg".to_string()),
            bio: Some("Test bio".to_string()),
        };
        assert!(req.validate().is_ok());

        // 测试无效邮箱
        let req = UpdateProfileRequest {
            username: None,
            email: Some("invalid-email".to_string()),
            avatar_url: None,
            bio: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_change_password_request_validation() {
        use crate::models::ChangePasswordRequest;
        use validator::Validate;

        // 测试有效请求
        let req = ChangePasswordRequest {
            old_password: "oldpassword".to_string(),
            new_password: "newpassword123".to_string(),
        };
        assert!(req.validate().is_ok());

        // 测试空的旧密码
        let req = ChangePasswordRequest {
            old_password: "".to_string(),
            new_password: "newpassword123".to_string(),
        };
        assert!(req.validate().is_err());

        // 测试无效的新密码（太短）
        let req = ChangePasswordRequest {
            old_password: "oldpassword".to_string(),
            new_password: "short".to_string(),
        };
        assert!(req.validate().is_err());
    }
}