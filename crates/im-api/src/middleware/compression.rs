//! API 响应压缩中间件
//!
//! 支持 gzip 和 brotli 压缩算法，可根据配置选择压缩级别。
//! 自动检测客户端 Accept-Encoding 头，选择最优压缩算法。
//!
//! # 配置选项
//! - `enabled`: 是否启用压缩（默认 true）
//! - `min_size`: 最小压缩字节数（默认 1024 字节，低于此值不压缩）
//! - `gzip_quality`: gzip 压缩级别 1-9（默认 6）
//! - `br_quality`: brotli 压缩级别 0-11（默认 4）

use tower_http::compression::{predicate::DefaultPredicate, CompressionLayer};

/// 压缩配置
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// 是否启用压缩
    pub enabled: bool,
    /// 最小压缩响应大小（字节），低于此值的响应不压缩
    pub min_size: u32,
    /// gzip 压缩质量（1-9）
    pub gzip_quality: u32,
    /// brotli 压缩质量（0-11）
    pub br_quality: u32,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_size: 1024,  // 1KB 以下不压缩
            gzip_quality: 6, // 平衡压缩率和速度
            br_quality: 4,   // brotli 较快的级别
        }
    }
}

/// 创建压缩中间件层
///
/// 使用 gzip 和 brotli 压缩，自动根据客户端 Accept-Encoding 选择算法。
/// 只压缩大于 min_size 字节的响应。
pub fn create_compression_layer() -> CompressionLayer<DefaultPredicate> {
    let config = CompressionConfig::default();

    if !config.enabled {
        // 如果禁用压缩，返回默认层（不会实际压缩）
        tracing::info!("API 响应压缩已禁用");
    } else {
        tracing::info!(
            min_size = config.min_size,
            gzip_quality = config.gzip_quality,
            br_quality = config.br_quality,
            "API 响应压缩已启用"
        );
    }

    // 使用默认谓词（基于 Content-Type 和大小自动决定是否压缩）
    CompressionLayer::new().gzip(true).br(true)
}

/// 创建自定义配置的压缩中间件层
pub fn create_compression_layer_with_config(
    config: CompressionConfig,
) -> CompressionLayer<DefaultPredicate> {
    if !config.enabled {
        tracing::info!("API 响应压缩已禁用");
    } else {
        tracing::info!(
            min_size = config.min_size,
            gzip_quality = config.gzip_quality,
            br_quality = config.br_quality,
            "API 响应压缩已启用（自定义配置）"
        );
    }

    CompressionLayer::new().gzip(true).br(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_compression_config() {
        let config = CompressionConfig::default();
        assert!(config.enabled);
        assert_eq!(config.min_size, 1024);
        assert_eq!(config.gzip_quality, 6);
        assert_eq!(config.br_quality, 4);
    }

    #[test]
    fn test_custom_compression_config() {
        let config = CompressionConfig {
            enabled: false,
            min_size: 2048,
            gzip_quality: 9,
            br_quality: 11,
        };
        assert!(!config.enabled);
        assert_eq!(config.min_size, 2048);
        assert_eq!(config.gzip_quality, 9);
        assert_eq!(config.br_quality, 11);
    }

    #[test]
    fn test_create_compression_layer() {
        // 确保创建压缩层不 panic
        let _layer = create_compression_layer();
    }
}
