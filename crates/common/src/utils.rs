use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

/// 生成UUID
pub fn generate_uuid() -> Uuid {
    Uuid::new_v4()
}

/// 验证邮箱格式
pub fn validate_email(email: &str) -> bool {
    // 简单的邮箱验证
    email.contains('@') && email.contains('.')
}

/// 截断字符串
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// 生成短ID（基于时间戳和随机数）
pub fn generate_short_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let random_part = rand::random::<u32>();
    format!("{:x}{:08x}", timestamp, random_part)
}

/// 清理文件名，移除不安全字符
pub fn sanitize_filename(filename: &str) -> String {
    let unsafe_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|', '\0'];
    filename
        .chars()
        .map(|c| if unsafe_chars.contains(&c) { '_' } else { c })
        .collect::<String>()
        .trim()
        .to_string()
}

/// 格式化时间戳为可读字符串
pub fn format_timestamp(timestamp_millis: u64) -> String {
    let datetime = chrono::DateTime::from_timestamp(
        (timestamp_millis / 1000) as i64,
        ((timestamp_millis % 1000) * 1_000_000) as u32,
    );
    match datetime {
        Some(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        None => "Invalid timestamp".to_string(),
    }
}

/// 检查字符串是否为空或仅包含空白
pub fn is_blank(s: &str) -> bool {
    s.trim().is_empty()
}

/// 截断字符串到指定字节数（UTF-8安全）
pub fn truncate_utf8(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        s.to_string()
    } else {
        let mut end = max_bytes;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &s[..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_uuid() {
        let uuid1 = generate_uuid();
        let uuid2 = generate_uuid();
        assert_ne!(uuid1, uuid2);
    }

    #[test]
    fn test_validate_email() {
        assert!(validate_email("test@example.com"));
        assert!(!validate_email("invalid"));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
    }

    #[test]
    fn test_validate_email_edge_cases() {
        assert!(validate_email("a@b.c"));
        assert!(validate_email("user@domain.com"));
        assert!(!validate_email(""));
        assert!(!validate_email("noat"));
        assert!(!validate_email("nodot@"));
        // Note: simple validation only checks for '@' and '.', so "@no.local" passes
        assert!(validate_email("@no.local"));
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("hi", 100), "hi");
    }

    #[test]
    fn test_truncate_empty_string() {
        assert_eq!(truncate("", 5), "");
    }

    #[test]
    fn test_generate_short_id() {
        let id1 = generate_short_id();
        let id2 = generate_short_id();
        assert_ne!(id1, id2);
        assert!(!id1.is_empty());
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("test.txt"), "test.txt");
        assert_eq!(sanitize_filename("test/file.txt"), "test_file.txt");
        assert_eq!(sanitize_filename("test\\file.txt"), "test_file.txt");
        assert_eq!(sanitize_filename("test:file.txt"), "test_file.txt");
        assert_eq!(sanitize_filename("test*file.txt"), "test_file.txt");
        assert_eq!(sanitize_filename("test?file.txt"), "test_file.txt");
        assert_eq!(sanitize_filename("test\"file.txt"), "test_file.txt");
        assert_eq!(sanitize_filename("test<file.txt"), "test_file.txt");
        assert_eq!(sanitize_filename("test>file.txt"), "test_file.txt");
        assert_eq!(sanitize_filename("test|file.txt"), "test_file.txt");
    }

    #[test]
    fn test_is_blank() {
        assert!(is_blank(""));
        assert!(is_blank("   "));
        assert!(is_blank("\t\n"));
        assert!(!is_blank("hello"));
        assert!(!is_blank(" hello "));
    }

    #[test]
    fn test_truncate_utf8() {
        // ASCII string
        assert_eq!(truncate_utf8("hello", 10), "hello");
        assert_eq!(truncate_utf8("hello world", 8), "hello wo...");

        // UTF-8 string with multi-byte characters
        assert_eq!(truncate_utf8("你好世界", 10), "你好世...");
        assert_eq!(truncate_utf8("hello", 5), "hello");
    }
}