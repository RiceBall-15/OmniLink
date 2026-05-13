//! 安全头中间件
//!
//! 为所有HTTP响应添加安全相关的HTTP头，防止常见的Web攻击。

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

/// 安全头中间件
///
/// 为所有响应添加以下安全头：
/// - X-Content-Type-Options: nosniff（防止MIME类型嗅探）
/// - X-Frame-Options: DENY（防止点击劫持）
/// - X-XSS-Protection: 1; mode=block（XSS过滤）
/// - Strict-Transport-Security（HSTS，强制HTTPS）
/// - Content-Security-Policy（CSP，限制资源加载）
/// - Referrer-Policy（控制Referer头）
/// - Permissions-Policy（控制浏览器功能）
pub async fn security_headers_middleware(
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    // 防止MIME类型嗅探
    headers.insert(
        "x-content-type-options",
        "nosniff".parse().unwrap(),
    );

    // 防止点击劫持
    headers.insert(
        "x-frame-options",
        "DENY".parse().unwrap(),
    );

    // XSS保护
    headers.insert(
        "x-xss-protection",
        "1; mode=block".parse().unwrap(),
    );

    // HSTS - 强制HTTPS（1年有效期）
    headers.insert(
        "strict-transport-security",
        "max-age=31536000; includeSubDomains".parse().unwrap(),
    );

    // 内容安全策略
    // 限制资源加载来源，防止XSS和数据注入攻击
    headers.insert(
        "content-security-policy",
        "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; connect-src 'self' ws: wss:; frame-ancestors 'none'; base-uri 'self'; form-action 'self'".parse().unwrap(),
    );

    // 控制Referer头
    headers.insert(
        "referrer-policy",
        "strict-origin-when-cross-origin".parse().unwrap(),
    );

    // 控制浏览器功能
    headers.insert(
        "permissions-policy",
        "camera=(), microphone=(), geolocation=(), payment=()".parse().unwrap(),
    );

    response
}
