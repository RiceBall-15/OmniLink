//! ETag 中间件 - HTTP 缓存验证
//!
//! 自动为 JSON 响应生成 ETag 头，支持 If-None-Match 条件请求。
//! 当客户端提供的 ETag 与当前资源匹配时，返回 304 Not Modified，
//! 减少不必要的数据传输。

use axum::{
    body::Body,
    http::{Request, Response, StatusCode, header},
    middleware::Next,
};
use sha2::{Sha256, Digest};

/// ETag 中间件
///
/// 为 GET 请求的 JSON 响应自动生成 ETag，并处理 If-None-Match 条件请求。
pub async fn etag_middleware(
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    // 只处理 GET 请求
    if req.method() != axum::http::Method::GET {
        return next.run(req).await;
    }

    // 获取 If-None-Match 头
    let if_none_match = req
        .headers()
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let response = next.run(req).await;

    // 只处理成功响应（2xx）且 Content-Type 为 JSON
    let status = response.status();
    if !status.is_success() {
        return response;
    }

    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !content_type.contains("application/json") {
        return response;
    }

    // 提取响应体
    let (parts, body) = response.into_parts();
    let body_bytes = match axum::body::to_bytes(body, 1024 * 1024).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return Response::from_parts(parts, Body::empty());
        }
    };

    // 计算 ETag（SHA256 哈希的前16字符）
    let mut hasher = Sha256::new();
    hasher.update(&body_bytes);
    let hash = format!("{:x}", hasher.finalize());
    let etag = format!("\"{}\"", &hash[..16]);

    // 检查 If-None-Match
    if let Some(client_etag) = if_none_match {
        if client_etag == etag {
            // ETag 匹配，返回 304 Not Modified
            let mut not_modified = Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .body(Body::empty())
                .unwrap();

            // 复制原始响应头
            for (key, value) in parts.headers.iter() {
                if key != header::CONTENT_TYPE
                    && key != header::CONTENT_LENGTH
                {
                    not_modified.headers_mut().insert(key, value.clone());
                }
            }
            not_modified.headers_mut().insert(
                header::ETAG,
                etag.parse().unwrap(),
            );

            return not_modified;
        }
    }

    // 添加 ETag 头到响应
    let mut response = Response::from_parts(parts, Body::from(body_bytes));
    response.headers_mut().insert(
        header::ETAG,
        etag.parse().unwrap(),
    );

    // 添加 Cache-Control 头（允许缓存5秒）
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        "private, max-age=5".parse().unwrap(),
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_etag_format() {
        // 验证 ETag 格式
        let mut hasher = Sha256::new();
        hasher.update(b"test content");
        let hash = format!("{:x}", hasher.finalize());
        let etag = format!("\"{}\"", &hash[..16]);

        assert!(etag.starts_with('"'));
        assert!(etag.ends_with('"'));
        assert_eq!(etag.len(), 18); // " + 16 chars + "
    }
}
