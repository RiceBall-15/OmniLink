//! Security module
//!
//! Provides input validation, XSS protection, and sensitive data handling.

use regex::Regex;
use once_cell::sync::Lazy;

/// XSS attack pattern detection regex
static XSS_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(<script[^>]*>.*?</script>|javascript:|on\w+\s*=|<iframe|<object|<embed|<form)").unwrap()
});

/// SQL injection attack pattern detection regex
static SQL_INJECTION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(union\s+select|drop\s+table|delete\s+from|insert\s+into|update\s+\w+\s+set|--|;|'|\b(or|and)\b\s+\d+\s*=\s*\d+)").unwrap()
});

/// Detect XSS attacks
pub fn detect_xss(input: &str) -> bool {
    XSS_PATTERN.is_match(input)
}

/// Sanitize XSS content
pub fn sanitize_xss(input: &str) -> String {
    input
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
        .replace('/', "&#x2F;")
}

/// Detect SQL injection attacks
pub fn detect_sql_injection(input: &str) -> bool {
    SQL_INJECTION_PATTERN.is_match(input)
}

/// Validate email format
pub fn validate_email(email: &str) -> bool {
    static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap()
    });
    EMAIL_REGEX.is_match(email)
}

/// Validate password strength
///
/// Requirements:
/// - At least 8 characters
/// - At least one uppercase letter
/// - At least one lowercase letter
/// - At least one digit
pub fn validate_password_strength(password: &str) -> bool {
    if password.len() < 8 {
        return false;
    }

    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_numeric());

    has_uppercase && has_lowercase && has_digit
}

/// Mask phone number
///
/// Replace middle 4 digits with asterisks
pub fn mask_phone_number(phone: &str) -> String {
    if phone.len() >= 11 {
        format!("{}****{}", &phone[..3], &phone[phone.len()-4..])
    } else {
        phone.to_string()
    }
}

/// Mask email address
///
/// Replace part before @ with asterisks
pub fn mask_email(email: &str) -> String {
    if let Some(at_pos) = email.find('@') {
        let prefix_len = at_pos.min(3);
        let masked_prefix = format!("{}***", &email[..prefix_len]);
        format!("{}@{}", masked_prefix, &email[at_pos+1..])
    } else {
        email.to_string()
    }
}

/// Validate username format
///
/// Requirements:
/// - 3-32 characters
/// - Only alphanumeric, underscore, hyphen
/// - Must start with letter
pub fn validate_username(username: &str) -> bool {
    static USERNAME_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]{2,31}$").unwrap()
    });
    USERNAME_REGEX.is_match(username)
}

/// Sanitize user input
///
/// Remove potentially dangerous characters while preserving normal text
pub fn sanitize_input(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_xss() {
        assert!(detect_xss("<script>alert('xss')</script>"));
        assert!(detect_xss("<img src=x onerror=alert(1)>"));
        assert!(detect_xss("javascript:alert(1)"));
        assert!(!detect_xss("Hello, World!"));
        assert!(!detect_xss("normal text"));
    }

    #[test]
    fn test_sanitize_xss() {
        let input = "<script>alert('xss')</script>";
        let sanitized = sanitize_xss(input);
        assert!(!sanitized.contains('<'));
        assert!(!sanitized.contains('>'));
    }

    #[test]
    fn test_detect_sql_injection() {
        assert!(detect_sql_injection("1' OR '1'='1"));
        assert!(detect_sql_injection("1; DROP TABLE users"));
        assert!(detect_sql_injection("1 UNION SELECT * FROM users"));
        assert!(!detect_sql_injection("normal query"));
        assert!(!detect_sql_injection("hello world"));
    }

    #[test]
    fn test_validate_email() {
        assert!(validate_email("test@example.com"));
        assert!(validate_email("user.name@domain.co"));
        assert!(!validate_email("invalid"));
        assert!(!validate_email("@domain.com"));
        assert!(!validate_email("user@"));
    }

    #[test]
    fn test_validate_password_strength() {
        assert!(validate_password_strength("Password123"));
        assert!(validate_password_strength("MyStr0ngPass"));
        assert!(!validate_password_strength("weak"));
        assert!(!validate_password_strength("nouppercase1"));
        assert!(!validate_password_strength("NOLOWERCASE1"));
        assert!(!validate_password_strength("NoDigitsHere"));
    }

    #[test]
    fn test_mask_phone_number() {
        assert_eq!(mask_phone_number("13812345678"), "138****5678");
        assert_eq!(mask_phone_number("13900001111"), "139****1111");
        assert_eq!(mask_phone_number("short"), "short");
    }

    #[test]
    fn test_mask_email() {
        assert_eq!(mask_email("test@example.com"), "tes***@example.com");
        assert_eq!(mask_email("ab@domain.com"), "ab***@domain.com");
        assert_eq!(mask_email("verylongname@domain.com"), "ver***@domain.com");
    }

    #[test]
    fn test_validate_username() {
        assert!(validate_username("john_doe"));
        assert!(validate_username("user-123"));
        assert!(validate_username("Abc"));
        assert!(!validate_username("ab"));  // too short
        assert!(!validate_username("1user"));  // starts with digit
        assert!(!validate_username("user@name"));  // invalid char
    }

    #[test]
    fn test_sanitize_input() {
        assert_eq!(sanitize_input("hello\nworld"), "hello\nworld");
        assert_eq!(sanitize_input("normal text"), "normal text");
        // Control characters should be removed (except newline/tab)
        assert_eq!(sanitize_input("test\x00value"), "testvalue");
    }
}
