//! API 版本管理
//!
//! 提供多版本 API 支持：
//! - URL 路径版本（`/api/v1/`, `/api/v2/`）
//! - Accept header 版本协商（`Accept: application/vnd.omnilink.v1+json`）
//! - 版本废弃警告 header
//! - 版本路由构建辅助函数
//!
//! # 使用示例
//!
//! ```rust
//! use common::api_version::{ApiVersion, versioned_routes, deprecation_middleware};
//! use axum::{Router, routing::get, middleware};
//!
//! async fn handler() -> &'static str { "ok" }
//!
//! // 构建版本化路由
//! let app = Router::new()
//!     .merge(versioned_routes(ApiVersion::V1, Router::new()
//!         .route("/users", get(handler))
//!     ))
//!     .merge(versioned_routes(ApiVersion::V2, Router::new()
//!         .route("/users", get(handler))
//!     ));
//! ```

use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Router,
};
use std::fmt;

/// API 版本
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ApiVersion {
    /// 版本 1（初始版本）
    V1,
    /// 版本 2
    V2,
    /// 版本 3
    V3,
}

impl ApiVersion {
    /// 获取版本号数值
    pub fn number(&self) -> u8 {
        match self {
            ApiVersion::V1 => 1,
            ApiVersion::V2 => 2,
            ApiVersion::V3 => 3,
        }
    }

    /// 获取 URL 路径前缀
    pub fn path_prefix(&self) -> &'static str {
        match self {
            ApiVersion::V1 => "/api/v1",
            ApiVersion::V2 => "/api/v2",
            ApiVersion::V3 => "/api/v3",
        }
    }

    /// 从 URL 路径解析版本
    ///
    /// 支持格式：`/api/v1/...`, `/api/v2/...`
    pub fn from_path(path: &str) -> Option<Self> {
        if path.starts_with("/api/v1") {
            Some(ApiVersion::V1)
        } else if path.starts_with("/api/v2") {
            Some(ApiVersion::V2)
        } else if path.starts_with("/api/v3") {
            Some(ApiVersion::V3)
        } else {
            None
        }
    }

    /// 从 Accept header 解析版本
    ///
    /// 支持格式：`application/vnd.omnilink.v1+json`
    pub fn from_accept_header(value: &str) -> Option<Self> {
        if value.contains("vnd.omnilink.v1") {
            Some(ApiVersion::V1)
        } else if value.contains("vnd.omnilink.v2") {
            Some(ApiVersion::V2)
        } else if value.contains("vnd.omnilink.v3") {
            Some(ApiVersion::V3)
        } else {
            None
        }
    }

    /// 获取 Accept header 值
    pub fn accept_value(&self) -> &'static str {
        match self {
            ApiVersion::V1 => "application/vnd.omnilink.v1+json",
            ApiVersion::V2 => "application/vnd.omnilink.v2+json",
            ApiVersion::V3 => "application/vnd.omnilink.v3+json",
        }
    }

    /// 是否为当前最新版本
    pub fn is_latest(&self) -> bool {
        *self == ApiVersion::latest()
    }

    /// 获取最新版本
    pub fn latest() -> Self {
        ApiVersion::V3
    }

    /// 是否已废弃（V1 标记为废弃）
    pub fn is_deprecated(&self) -> bool {
        matches!(self, ApiVersion::V1)
    }

    /// 废弃日期（用于 Sunset header）
    pub fn sunset_date(&self) -> Option<&'static str> {
        match self {
            ApiVersion::V1 => Some("2026-12-31"),
            _ => None,
        }
    }
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.number())
    }
}

/// 版本信息，存储在请求扩展中
#[derive(Debug, Clone)]
pub struct ApiVersionInfo {
    /// 解析到的 API 版本
    pub version: ApiVersion,
    /// 版本来源
    pub source: VersionSource,
}

/// 版本信息来源
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionSource {
    /// URL 路径（`/api/v1/...`）
    Path,
    /// Accept header（`application/vnd.omnilink.v1+json`）
    AcceptHeader,
    /// 默认版本（未指定时）
    Default,
}

/// 构建版本化路由
///
/// 为路由添加 `/api/vN` 前缀，并在请求扩展中注入版本信息。
///
/// # 示例
///
/// ```rust
/// use common::api_version::{ApiVersion, versioned_routes};
/// use axum::{Router, routing::get};
///
/// async fn handler() -> &'static str { "ok" }
///
/// let v1_routes = versioned_routes(ApiVersion::V1, Router::new()
///     .route("/users", get(handler))
/// );
/// // 路由变为 /api/v1/users
/// ```
pub fn versioned_routes(version: ApiVersion, router: Router) -> Router {
    let version_middleware = {
        let version = version;
        move |mut req: Request, next: Next| {
            let version = version;
            async move {
                req.extensions_mut().insert(ApiVersionInfo {
                    version,
                    source: VersionSource::Path,
                });
                next.run(req).await
            }
        }
    };

    Router::new()
        .nest(version.path_prefix(), router)
        .layer(axum::middleware::from_fn(version_middleware))
}

/// API 版本解析中间件
///
/// 从 URL 路径或 Accept header 解析 API 版本，并注入到请求扩展中。
/// 如果未指定版本，使用默认版本（V1）。
pub async fn version_detection_middleware(
    mut req: Request,
    next: Next,
) -> Response {
    // 如果已经由 versioned_routes 设置了版本，跳过
    if req.extensions().get::<ApiVersionInfo>().is_some() {
        return next.run(req).await;
    }

    let path = req.uri().path().to_string();
    let accept = req
        .headers()
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    // 优先从 URL 路径解析
    let version_info = if let Some(version) = ApiVersion::from_path(&path) {
        ApiVersionInfo {
            version,
            source: VersionSource::Path,
        }
    }
    // 其次从 Accept header 解析
    else if let Some(version) = ApiVersion::from_accept_header(&accept) {
        ApiVersionInfo {
            version,
            source: VersionSource::AcceptHeader,
        }
    }
    // 默认版本
    else {
        ApiVersionInfo {
            version: ApiVersion::V1,
            source: VersionSource::Default,
        }
    };

    req.extensions_mut().insert(version_info);
    next.run(req).await
}

/// 版本废弃警告中间件
///
/// 为已废弃的 API 版本添加警告 header：
/// - `Deprecation: true`
/// - `Sunset: <date>`
/// - `X-API-Warn: <message>`
pub async fn deprecation_middleware(
    req: Request,
    next: Next,
) -> Response {
    let version_info = req.extensions().get::<ApiVersionInfo>().cloned();

    let mut response = next.run(req).await;

    if let Some(info) = version_info {
        if info.version.is_deprecated() {
            let headers = response.headers_mut();

            headers.insert("Deprecation", "true".parse().unwrap());

            if let Some(sunset) = info.version.sunset_date() {
                if let Ok(value) = sunset.parse() {
                    headers.insert("Sunset", value);
                }
            }

            let warn_msg = format!(
                "API {} is deprecated. Please migrate to {}.",
                info.version,
                ApiVersion::latest()
            );
            if let Ok(value) = warn_msg.parse() {
                headers.insert("X-API-Warn", value);
            }

            tracing::info!(
                version = %info.version,
                source = ?info.source,
                "废弃 API 版本请求"
            );
        }
    }

    response
}

/// 版本不支持的错误响应
pub fn unsupported_version_response(version: &str) -> impl IntoResponse {
    let body = serde_json::json!({
        "error": "unsupported_api_version",
        "message": format!("API version '{}' is not supported", version),
        "supported_versions": ["v1", "v2"],
        "latest_version": ApiVersion::latest().to_string(),
    });

    (
        StatusCode::NOT_FOUND,
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
}

/// 从请求中提取 API 版本
///
/// 如果请求中没有版本信息，返回默认版本（V1）。
pub fn extract_version(req: &Request) -> ApiVersion {
    req.extensions()
        .get::<ApiVersionInfo>()
        .map(|info| info.version)
        .unwrap_or(ApiVersion::V1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_version_number() {
        assert_eq!(ApiVersion::V1.number(), 1);
        assert_eq!(ApiVersion::V2.number(), 2);
        assert_eq!(ApiVersion::V3.number(), 3);
    }

    #[test]
    fn test_api_version_path_prefix() {
        assert_eq!(ApiVersion::V1.path_prefix(), "/api/v1");
        assert_eq!(ApiVersion::V2.path_prefix(), "/api/v2");
        assert_eq!(ApiVersion::V3.path_prefix(), "/api/v3");
    }

    #[test]
    fn test_api_version_from_path() {
        assert_eq!(ApiVersion::from_path("/api/v1/users"), Some(ApiVersion::V1));
        assert_eq!(ApiVersion::from_path("/api/v2/messages"), Some(ApiVersion::V2));
        assert_eq!(ApiVersion::from_path("/api/v3/health"), Some(ApiVersion::V3));
        assert_eq!(ApiVersion::from_path("/health"), None);
        assert_eq!(ApiVersion::from_path("/api/v4/test"), None);
    }

    #[test]
    fn test_api_version_from_accept_header() {
        assert_eq!(
            ApiVersion::from_accept_header("application/vnd.omnilink.v1+json"),
            Some(ApiVersion::V1)
        );
        assert_eq!(
            ApiVersion::from_accept_header("application/vnd.omnilink.v2+json"),
            Some(ApiVersion::V2)
        );
        assert_eq!(
            ApiVersion::from_accept_header("application/json"),
            None
        );
    }

    #[test]
    fn test_api_version_accept_value() {
        assert_eq!(ApiVersion::V1.accept_value(), "application/vnd.omnilink.v1+json");
        assert_eq!(ApiVersion::V2.accept_value(), "application/vnd.omnilink.v2+json");
    }

    #[test]
    fn test_api_version_display() {
        assert_eq!(ApiVersion::V1.to_string(), "v1");
        assert_eq!(ApiVersion::V2.to_string(), "v2");
        assert_eq!(ApiVersion::V3.to_string(), "v3");
    }

    #[test]
    fn test_api_version_ordering() {
        assert!(ApiVersion::V1 < ApiVersion::V2);
        assert!(ApiVersion::V2 < ApiVersion::V3);
    }

    #[test]
    fn test_api_version_is_latest() {
        assert!(!ApiVersion::V1.is_latest());
        assert!(!ApiVersion::V2.is_latest());
        assert!(ApiVersion::V3.is_latest());
    }

    #[test]
    fn test_api_version_is_deprecated() {
        assert!(ApiVersion::V1.is_deprecated());
        assert!(!ApiVersion::V2.is_deprecated());
        assert!(!ApiVersion::V3.is_deprecated());
    }

    #[test]
    fn test_api_version_sunset_date() {
        assert_eq!(ApiVersion::V1.sunset_date(), Some("2026-12-31"));
        assert_eq!(ApiVersion::V2.sunset_date(), None);
    }

    #[test]
    fn test_version_source_equality() {
        assert_eq!(VersionSource::Path, VersionSource::Path);
        assert_ne!(VersionSource::Path, VersionSource::AcceptHeader);
    }

    #[test]
    fn test_unsupported_version_response_structure() {
        // 确保响应可以正常创建
        let _ = unsupported_version_response("v99");
    }
}
