use anyhow::{Context, Result};
use sqlx::PgPool;
use uuid::Uuid;

use super::models::*;
use super::repository::PushRepository;

/// Push Service - 处理所有推送相关业务逻辑
///
/// 支持多种推送平台：
/// - APNs (Apple Push Notification Service)
/// - FCM (Firebase Cloud Messaging)
/// - Web Push
pub struct PushService {
    repository: PushRepository,
}

impl PushService {
    /// 创建新的推送服务实例
    pub fn new(pool: PgPool) -> Self {
        Self {
            repository: PushRepository::new(pool),
        }
    }

    /// 发送单条推送消息
    pub async fn send_push(&self, request: CreatePushRequest) -> Result<PushMessage> {
        let message_id = Uuid::new_v4();
        let created_at = chrono::Utc::now();

        // 创建推送消息记录
        let message = PushMessage {
            id: message_id,
            user_id: request.user_id,
            device_type: request.device_type.clone(),
            device_token: request.device_token.clone(),
            title: request.title,
            body: request.body,
            data: request.data,
            badge: request.badge,
            sound: request.sound,
            priority: request.priority,
            ttl: request.ttl,
            created_at,
            sent_at: None,
            failed_at: None,
            status: "pending".to_string(),
            error: None,
        };

        // 保存到数据库
        let message = self.repository.create_push_message(message).await?;

        // 根据设备类型选择推送平台并发送
        let result = match message.device_type.as_str() {
            "ios" => {
                Self::send_apns(
                    &message.device_token,
                    &message.title,
                    &message.body,
                    message.data.as_ref(),
                    message.badge,
                    message.sound.as_deref(),
                )
                .await
            }
            "android" => {
                Self::send_fcm(
                    &message.device_token,
                    &message.title,
                    &message.body,
                    message.data.as_ref(),
                )
                .await
            }
            "web" => {
                Self::send_web_push(
                    &message.device_token,
                    &message.title,
                    &message.body,
                    message.data.as_ref(),
                )
                .await
            }
            _ => {
                tracing::warn!("Unsupported device type: {}", message.device_type);
                Err(anyhow::anyhow!(
                    "Unsupported device type: {}",
                    message.device_type
                ))
            }
        };

        // 更新推送状态
        if let Err(e) = &result {
            self.repository
                .update_push_status(message.id, "failed", Some(&e.to_string()))
                .await?;
            // Return updated message
            let mut updated = message.clone();
            updated.status = "failed".to_string();
            updated.failed_at = Some(chrono::Utc::now());
            updated.error = Some(e.to_string());
            Ok(updated)
        } else {
            self.repository
                .update_push_status(message.id, "sent", None)
                .await?;
            let mut updated = message.clone();
            updated.status = "sent".to_string();
            updated.sent_at = Some(chrono::Utc::now());
            Ok(updated)
        }
    }

    /// 批量发送推送消息
    pub async fn batch_send_push(&self, request: BatchPushRequest) -> Result<BatchPushResponse> {
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();

        for req in request.messages {
            match self.send_push(req).await {
                Ok(msg) => succeeded.push(msg.id),
                Err(e) => {
                    tracing::error!("Failed to send push: {:?}", e);
                    failed.push(Uuid::new_v4());
                }
            }
        }

        Ok(BatchPushResponse { succeeded, failed })
    }

    /// 使用模板发送推送
    pub async fn send_template_push(&self, request: TemplatePushRequest) -> Result<PushMessage> {
        // 获取模板
        let template = self
            .repository
            .get_template(&request.template_name)
            .await?
            .context("Template not found")?;

        // 渲染模板（简单字符串替换）
        let title = Self::render_template(&template.title_template, &request.variables);
        let body = Self::render_template(&template.body_template, &request.variables);

        // 构建推送请求
        let push_request = CreatePushRequest {
            user_id: request.user_id,
            device_type: request.device_type,
            device_token: request.device_token,
            title,
            body,
            data: template.data_template,
            badge: template.badge.map(|_| 1),
            sound: template.sound,
            priority: None,
            ttl: None,
        };

        self.send_push(push_request).await
    }

    /// 获取用户推送历史
    pub async fn get_user_push_history(
        &self,
        user_id: Uuid,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<PushMessage>> {
        let offset = (page - 1) * page_size;
        self.repository
            .get_user_push_messages(user_id, page_size, offset)
            .await
    }

    /// 创建推送模板
    pub async fn create_template(&self, request: CreateTemplateRequest) -> Result<PushTemplate> {
        let template_id = Uuid::new_v4();
        let now = chrono::Utc::now();

        let template = PushTemplate {
            id: template_id,
            name: request.name,
            title_template: request.title_template,
            body_template: request.body_template,
            data_template: request.data_template,
            sound: request.sound,
            badge: request.badge,
            created_at: now,
            updated_at: now,
        };

        self.repository.create_template(template).await
    }

    /// 获取所有推送模板
    pub async fn list_templates(&self) -> Result<Vec<PushTemplate>> {
        self.repository.list_templates().await
    }

    /// 删除推送模板
    pub async fn delete_template(&self, name: &str) -> Result<bool> {
        self.repository.delete_template(name).await
    }

    /// 获取推送统计
    pub async fn get_push_stats(
        &self,
        start_date: Option<chrono::DateTime<chrono::Utc>>,
        end_date: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<PushStats> {
        self.repository.get_push_stats(start_date, end_date).await
    }

    /// 清理过期推送记录
    pub async fn cleanup_old_messages(&self, days: i64) -> Result<u64> {
        self.repository.cleanup_old_messages(days).await
    }

    // 内部方法

    /// 渲染模板（简单实现）
    fn render_template(template: &str, variables: &serde_json::Value) -> String {
        let mut result = template.to_string();

        if let Some(obj) = variables.as_object() {
            for (key, value) in obj {
                let placeholder = format!("{{{{{}}}}}", key);
                let owned = value.as_str().map(|s| s.to_string()).unwrap_or_else(|| value.to_string());
                result = result.replace(&placeholder, &owned);
            }
        }

        result
    }

    /// 发送APNs推送（模拟实现，生产环境需集成apns2）
    async fn send_apns(
        device_token: &str,
        title: &str,
        body: &str,
        _data: Option<&serde_json::Value>,
        _badge: Option<i32>,
        _sound: Option<&str>,
    ) -> Result<()> {
        tracing::info!(
            "Sending APNs push to {}: {} - {}",
            device_token,
            title,
            body
        );
        // TODO: 实现真实APNs推送
        // 需要集成 apns2 或类似库
        Ok(())
    }

    /// 发送FCM推送（模拟实现，生产环境需集成FCM SDK）
    async fn send_fcm(
        device_token: &str,
        title: &str,
        body: &str,
        _data: Option<&serde_json::Value>,
    ) -> Result<()> {
        tracing::info!(
            "Sending FCM push to {}: {} - {}",
            device_token,
            title,
            body
        );
        // TODO: 实现真实FCM推送
        // 需要集成 fcm 或使用 REST API
        Ok(())
    }

    /// 发送Web Push（模拟实现，生产环境需集成web-push）
    async fn send_web_push(
        device_token: &str,
        title: &str,
        body: &str,
        _data: Option<&serde_json::Value>,
    ) -> Result<()> {
        tracing::info!(
            "Sending Web Push to {}: {} - {}",
            device_token,
            title,
            body
        );
        // TODO: 实现Web Push
        // 需要集成 web-push 库
        Ok(())
    }
}
