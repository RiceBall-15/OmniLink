use base64::Engine;
use reqwest::Client;
use serde_json::{json, Value};
use uuid::Uuid;
use chrono::Utc;

use crate::config::TEST_CONFIG;

/// 测试数据工厂
/// 
/// 用于生成测试数据，支持创建用户、消息、会话等
pub struct TestFactory {
    client: Client,
    base_url: String,
    auth_token: String,
}

impl TestFactory {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: TEST_CONFIG.base_url.clone(),
            auth_token: TEST_CONFIG.auth_token.clone(),
        }
    }

    pub fn with_auth(token: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: TEST_CONFIG.base_url.clone(),
            auth_token: token.to_string(),
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.auth_token)
    }

    // ========== 用户相关 ==========

    /// 生成随机用户注册数据
    pub fn user_registration_data(&self) -> Value {
        let id = Uuid::new_v4().to_string();
        json!({
            "username": format!("testuser_{}", &id[..8]),
            "email": format!("test_{}@example.com", &id[..8]),
            "password": "TestPassword123!",
            "displayName": format!("Test User {}", &id[..8])
        })
    }

    /// 注册用户
    pub async fn register_user(&self) -> Result<Value, reqwest::Error> {
        let data = self.user_registration_data();
        let response = self.client
            .post(format!("{}/api/auth/register", self.base_url))
            .header("Authorization", self.auth_header())
            .json(&data)
            .send()
            .await?;
        
        response.json().await
    }

    /// 登录用户
    pub async fn login_user(&self, username: &str, password: &str) -> Result<Value, reqwest::Error> {
        let response = self.client
            .post(format!("{}/api/auth/login", self.base_url))
            .json(&json!({
                "username": username,
                "password": password
            }))
            .send()
            .await?;
        
        response.json().await
    }

    // ========== 消息相关 ==========

    /// 生成文本消息数据
    pub fn text_message(&self, conversation_id: &str) -> Value {
        json!({
            "conversationId": conversation_id,
            "content": format!("测试消息 {}", Uuid::new_v4().to_string()[..8].to_string()),
            "contentType": "text",
            "metadata": {}
        })
    }

    /// 生成图片消息数据
    pub fn image_message(&self, conversation_id: &str, file_url: &str) -> Value {
        json!({
            "conversationId": conversation_id,
            "content": file_url,
            "contentType": "image",
            "metadata": {
                "width": 800,
                "height": 600,
                "size": 102400
            }
        })
    }

    /// 生成文件消息数据
    pub fn file_message(&self, conversation_id: &str, file_url: &str) -> Value {
        json!({
            "conversationId": conversation_id,
            "content": file_url,
            "contentType": "file",
            "metadata": {
                "fileName": "test.pdf",
                "fileSize": 1048576,
                "mimeType": "application/pdf"
            }
        })
    }

    /// 发送消息
    pub async fn send_message(&self, conversation_id: &str) -> Result<Value, reqwest::Error> {
        let data = self.text_message(conversation_id);
        let response = self.client
            .post(format!("{}/api/im/messages", self.base_url))
            .header("Authorization", self.auth_header())
            .json(&data)
            .send()
            .await?;
        
        response.json().await
    }

    /// 批量发送消息
    pub async fn send_batch_messages(&self, conversation_id: &str, count: usize) -> Result<Value, reqwest::Error> {
        let messages: Vec<Value> = (0..count)
            .map(|_| self.text_message(conversation_id))
            .collect();
        
        let response = self.client
            .post(format!("{}/api/im/messages/batch", self.base_url))
            .header("Authorization", self.auth_header())
            .json(&json!({ "messages": messages }))
            .send()
            .await?;
        
        response.json().await
    }

    // ========== 会话相关 ==========

    /// 生成会话创建数据
    pub fn conversation_data(&self) -> Value {
        json!({
            "type": "direct",
            "participantIds": [Uuid::new_v4().to_string()],
            "name": null,
            "metadata": {}
        })
    }

    /// 创建会话
    pub async fn create_conversation(&self) -> Result<Value, reqwest::Error> {
        let data = self.conversation_data();
        let response = self.client
            .post(format!("{}/api/im/conversations", self.base_url))
            .header("Authorization", self.auth_header())
            .json(&data)
            .send()
            .await?;
        
        response.json().await
    }

    /// 获取会话列表
    pub async fn get_conversations(&self) -> Result<Value, reqwest::Error> {
        let response = self.client
            .get(format!("{}/api/im/conversations", self.base_url))
            .header("Authorization", self.auth_header())
            .send()
            .await?;
        
        response.json().await
    }

    // ========== 文件相关 ==========

    /// 上传测试文件
    pub async fn upload_test_file(&self) -> Result<Value, reqwest::Error> {
        // 创建临时测试文件
        let file_content = b"Test file content for integration test";
        let part = reqwest::multipart::Part::bytes(file_content.to_vec())
            .file_name("test.txt")
            .mime_str("text/plain").unwrap();
        
        let form = reqwest::multipart::Form::new()
            .part("file", part);
        
        let response = self.client
            .post(format!("{}/api/files/upload", self.base_url))
            .header("Authorization", self.auth_header())
            .multipart(form)
            .send()
            .await?;
        
        response.json().await
    }

    // ========== 加密相关 ==========

    /// 生成密钥对
    pub fn key_pair_data(&self) -> Value {
        json!({
            "keyType": "identity",
            "publicKey": base64::engine::general_purpose::STANDARD.encode(Uuid::new_v4().as_bytes()),
            "keyVersion": 1
        })
    }

    /// 注册公钥
    pub async fn register_public_key(&self) -> Result<Value, reqwest::Error> {
        let data = self.key_pair_data();
        let response = self.client
            .post(format!("{}/api/im/encryption/register-key", self.base_url))
            .header("Authorization", self.auth_header())
            .json(&data)
            .send()
            .await?;
        
        response.json().await
    }

    // ========== 用户状态相关 ==========

    /// 更新用户状态
    pub async fn update_presence(&self, status: &str) -> Result<Value, reqwest::Error> {
        let response = self.client
            .post(format!("{}/api/im/presence", self.base_url))
            .header("Authorization", self.auth_header())
            .json(&json!({
                "status": status,
                "statusText": "测试状态"
            }))
            .send()
            .await?;
        
        response.json().await
    }

    /// 获取用户在线状态
    pub async fn get_user_presence(&self, user_id: &str) -> Result<Value, reqwest::Error> {
        let response = self.client
            .get(format!("{}/api/im/presence/{}", self.base_url, user_id))
            .header("Authorization", self.auth_header())
            .send()
            .await?;
        
        response.json().await
    }

    // ========== 工具方法 ==========

    /// 等待指定时间
    pub async fn wait_ms(ms: u64) {
        tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
    }

    /// 生成随机字符串
    pub fn random_string(len: usize) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..len)
            .map(|_| rng.gen_range(b'a'..=b'z') as char)
            .collect()
    }

    /// 生成时间戳
    pub fn timestamp() -> String {
        Utc::now().to_rfc3339()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_creation() {
        let factory = TestFactory::new();
        assert!(!factory.base_url.is_empty());
    }

    #[test]
    fn test_random_string() {
        let s1 = TestFactory::random_string(10);
        let s2 = TestFactory::random_string(10);
        assert_eq!(s1.len(), 10);
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_user_data_generation() {
        let factory = TestFactory::new();
        let data = factory.user_registration_data();
        assert!(data.get("username").is_some());
        assert!(data.get("email").is_some());
        assert!(data.get("password").is_some());
    }
}
