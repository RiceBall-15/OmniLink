//! Push Notification Providers
//!
//! 实现各平台推送服务的真实调用：
//! - FCM (Firebase Cloud Messaging) - 使用 HTTP v1 API
//! - APNs (Apple Push Notification Service) - 使用 HTTP/2 API
//! - Web Push - 使用 Web Push Protocol (RFC 8030)
//!
//! 配置通过环境变量读取，未配置时使用模拟模式（开发环境）

use anyhow::{Context, Result};
use serde_json::json;
use std::env;

/// FCM (Firebase Cloud Messaging) Provider
///
/// 使用 FCM HTTP v1 API 发送推送
/// 文档: https://firebase.google.com/docs/reference/fcm/rest/v1/projects.messages
pub struct FcmProvider {
    project_id: String,
    access_token: String,
    client: reqwest::Client,
}

impl FcmProvider {
    /// 从环境变量创建 FCM Provider
    ///
    /// 环境变量:
    /// - FIREBASE_PROJECT_ID: Firebase 项目 ID
    /// - FIREBASE_ACCESS_TOKEN: OAuth2 access token (或 service account key)
    pub fn from_env() -> Option<Self> {
        let project_id = env::var("FIREBASE_PROJECT_ID").ok()?;
        let access_token = env::var("FIREBASE_ACCESS_TOKEN").ok()?;

        if project_id.is_empty() || access_token.is_empty() {
            tracing::warn!("FCM credentials not configured, using mock mode");
            return None;
        }

        Some(Self {
            project_id,
            access_token,
            client: reqwest::Client::new(),
        })
    }

    /// 发送 FCM 推送消息
    ///
    /// 使用 FCM HTTP v1 API
    /// POST https://fcm.googleapis.com/v1/projects/{project_id}/messages:send
    pub async fn send(
        &self,
        device_token: &str,
        title: &str,
        body: &str,
        data: Option<&serde_json::Value>,
        badge: Option<i32>,
        sound: Option<&str>,
    ) -> Result<String> {
        let url = format!(
            "https://fcm.googleapis.com/v1/projects/{}/messages:send",
            self.project_id
        );

        // 构建 FCM v1 消息体
        let mut message = json!({
            "token": device_token,
            "notification": {
                "title": title,
                "body": body,
            },
            "android": {
                "priority": "high",
                "notification": {
                    "channel_id": "omnilink_messages",
                    "sound": sound.unwrap_or("default"),
                }
            }
        });

        // 添加 badge (iOS)
        if let Some(badge_count) = badge {
            message["apns"] = json!({
                "payload": {
                    "aps": {
                        "badge": badge_count,
                        "sound": sound.unwrap_or("default"),
                    }
                }
            });
        }

        // 添加自定义数据
        if let Some(data_value) = data {
            if let Some(obj) = data_value.as_object() {
                let mut data_map = serde_json::Map::new();
                for (key, value) in obj {
                    // FCM data values must be strings
                    data_map.insert(
                        key.clone(),
                        serde_json::Value::String(
                            value.as_str().unwrap_or(&value.to_string()).to_string(),
                        ),
                    );
                }
                message["data"] = serde_json::Value::Object(data_map);
            }
        }

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&json!({ "message": message }))
            .send()
            .await
            .context("Failed to send FCM request")?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .context("Failed to read FCM response")?;

        if status.is_success() {
            // 解析响应获取 message_id
            let response_json: serde_json::Value =
                serde_json::from_str(&response_text).unwrap_or_default();
            let message_id = response_json["name"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            tracing::info!("FCM push sent successfully: {}", message_id);
            Ok(message_id)
        } else {
            tracing::error!("FCM push failed: {} - {}", status, response_text);
            Err(anyhow::anyhow!("FCM error: {} - {}", status, response_text))
        }
    }
}

/// APNs (Apple Push Notification Service) Provider
///
/// 使用 APNs HTTP/2 API 发送推送
/// 文档: https://developer.apple.com/documentation/usernotifications/sending-notification-requests-to-apns
pub struct ApnsProvider {
    key_id: String,
    team_id: String,
    bundle_id: String,
    key_content: String, // PEM 格式的私钥
    client: reqwest::Client,
    use_sandbox: bool,
}

impl ApnsProvider {
    /// 从环境变量创建 APNs Provider
    ///
    /// 环境变量:
    /// - APNS_KEY_ID: Apple Push Notification Key ID
    /// - APNS_TEAM_ID: Apple Team ID
    /// - APNS_BUNDLE_ID: App Bundle ID
    /// - APNS_KEY_CONTENT: PEM 格式的私钥内容
    /// - APNS_USE_SANDBOX: 是否使用沙箱环境 (true/false)
    pub fn from_env() -> Option<Self> {
        let key_id = env::var("APNS_KEY_ID").ok()?;
        let team_id = env::var("APNS_TEAM_ID").ok()?;
        let bundle_id = env::var("APNS_BUNDLE_ID").unwrap_or_else(|_| "com.omnilink.app".to_string());
        let key_content = env::var("APNS_KEY_CONTENT").ok()?;
        let use_sandbox = env::var("APNS_USE_SANDBOX")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);

        if key_id.is_empty() || team_id.is_empty() || key_content.is_empty() {
            tracing::warn!("APNs credentials not configured, using mock mode");
            return None;
        }

        Some(Self {
            key_id,
            team_id,
            bundle_id,
            key_content,
            client: reqwest::Client::new(),
            use_sandbox,
        })
    }

    /// 生成 APNs JWT Token
    ///
    /// 使用 ES256 算法签名
    fn generate_token(&self) -> Result<String> {
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

        let now = chrono::Utc::now().timestamp() as usize;

        let claims = json!({
            "iss": self.team_id,
            "iat": now,
        });

        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(self.key_id.clone());

        let key = EncodingKey::from_ec_pem(self.key_content.as_bytes())
            .context("Failed to parse APNs private key")?;

        encode(&header, &claims, &key).context("Failed to generate APNs JWT token")
    }

    /// 发送 APNs 推送消息
    ///
    /// POST https://api.push.apple.com/3/device/{device_token}
    pub async fn send(
        &self,
        device_token: &str,
        title: &str,
        body: &str,
        data: Option<&serde_json::Value>,
        badge: Option<i32>,
        sound: Option<&str>,
    ) -> Result<()> {
        let host = if self.use_sandbox {
            "https://api.sandbox.push.apple.com"
        } else {
            "https://api.push.apple.com"
        };

        let url = format!("{}/3/device/{}", host, device_token);
        let jwt_token = self.generate_token()?;

        // 构建 APNs payload
        let mut aps = json!({
            "alert": {
                "title": title,
                "body": body,
            },
            "sound": sound.unwrap_or("default"),
            "content-available": 1,
        });

        if let Some(badge_count) = badge {
            aps["badge"] = json!(badge_count);
        }

        let mut payload = json!({ "aps": aps });

        // 添加自定义数据
        if let Some(data_value) = data {
            if let Some(obj) = data_value.as_object() {
                for (key, value) in obj {
                    payload[key.clone()] = value.clone();
                }
            }
        }

        let response = self
            .client
            .post(&url)
            .bearer_auth(&jwt_token)
            .header("apns-topic", &self.bundle_id)
            .header("apns-push-type", "alert")
            .header("apns-priority", "10")
            .json(&payload)
            .send()
            .await
            .context("Failed to send APNs request")?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .context("Failed to read APNs response")?;

        if status.is_success() {
            tracing::info!("APNs push sent successfully to {}", device_token);
            Ok(())
        } else {
            tracing::error!("APNs push failed: {} - {}", status, response_text);
            Err(anyhow::anyhow!("APNs error: {} - {}", status, response_text))
        }
    }
}

/// Web Push Provider
///
/// 使用 Web Push Protocol (RFC 8030) 发送推送
/// 文档: https://developers.google.com/web/fundamentals/push-notifications
pub struct WebPushProvider {
    vapid_private_key: String,
    vapid_public_key: String,
    vapid_subject: String, // mailto: 或 https:// URL
    client: reqwest::Client,
}

impl WebPushProvider {
    /// 从环境变量创建 Web Push Provider
    ///
    /// 环境变量:
    /// - VAPID_PRIVATE_KEY: VAPID 私钥 (Base64 URL-safe 编码)
    /// - VAPID_PUBLIC_KEY: VAPID 公钥 (Base64 URL-safe 编码)
    /// - VAPID_SUBJECT: VAPID 主题 (mailto: 或 https:// URL)
    pub fn from_env() -> Option<Self> {
        let vapid_private_key = env::var("VAPID_PRIVATE_KEY").ok()?;
        let vapid_public_key = env::var("VAPID_PUBLIC_KEY").ok()?;
        let vapid_subject = env::var("VAPID_SUBJECT")
            .unwrap_or_else(|_| "mailto:admin@omnilink.com".to_string());

        if vapid_private_key.is_empty() || vapid_public_key.is_empty() {
            tracing::warn!("VAPID credentials not configured, using mock mode");
            return None;
        }

        Some(Self {
            vapid_private_key,
            vapid_public_key,
            vapid_subject,
            client: reqwest::Client::new(),
        })
    }

    /// 发送 Web Push 消息
    ///
    /// 实现 Web Push Protocol
    /// 注意: 完整实现需要 web-push 库，这里提供基础 HTTP 调用框架
    pub async fn send(
        &self,
        endpoint: &str,
        title: &str,
        body: &str,
        data: Option<&serde_json::Value>,
    ) -> Result<()> {
        // 构建 Web Push payload
        let mut payload = json!({
            "title": title,
            "body": body,
            "icon": "/icons/notification-icon.png",
            "badge": "/icons/badge-icon.png",
            "data": {
                "url": "/",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }
        });

        // 添加自定义数据
        if let Some(data_value) = data {
            if let Some(obj) = data_value.as_object() {
                for (key, value) in obj {
                    payload["data"][key.clone()] = value.clone();
                }
            }
        }

        // 生成 VAPID Authorization header
        // 注意: 完整实现需要使用 web-push 库进行加密
        // 这里提供框架，实际部署时应集成 web-push crate
        let auth_header = format!(
            "vapid t={}, k={}",
            self.generate_vapid_token()?,
            self.vapid_public_key
        );

        let response = self
            .client
            .post(endpoint)
            .header("Authorization", &auth_header)
            .header("Content-Type", "application/json")
            .header("TTL", "86400") // 24 hours
            .json(&payload)
            .send()
            .await
            .context("Failed to send Web Push request")?;

        let status = response.status();

        if status.is_success() || status == reqwest::StatusCode::CREATED {
            tracing::info!("Web Push sent successfully to {}", endpoint);
            Ok(())
        } else {
            let response_text = response.text().await.unwrap_or_default();
            tracing::error!("Web Push failed: {} - {}", status, response_text);
            Err(anyhow::anyhow!(
                "Web Push error: {} - {}",
                status,
                response_text
            ))
        }
    }

    /// 生成 VAPID JWT Token
    fn generate_vapid_token(&self) -> Result<String> {
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

        let now = chrono::Utc::now().timestamp() as usize;
        let endpoint_origin = "https://fcm.googleapis.com"; // 应该从 endpoint 解析

        let claims = json!({
            "aud": endpoint_origin,
            "exp": now + 86400, // 24 hours
            "sub": self.vapid_subject,
        });

        let key = EncodingKey::from_base64_secret(&self.vapid_private_key)
            .context("Failed to parse VAPID private key")?;

        encode(&Header::new(Algorithm::ES256), &claims, &key)
            .context("Failed to generate VAPID JWT token")
    }
}

/// Push Provider 枚举
///
/// 统一管理所有推送提供商
pub enum PushProvider {
    Fcm(FcmProvider),
    Apns(ApnsProvider),
    WebPush(WebPushProvider),
    Mock { device_type: String },
}

impl PushProvider {
    /// 根据设备类型创建对应的 Provider
    pub fn for_device_type(device_type: &str) -> Self {
        match device_type {
            "android" => {
                if let Some(provider) = FcmProvider::from_env() {
                    PushProvider::Fcm(provider)
                } else {
                    PushProvider::Mock {
                        device_type: device_type.to_string(),
                    }
                }
            }
            "ios" => {
                if let Some(provider) = ApnsProvider::from_env() {
                    PushProvider::Apns(provider)
                } else {
                    PushProvider::Mock {
                        device_type: device_type.to_string(),
                    }
                }
            }
            "web" => {
                if let Some(provider) = WebPushProvider::from_env() {
                    PushProvider::WebPush(provider)
                } else {
                    PushProvider::Mock {
                        device_type: device_type.to_string(),
                    }
                }
            }
            _ => PushProvider::Mock {
                device_type: device_type.to_string(),
            },
        }
    }

    /// 发送推送消息
    pub async fn send(
        &self,
        device_token: &str,
        title: &str,
        body: &str,
        data: Option<&serde_json::Value>,
        badge: Option<i32>,
        sound: Option<&str>,
    ) -> Result<()> {
        match self {
            PushProvider::Fcm(provider) => {
                provider
                    .send(device_token, title, body, data, badge, sound)
                    .await?;
                Ok(())
            }
            PushProvider::Apns(provider) => {
                provider
                    .send(device_token, title, body, data, badge, sound)
                    .await
            }
            PushProvider::WebPush(provider) => {
                provider.send(device_token, title, body, data).await
            }
            PushProvider::Mock { device_type } => {
                tracing::info!(
                    "[MOCK] Sending {} push to {}: {} - {}",
                    device_type,
                    device_token,
                    title,
                    body
                );
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fcm_provider_not_configured() {
        // 确保环境变量未设置
        env::remove_var("FIREBASE_PROJECT_ID");
        env::remove_var("FIREBASE_ACCESS_TOKEN");

        let provider = FcmProvider::from_env();
        assert!(provider.is_none());
    }

    #[test]
    fn test_apns_provider_not_configured() {
        env::remove_var("APNS_KEY_ID");
        env::remove_var("APNS_TEAM_ID");
        env::remove_var("APNS_KEY_CONTENT");

        let provider = ApnsProvider::from_env();
        assert!(provider.is_none());
    }

    #[test]
    fn test_webpush_provider_not_configured() {
        env::remove_var("VAPID_PRIVATE_KEY");
        env::remove_var("VAPID_PUBLIC_KEY");

        let provider = WebPushProvider::from_env();
        assert!(provider.is_none());
    }

    #[test]
    fn test_push_provider_for_device_types() {
        // Android 应该返回 Mock（因为没有配置 FCM）
        let provider = PushProvider::for_device_type("android");
        assert!(matches!(provider, PushProvider::Mock { .. }));

        // iOS 应该返回 Mock（因为没有配置 APNs）
        let provider = PushProvider::for_device_type("ios");
        assert!(matches!(provider, PushProvider::Mock { .. }));

        // Web 应该返回 Mock（因为没有配置 VAPID）
        let provider = PushProvider::for_device_type("web");
        assert!(matches!(provider, PushProvider::Mock { .. }));

        // 未知类型应该返回 Mock
        let provider = PushProvider::for_device_type("unknown");
        assert!(matches!(provider, PushProvider::Mock { .. }));
    }

    #[tokio::test]
    async fn test_mock_push_sends_successfully() {
        let provider = PushProvider::Mock {
            device_type: "android".to_string(),
        };

        let result = provider
            .send("test_token", "Test Title", "Test Body", None, None, None)
            .await;

        assert!(result.is_ok());
    }
}
