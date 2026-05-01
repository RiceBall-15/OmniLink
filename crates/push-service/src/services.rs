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
/// - 极光推送 (JPush)
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
    /// 
    /// # 参数
    /// - `request`: 推送消息请求
    /// 
    /// # 返回
    /// - `Result<PushMessage>`: 推送消息记录
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

        // 异步发送推送
        let msg_id = message.id;
        let device_type = message.device_type.clone();
        let device_token = message.device_token.clone();
        let title = message.title.clone();
        let body = message.body.clone();
        let data = message.data.clone();
        let badge = message.badge;
        let sound = message.sound.clone();
        let priority = message.priority;
        let ttl = message.ttl;

        tokio::spawn(async move {
            // 根据设备类型选择推送平台
            let result = match device_type.as_str() {
                "ios" => Self::_send_apns(&device_token, &title, &body, data.as_ref(), badge, sound.as_deref(), priority, ttl).await,
                "android" => Self::_send_fcm(&device_token, &title, &body, data.as_ref(), priority, ttl).await,
                _ => {
                    tracing::warn!("Unsupported device type: {}", device_type);
                    Err(anyhow::anyhow!("Unsupported device type"))
                }
            };

            // 更新推送状态
            if let Err(e) = result {
                let _ = self.repository.update_push_status(msg_id, "failed", Some(&e.to_string())).await;
            } else {
                let _ = self.repository.update_push_status(msg_id, "sent", None).await;
            }
        });

        Ok(message)
    }

    /// 批量发送推送消息
    /// 
    /// # 参数
    /// - `request`: 批量推送请求
    /// 
    /// # 返回
    /// - `Result<BatchPushResponse>`: 批量推送响应
    pub async fn batch_send_push(&self, request: BatchPushRequest) -> Result<BatchPushResponse> {
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();

        for req in request.messages {
            match self.send_push(req).await {
                Ok(msg) => succeeded.push(msg.id),
                Err(e) => {
                    tracing::error!("Failed to send push: {:?}", e);
                    failed.push(Uuid::new_v4()); // 生成一个UUID占位
                }
            }
        }

        Ok(BatchPushResponse { succeeded, failed })
    }

    /// 使用模板发送推送
    /// 
    /// # 参数
    /// - `request`: 模板推送请求
    /// 
    /// # 返回
    /// - `Result<PushMessage>`: 推送消息记录
    pub async fn send_template_push(&self, request: TemplatePushRequest) -> Result<PushMessage> {
        // 获取模板
        let template = self
            .repository
            .get_template(&request.template_name)
            .await?
            .context("Template not found")?;

        // 渲染模板（简单的字符串替换，实际可以使用模板引擎如tera）
        let title = self._render_template(&template.title_template, &request.variables);
        let body = self._render_template(&template.body_template, &request.variables);

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
    /// 
    /// # 参数
    /// - `user_id`: 用户ID
    /// - `page`: 页码
    /// - `page_size`: 每页大小
    /// 
    /// # 返回
    /// - `Result<Vec<PushMessage>>`: 推送消息列表
    pub async fn get_user_push_history(
        &self,
        user_id: Uuid,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<PushMessage>> {
        let offset = (page - 1) * page_size;
        self.repository.get_user_push_messages(user_id, page_size, offset).await
    }

    /// 创建推送模板
    /// 
    /// # 参数
    /// - `request`: 模板创建请求
    /// 
    /// # 返回
    /// - `Result<PushTemplate>`: 推送模板
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
    /// 
    /// # 返回
    /// - `Result<Vec<PushTemplate>>`: 模板列表
    pub async fn list_templates(&self) -> Result<Vec<PushTemplate>> {
        self.repository.list_templates().await
    }

    /// 删除推送模板
    /// 
    /// # 参数
    /// - `name`: 模板名称
    /// 
    /// # 返回
    /// - `Result<bool>`: 是否删除成功
    pub async fn delete_template(&self, name: &str) -> Result<bool> {
        self.repository.delete_template(name).await
    }

    /// 获取推送统计
    /// 
    /// # 参数
    /// - `start_date`: 开始日期
    /// - `end_date`: 结束日期
    /// 
    /// # 返回
    /// - `Result<PushStats>`: 推送统计数据
    pub async fn get_push_stats(
        &self,
        start_date: Option<chrono::DateTime<chrono::Utc>>,
        end_date: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<PushStats> {
        self.repository.get_push_stats(start_date, end_date).await
    }

    /// 清理过期推送记录
    /// 
    /// # 参数
    /// - `days`: 保留天数
    /// 
    /// # 返回
    /// - `Result<u64>`: 删除的记录数
    pub async fn cleanup_old_messages(&self, days: i64) -> Result<u64> {
        self.repository.cleanup_old_messages(days).await
    }

    // 内部方法

    /// 渲染模板（简单实现）
    fn _render_template(template: &str, variables: &serde_json::Value) -> String {
        let mut result = template.to_string();
        
        if let Some(obj) = variables.as_object() {
            for (key, value) in obj {
                let placeholder = format!("{{{{{}}}}}", key);
                let replacement = value.as_str().unwrap_or(&value.to_string());
                result = result.replace(&placeholder, replacement);
            }
        }
        
        result
    }

    /// 发送APNs推送
    async fn _send_apns(
        device_token: &str,
        title: &str,
        body: &str,
        data: Option<&serde_json::Value>,
        badge: Option<i32>,
        sound: Option<&str>,
        priority: Option<i32>,
        ttl: Option<i32>,
    ) -> Result<()> {
        // TODO: 实现APNs推送
        // 需要集成 apns2 或类似库
        tracing::info!(
            "Sending APNs push to {}: {} - {}",
            device_token,
            title,
            body
        );
        
        // 模拟推送成功
        Ok(())
    }

    /// 发送FCM推送
    async fn _send_fcm(
        device_token: &str,
        title: &str,
        body: &str,
        data: Option<&serde_json::Value>,
        priority: Option<i32>,
        ttl: Option<i32>,
    ) -> Result<()> {
        // TODO: 实现FCM推送
        // 需要集成 fcm或使用REST API
        tracing::info!(
            "Sending FCM push to {}: {} - {}",
            device_token,
            title,
            body
        );
        
        // 模拟推送成功
        Ok(())
    }

    /// 发送极光推送
    async fn _send_jpush(
        device_token: &str,
        title: &str,
        body: &str,
        data: Option<&serde_json::Value>,
    ) -> Result<()> {
        // TODO: 实现极光推送
        // 需要集成极光推送SDK
        tracing::info!(
            "Sending JPush to {}: {} - {}",
            device_token,
            title,
            body
        );
        
        // 模拟推送成功
        Ok(())
    }
}

// 为修复编译错误，添加必要的字段访问
impl PushRepository {
    fn get_pool(&self) -> &PgPool {
        &self.pool
    }
}