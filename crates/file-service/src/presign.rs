//! AWS Signature V4 - 用于生成 MinIO/S3 预签名 URL
//!
//! 实现了简化版的 AWS Signature V4 签名算法，
//! 支持生成 PUT 和 GET 预签名 URL。

use anyhow::Result;
use chrono::Utc;
use sha2::{Digest, Sha256};
use hmac::{Hmac, Mac};

type HmacSha256 = Hmac<Sha256>;

/// 预签名 URL 配置
#[derive(Debug, Clone)]
pub struct PresignConfig {
    pub endpoint: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    pub region: String,
    pub use_ssl: bool,
}

impl PresignConfig {
    pub fn from_minio_config(config: &super::storage::MinioConfig) -> Self {
        Self {
            endpoint: config.endpoint.clone(),
            bucket: config.bucket.clone(),
            access_key: config.access_key.clone(),
            secret_key: config.secret_key.clone(),
            region: config.region.clone(),
            use_ssl: config.use_ssl,
        }
    }

    fn protocol(&self) -> &str {
        if self.use_ssl { "https" } else { "http" }
    }

    fn host(&self) -> &str {
        &self.endpoint
    }
}

/// 计算 SHA256 哈希
fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// 计算 HMAC-SHA256
fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key size");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// 获取签名密钥
fn get_signature_key(secret: &str, date: &str, region: &str, service: &str) -> Vec<u8> {
    let k_date = hmac_sha256(format!("AWS4{}", secret).as_bytes(), date.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    hmac_sha256(&k_service, b"aws4_request")
}

/// 生成预签名下载 URL (GET)
///
/// # Arguments
/// * `config` - MinIO/S3 配置
/// * `object_key` - 对象路径（不包含 bucket）
/// * `expires_in_secs` - URL 有效期（秒），默认 3600
///
/// # Returns
/// 预签名的完整 URL
pub fn generate_presigned_get_url(
    config: &PresignConfig,
    object_key: &str,
    expires_in_secs: u64,
) -> Result<String> {
    let now = Utc::now();
    let date = now.format("%Y%m%d").to_string();
    let datetime = now.format("%Y%m%dT%H%M%SZ").to_string();
    let credential = format!("{}/{}/{}/s3/aws4_request", config.access_key, date, config.region);

    let host = config.host();
    let protocol = config.protocol();

    // 构建规范查询参数
    let mut query_params: Vec<(String, String)> = vec![
        ("X-Amz-Algorithm".to_string(), "AWS4-HMAC-SHA256".to_string()),
        ("X-Amz-Credential".to_string(), credential),
        ("X-Amz-Date".to_string(), datetime.clone()),
        ("X-Amz-Expires".to_string(), expires_in_secs.to_string()),
        ("X-Amz-SignedHeaders".to_string(), "host".to_string()),
    ];
    query_params.sort();

    // 构建规范请求
    let query_string: String = query_params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    let canonical_request = format!(
        "GET\n/{bucket}/{key}\n{query}\nhost:{host}\n\nhost\nUNSIGNED-PAYLOAD",
        bucket = config.bucket,
        key = object_key,
        query = query_string,
        host = host,
    );

    // 构建签名字符串
    let scope = format!("{}/{}/s3/aws4_request", date, config.region);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{datetime}\n{scope}\n{hash}",
        datetime = datetime,
        scope = scope,
        hash = sha256_hex(canonical_request.as_bytes()),
    );

    // 计算签名
    let signing_key = get_signature_key(&config.secret_key, &date, &config.region, "s3");
    let signature = hex::encode(hmac_sha256(&signing_key, string_to_sign.as_bytes()));

    // 构建最终 URL
    let url = format!(
        "{protocol}://{host}/{bucket}/{key}?{query}&X-Amz-Signature={signature}",
        protocol = protocol,
        host = host,
        bucket = config.bucket,
        key = object_key,
        query = query_string,
        signature = signature,
    );

    Ok(url)
}

/// 生成预签名上传 URL (PUT)
///
/// # Arguments
/// * `config` - MinIO/S3 配置
/// * `object_key` - 对象路径（不包含 bucket）
/// * `content_type` - 文件 MIME 类型
/// * `expires_in_secs` - URL 有效期（秒），默认 3600
///
/// # Returns
/// 预签名的完整 URL（客户端可直接 PUT 文件到此 URL）
pub fn generate_presigned_put_url(
    config: &PresignConfig,
    object_key: &str,
    content_type: &str,
    expires_in_secs: u64,
) -> Result<String> {
    let now = Utc::now();
    let date = now.format("%Y%m%d").to_string();
    let datetime = now.format("%Y%m%dT%H%M%SZ").to_string();
    let credential = format!("{}/{}/{}/s3/aws4_request", config.access_key, date, config.region);

    let host = config.host();
    let protocol = config.protocol();

    // 构建规范查询参数
    let mut query_params: Vec<(String, String)> = vec![
        ("X-Amz-Algorithm".to_string(), "AWS4-HMAC-SHA256".to_string()),
        ("X-Amz-Credential".to_string(), credential),
        ("X-Amz-Date".to_string(), datetime.clone()),
        ("X-Amz-Expires".to_string(), expires_in_secs.to_string()),
        ("X-Amz-SignedHeaders".to_string(), "content-type;host".to_string()),
    ];
    query_params.sort();

    let query_string: String = query_params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    let canonical_request = format!(
        "PUT\n/{bucket}/{key}\n{query}\ncontent-type:{ct}\nhost:{host}\n\ncontent-type;host\nUNSIGNED-PAYLOAD",
        bucket = config.bucket,
        key = object_key,
        query = query_string,
        ct = content_type,
        host = host,
    );

    let scope = format!("{}/{}/s3/aws4_request", date, config.region);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{datetime}\n{scope}\n{hash}",
        datetime = datetime,
        scope = scope,
        hash = sha256_hex(canonical_request.as_bytes()),
    );

    let signing_key = get_signature_key(&config.secret_key, &date, &config.region, "s3");
    let signature = hex::encode(hmac_sha256(&signing_key, string_to_sign.as_bytes()));

    let url = format!(
        "{protocol}://{host}/{bucket}/{key}?{query}&X-Amz-Signature={signature}",
        protocol = protocol,
        host = host,
        bucket = config.bucket,
        key = object_key,
        query = query_string,
        signature = signature,
    );

    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> PresignConfig {
        PresignConfig {
            endpoint: "localhost:9000".to_string(),
            bucket: "omnilink".to_string(),
            access_key: "minioadmin".to_string(),
            secret_key: "minioadmin".to_string(),
            region: "us-east-1".to_string(),
            use_ssl: false,
        }
    }

    #[test]
    fn test_generate_presigned_get_url() {
        let config = test_config();
        let url = generate_presigned_get_url(&config, "test/file.txt", 3600).unwrap();
        assert!(url.starts_with("http://localhost:9000/omnilink/test/file.txt?"));
        assert!(url.contains("X-Amz-Algorithm=AWS4-HMAC-SHA256"));
        assert!(url.contains("X-Amz-Expires=3600"));
        assert!(url.contains("X-Amz-Signature="));
    }

    #[test]
    fn test_generate_presigned_put_url() {
        let config = test_config();
        let url = generate_presigned_put_url(&config, "test/file.txt", "image/jpeg", 3600).unwrap();
        assert!(url.starts_with("http://localhost:9000/omnilink/test/file.txt?"));
        assert!(url.contains("X-Amz-SignedHeaders=content-type%3Bhost"));
    }

    #[test]
    fn test_sha256_hex() {
        let hash = sha256_hex(b"hello");
        assert_eq!(hash.len(), 64);
        // Known SHA256 of "hello"
        assert_eq!(hash, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
    }
}
