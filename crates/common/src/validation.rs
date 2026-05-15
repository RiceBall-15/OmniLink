//! 输入验证工具
//!
//! 提供统一的输入验证函数，防止 SQL 注入、XSS 攻击等安全问题。

use regex::Regex;
use once_cell::sync::Lazy;
use thiserror::Error;

/// 验证错误类型
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("输入包含非法字符: {0}")]
    IllegalCharacters(String),
    #[error("输入长度超限: 最大 {max}，实际 {actual}")]
    TooLong { max: usize, actual: usize },
    #[error("输入为空")]
    Empty,
    #[error("格式不正确: {0}")]
    InvalidFormat(String),
    #[error("包含潜在的 SQL 注入内容")]
    SqlInjection,
    #[error("包含潜在的 XSS 攻击内容")]
    XssAttack,
}

/// SQL 注入检测模式
static SQL_INJECTION_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)(union\s+select)").unwrap(),
        Regex::new(r"(?i)(drop\s+table)").unwrap(),
        Regex::new(r"(?i)(delete\s+from)").unwrap(),
        Regex::new(r"(?i)(insert\s+into)").unwrap(),
        Regex::new(r"(?i)(update\s+\w+\s+set)").unwrap(),
        Regex::new(r"(?i)(--\s*$)").unwrap(),
        Regex::new(r"(?i)(/\*.*\*/)").unwrap(),
        Regex::new(r"(?i)('\s*or\s+')").unwrap(),
        Regex::new(r#"(?i)(;\s*drop\s)"#).unwrap(),
        Regex::new(r"(?i)(exec\s*\()").unwrap(),
    ]
});

/// XSS 攻击检测模式
static XSS_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)(<script[^>]*>)").unwrap(),
        Regex::new(r"(?i)(javascript\s*:)").unwrap(),
        Regex::new(r"(?i)(on\w+\s*=\s*['""])").unwrap(),
        Regex::new(r"(?i)(<iframe[^>]*>)").unwrap(),
        Regex::new(r"(?i)(<object[^>]*>)").unwrap(),
        Regex::new(r"(?i)(<embed[^>]*>)").unwrap(),
    ]
});

/// 验证字符串输入
///
/// 检查内容是否包含 SQL 注入或 XSS 攻击模式
pub fn validate_string_input(input: &str, max_length: usize, field_name: &str) -> Result<(), ValidationError> {
    // 检查空输入
    if input.trim().is_empty() {
        return Err(ValidationError::Empty);
    }

    // 检查长度
    if input.len() > max_length {
        return Err(ValidationError::TooLong {
            max: max_length,
            actual: input.len(),
        });
    }

    // 检查 SQL 注入
    for pattern in SQL_INJECTION_PATTERNS.iter() {
        if pattern.is_match(input) {
            tracing::warn!(
                field = field_name,
                input = %input.chars().take(50).collect::<String>(),
                "检测到潜在的 SQL 注入攻击"
            );
            return Err(ValidationError::SqlInjection);
        }
    }

    // 检查 XSS
    for pattern in XSS_PATTERNS.iter() {
        if pattern.is_match(input) {
            tracing::warn!(
                field = field_name,
                input = %input.chars().take(50).collect::<String>(),
                "检测到潜在的 XSS 攻击"
            );
            return Err(ValidationError::XssAttack);
        }
    }

    Ok(())
}

/// 验证用户名
///
/// 规则：
/// - 长度 3-50
/// - 只允许字母、数字、下划线、连字符
pub fn validate_username(username: &str) -> Result<(), ValidationError> {
    if username.len() < 3 || username.len() > 50 {
        return Err(ValidationError::TooLong {
            max: 50,
            actual: username.len(),
        });
    }

    let re = Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
    if !re.is_match(username) {
        return Err(ValidationError::IllegalCharacters(
            "用户名只允许字母、数字、下划线和连字符".to_string()
        ));
    }

    Ok(())
}

/// 验证邮箱格式
pub fn validate_email(email: &str) -> Result<(), ValidationError> {
    let re = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
    if !re.is_match(email) {
        return Err(ValidationError::InvalidFormat("邮箱格式不正确".to_string()));
    }
    Ok(())
}

/// 验证 UUID 格式
pub fn validate_uuid(uuid_str: &str) -> Result<(), ValidationError> {
    let re = Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap();
    if !re.is_match(uuid_str) {
        return Err(ValidationError::InvalidFormat("UUID 格式不正确".to_string()));
    }
    Ok(())
}

/// 验证消息内容
pub fn validate_message_content(content: &str) -> Result<(), ValidationError> {
    validate_string_input(content, 10000, "message_content")
}

/// 验证会话名称
pub fn validate_conversation_name(name: &str) -> Result<(), ValidationError> {
    validate_string_input(name, 100, "conversation_name")
}

/// 验证密码强度
///
/// 规则：
/// - 最少 8 位
/// - 包含大小写字母和数字
pub fn validate_password(password: &str) -> Result<(), ValidationError> {
    if password.len() < 8 {
        return Err(ValidationError::TooLong {
            max: 128,
            actual: password.len(),
        });
    }

    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_numeric());

    if !has_uppercase || !has_lowercase || !has_digit {
        return Err(ValidationError::InvalidFormat(
            "密码必须包含大小写字母和数字".to_string()
        ));
    }

    Ok(())
}

/// 清理用户输入（移除潜在危险字符）
pub fn sanitize_input(input: &str) -> String {
    input
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
        .replace('/', "&#x2F;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_normal_input() {
        assert!(validate_string_input("正常的消息内容", 100, "test").is_ok());
    }

    #[test]
    fn test_validate_empty_input() {
        assert!(validate_string_input("", 100, "test").is_err());
        assert!(validate_string_input("   ", 100, "test").is_err());
    }

    #[test]
    fn test_validate_too_long() {
        let long_input = "a".repeat(101);
        assert!(validate_string_input(&long_input, 100, "test").is_err());
    }

    #[test]
    fn test_detect_sql_injection() {
        assert!(validate_string_input("'; DROP TABLE users; --", 100, "test").is_err());
        assert!(validate_string_input("1' OR '1'='1", 100, "test").is_err());
        assert!(validate_string_input("UNION SELECT * FROM users", 100, "test").is_err());
    }

    #[test]
    fn test_detect_xss() {
        assert!(validate_string_input("<script>alert('xss')</script>", 100, "test").is_err());
        assert!(validate_string_input("javascript:alert(1)", 100, "test").is_err());
    }

    #[test]
    fn test_validate_username() {
        assert!(validate_username("valid_user").is_ok());
        assert!(validate_username("ab").is_err()); // 太短
        assert!(validate_username("user@name").is_err()); // 非法字符
    }

    #[test]
    fn test_validate_email() {
        assert!(validate_email("test@example.com").is_ok());
        assert!(validate_email("invalid-email").is_err());
    }

    #[test]
    fn test_validate_uuid() {
        assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(validate_uuid("invalid-uuid").is_err());
    }

    #[test]
    fn test_validate_password() {
        assert!(validate_password("StrongPass1").is_ok());
        assert!(validate_password("weak").is_err()); // 太短
        assert!(validate_password("alllowercase1").is_err()); // 没有大写
        assert!(validate_password("ALLUPPERCASE1").is_err()); // 没有小写
    }

    #[test]
    fn test_sanitize_input() {
        let input = "<script>alert('xss')</script>";
        let sanitized = sanitize_input(input);
        assert!(!sanitized.contains('<'));
        assert!(!sanitized.contains('>'));
    }
}
