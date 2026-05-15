//! API 响应压缩中间件集成测试
//!
//! 测试 gzip 和 brotli 压缩效果，验证压缩配置选项。

use axum::{routing::get, Router};
use flate2::read::GzDecoder;
use http_body_util::BodyExt;
use std::io::Read;
use tower::ServiceExt;

/// 创建一个返回大 JSON 响应的测试路由
fn test_app() -> Router {
    Router::new().route("/api/test", get(|| async {
        let data = serde_json::json!({
            "status": "success",
            "data": {
                "users": (0..100).map(|i| {
                    serde_json::json!({
                        "id": i,
                        "name": format!("User {}", i),
                        "email": format!("user{}@example.com", i),
                        "bio": "A".repeat(50),
                    })
                }).collect::<Vec<_>>(),
                "total": 100,
                "page": 1,
                "per_page": 100,
            },
            "message": "Operation completed successfully",
            "timestamp": "2026-05-16T04:30:00Z",
        });
        axum::Json(data)
    }))
}

/// 创建带压缩中间件的测试应用
fn test_app_with_compression() -> Router {
    use im_api::middleware::compression::create_compression_layer;
    test_app().layer(create_compression_layer())
}

#[tokio::test]
async fn test_gzip_compression() {
    let app = test_app_with_compression();

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/test")
                .header("Accept-Encoding", "gzip")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let encoding = response
        .headers()
        .get("content-encoding")
        .map(|v| v.to_str().unwrap());
    assert_eq!(encoding, Some("gzip"), "Expected gzip content-encoding");

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let mut decoder = GzDecoder::new(&body[..]);
    let mut decompressed = String::new();
    decoder.read_to_string(&mut decompressed).unwrap();

    let json: serde_json::Value = serde_json::from_str(&decompressed).unwrap();
    assert_eq!(json["status"], "success");
    assert!(json["data"]["users"].as_array().unwrap().len() == 100);
}

#[tokio::test]
async fn test_brotli_compression() {
    let app = test_app_with_compression();

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/test")
                .header("Accept-Encoding", "br")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let encoding = response
        .headers()
        .get("content-encoding")
        .map(|v| v.to_str().unwrap());
    assert_eq!(encoding, Some("br"), "Expected brotli content-encoding");

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let mut output = Vec::new();
    brotli::BrotliDecompress(&mut &body[..], &mut output).unwrap();
    let decompressed = String::from_utf8(output).unwrap();

    let json: serde_json::Value = serde_json::from_str(&decompressed).unwrap();
    assert_eq!(json["status"], "success");
    assert!(json["data"]["users"].as_array().unwrap().len() == 100);
}

#[tokio::test]
async fn test_no_compression_without_accept_encoding() {
    let app = test_app_with_compression();

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/test")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let encoding = response
        .headers()
        .get("content-encoding")
        .map(|v| v.to_str().unwrap());
    assert!(
        encoding.is_none(),
        "Should not compress without Accept-Encoding header"
    );
}

#[tokio::test]
async fn test_compression_reduces_size() {
    // Get uncompressed response
    let app1 = test_app_with_compression();
    let response_uncompressed = app1
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/test")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body_uncompressed = response_uncompressed
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();

    // Get gzip compressed response
    let app2 = test_app_with_compression();
    let response_gzip = app2
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/test")
                .header("Accept-Encoding", "gzip")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body_gzip = response_gzip
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();

    assert!(
        body_gzip.len() < body_uncompressed.len(),
        "Gzip compressed ({}) should be smaller than uncompressed ({})",
        body_gzip.len(),
        body_uncompressed.len()
    );

    let ratio = body_gzip.len() as f64 / body_uncompressed.len() as f64;
    assert!(
        ratio < 0.7,
        "Compression ratio ({:.2}%) should be less than 70%",
        ratio * 100.0
    );
}

#[tokio::test]
async fn test_compression_config_defaults() {
    use im_api::middleware::compression::CompressionConfig;

    let config = CompressionConfig::default();
    assert!(config.enabled);
    assert_eq!(config.min_size, 1024);
    assert_eq!(config.gzip_quality, 6);
    assert_eq!(config.br_quality, 4);
}

#[tokio::test]
async fn test_compression_with_wildcard_encoding() {
    let app = test_app_with_compression();

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/test")
                .header("Accept-Encoding", "*")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let encoding = response
        .headers()
        .get("content-encoding")
        .map(|v| v.to_str().unwrap());
    assert!(
        encoding.is_some(),
        "Wildcard Accept-Encoding should trigger compression"
    );
}

#[tokio::test]
async fn test_compression_preserves_json_content_type() {
    let app = test_app_with_compression();

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/test")
                .header("Accept-Encoding", "gzip")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let content_type = response
        .headers()
        .get("content-type")
        .map(|v: &axum::http::HeaderValue| v.to_str().unwrap())
        .unwrap_or("");
    assert!(
        content_type.contains("application/json"),
        "Content-Type should be application/json, got: {}",
        content_type
    );
}
