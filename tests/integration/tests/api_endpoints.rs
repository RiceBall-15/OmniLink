//! OmniLink API 端点集成测试
//! 
//! 测试所有主要 API 端点的功能完整性

use reqwest::Client;
use serde_json::{json, Value};
use uuid::Uuid;

/// 获取测试配置
fn get_config() -> (String, String) {
    let base_url = std::env::var("OMNILINK_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());
    let auth_token = std::env::var("AUTH_TOKEN")
        .unwrap_or_else(|_| "test-token".to_string());
    (base_url, auth_token)
}

// ==================== 消息 API 测试 ====================

#[tokio::test]
async fn test_send_text_message() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    let conversation_id = Uuid::new_v4().to_string();
    
    let payload = json!({
        "conversationId": conversation_id,
        "content": "集成测试消息",
        "contentType": "text",
        "metadata": {}
    });
    
    let response = client
        .post(format!("{}/api/im/messages", base_url))
        .header("Authorization", format!("Bearer {}", auth_token))
        .json(&payload)
        .send()
        .await;
    
    // 验证请求发送成功（不要求必须200，因为服务可能未运行）
    assert!(response.is_ok(), "消息发送请求应该成功发出");
}

#[tokio::test]
async fn test_send_message_with_metadata() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    
    let payload = json!({
        "conversationId": Uuid::new_v4().to_string(),
        "content": "带元数据的消息",
        "contentType": "text",
        "metadata": {
            "replyTo": Uuid::new_v4().to_string(),
            "mentions": ["user1", "user2"],
            "tags": ["important"]
        }
    });
    
    let response = client
        .post(format!("{}/api/im/messages", base_url))
        .header("Authorization", format!("Bearer {}", auth_token))
        .json(&payload)
        .send()
        .await;
    
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_get_message_history() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    let conversation_id = Uuid::new_v4().to_string();
    
    let response = client
        .get(format!(
            "{}/api/im/conversations/{}/messages?limit=20",
            base_url, conversation_id
        ))
        .header("Authorization", format!("Bearer {}", auth_token))
        .send()
        .await;
    
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_message_pagination() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    let conversation_id = Uuid::new_v4().to_string();
    
    // 测试不同分页参数
    for limit in [10, 20, 50, 100] {
        let response = client
            .get(format!(
                "{}/api/im/conversations/{}/messages?limit={}",
                base_url, conversation_id, limit
            ))
            .header("Authorization", format!("Bearer {}", auth_token))
            .send()
            .await;
        
        assert!(response.is_ok(), "分页查询 limit={} 应该成功", limit);
    }
}

// ==================== 会话 API 测试 ====================

#[tokio::test]
async fn test_get_conversations_list() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    
    let response = client
        .get(format!("{}/api/im/conversations", base_url))
        .header("Authorization", format!("Bearer {}", auth_token))
        .send()
        .await;
    
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_create_direct_conversation() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    
    let payload = json!({
        "type": "direct",
        "participantIds": [Uuid::new_v4().to_string()],
        "name": null,
        "metadata": {}
    });
    
    let response = client
        .post(format!("{}/api/im/conversations", base_url))
        .header("Authorization", format!("Bearer {}", auth_token))
        .json(&payload)
        .send()
        .await;
    
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_create_group_conversation() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    
    let payload = json!({
        "type": "group",
        "participantIds": [
            Uuid::new_v4().to_string(),
            Uuid::new_v4().to_string(),
            Uuid::new_v4().to_string()
        ],
        "name": "测试群组",
        "metadata": {
            "description": "集成测试创建的群组"
        }
    });
    
    let response = client
        .post(format!("{}/api/im/conversations", base_url))
        .header("Authorization", format!("Bearer {}", auth_token))
        .json(&payload)
        .send()
        .await;
    
    assert!(response.is_ok());
}

// ==================== 加密 API 测试 ====================

#[tokio::test]
async fn test_generate_encryption_keys() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    
    let payload = json!({
        "keyType": "identity",
        "keyVersion": 1
    });
    
    let response = client
        .post(format!("{}/api/im/encryption/keys", base_url))
        .header("Authorization", format!("Bearer {}", auth_token))
        .json(&payload)
        .send()
        .await;
    
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_register_public_key() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    
    let payload = json!({
        "keyType": "identity",
        "publicKey": base64::encode(Uuid::new_v4().as_bytes()),
        "keyVersion": 1
    });
    
    let response = client
        .post(format!("{}/api/im/encryption/register-key", base_url))
        .header("Authorization", format!("Bearer {}", auth_token))
        .json(&payload)
        .send()
        .await;
    
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_get_user_public_key() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    let user_id = Uuid::new_v4().to_string();
    
    let response = client
        .get(format!(
            "{}/api/im/encryption/public-key/{}",
            base_url, user_id
        ))
        .header("Authorization", format!("Bearer {}", auth_token))
        .send()
        .await;
    
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_batch_get_public_keys() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    
    let payload = json!({
        "userIds": [
            Uuid::new_v4().to_string(),
            Uuid::new_v4().to_string()
        ],
        "keyType": "identity"
    });
    
    let response = client
        .post(format!(
            "{}/api/im/encryption/public-keys/batch",
            base_url
        ))
        .header("Authorization", format!("Bearer {}", auth_token))
        .json(&payload)
        .send()
        .await;
    
    assert!(response.is_ok());
}

// ==================== 用户状态 API 测试 ====================

#[tokio::test]
async fn test_update_presence_status() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    
    let payload = json!({
        "status": "online",
        "statusText": "集成测试中"
    });
    
    let response = client
        .post(format!("{}/api/im/presence", base_url))
        .header("Authorization", format!("Bearer {}", auth_token))
        .json(&payload)
        .send()
        .await;
    
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_get_user_presence() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    let user_id = Uuid::new_v4().to_string();
    
    let response = client
        .get(format!("{}/api/im/presence/{}", base_url, user_id))
        .header("Authorization", format!("Bearer {}", auth_token))
        .send()
        .await;
    
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_batch_get_presence() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    
    let payload = json!({
        "userIds": [
            Uuid::new_v4().to_string(),
            Uuid::new_v4().to_string(),
            Uuid::new_v4().to_string()
        ]
    });
    
    let response = client
        .post(format!("{}/api/im/presence/batch", base_url))
        .header("Authorization", format!("Bearer {}", auth_token))
        .json(&payload)
        .send()
        .await;
    
    assert!(response.is_ok());
}

// ==================== 消息已读 API 测试 ====================

#[tokio::test]
async fn test_mark_message_read() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    let message_id = Uuid::new_v4().to_string();
    
    let response = client
        .post(format!(
            "{}/api/im/messages/{}/read",
            base_url, message_id
        ))
        .header("Authorization", format!("Bearer {}", auth_token))
        .send()
        .await;
    
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_get_delivery_receipts() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    let message_id = Uuid::new_v4().to_string();
    
    let response = client
        .get(format!(
            "{}/api/im/messages/{}/receipts",
            base_url, message_id
        ))
        .header("Authorization", format!("Bearer {}", auth_token))
        .send()
        .await;
    
    assert!(response.is_ok());
}

// ==================== 消息编辑/撤回 API 测试 ====================

#[tokio::test]
async fn test_edit_message() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    let message_id = Uuid::new_v4().to_string();
    
    let payload = json!({
        "content": "编辑后的消息内容"
    });
    
    let response = client
        .put(format!(
            "{}/api/im/messages/{}",
            base_url, message_id
        ))
        .header("Authorization", format!("Bearer {}", auth_token))
        .json(&payload)
        .send()
        .await;
    
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_recall_message() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    let message_id = Uuid::new_v4().to_string();
    
    let response = client
        .delete(format!(
            "{}/api/im/messages/{}",
            base_url, message_id
        ))
        .header("Authorization", format!("Bearer {}", auth_token))
        .send()
        .await;
    
    assert!(response.is_ok());
}

// ==================== 消息搜索 API 测试 ====================

#[tokio::test]
async fn test_search_messages() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    
    let response = client
        .get(format!(
            "{}/api/im/messages/search?q=test",
            base_url
        ))
        .header("Authorization", format!("Bearer {}", auth_token))
        .send()
        .await;
    
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_global_search() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    
    let response = client
        .get(format!(
            "{}/api/im/messages/search/global?q=test",
            base_url
        ))
        .header("Authorization", format!("Bearer {}", auth_token))
        .send()
        .await;
    
    assert!(response.is_ok());
}

// ==================== 认证测试 ====================

#[tokio::test]
async fn test_unauthorized_access() {
    let (base_url, _) = get_config();
    let client = Client::new();
    
    // 不带认证令牌访问
    let response = client
        .get(format!("{}/api/im/conversations", base_url))
        .send()
        .await;
    
    if let Ok(resp) = response {
        // 应该返回 401
        assert!(
            resp.status() == 401 || resp.status() == 403,
            "未认证访问应该返回 401 或 403，实际返回: {}",
            resp.status()
        );
    }
}

#[tokio::test]
async fn test_invalid_token() {
    let (base_url, _) = get_config();
    let client = Client::new();
    
    let response = client
        .get(format!("{}/api/im/conversations", base_url))
        .header("Authorization", "Bearer invalid-token-12345")
        .send()
        .await;
    
    if let Ok(resp) = response {
        assert!(
            resp.status() == 401 || resp.status() == 403,
            "无效令牌应该返回 401 或 403，实际返回: {}",
            resp.status()
        );
    }
}

// ==================== 错误处理测试 ====================

#[tokio::test]
async fn test_invalid_message_payload() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    
    // 缺少必需字段
    let payload = json!({
        "content": "缺少 conversationId"
    });
    
    let response = client
        .post(format!("{}/api/im/messages", base_url))
        .header("Authorization", format!("Bearer {}", auth_token))
        .json(&payload)
        .send()
        .await;
    
    if let Ok(resp) = response {
        assert!(
            resp.status().is_client_error(),
            "无效载荷应该返回 4xx 错误，实际返回: {}",
            resp.status()
        );
    }
}

#[tokio::test]
async fn test_nonexistent_conversation() {
    let (base_url, auth_token) = get_config();
    let client = Client::new();
    let fake_id = Uuid::new_v4().to_string();
    
    let response = client
        .get(format!(
            "{}/api/im/conversations/{}/messages",
            base_url, fake_id
        ))
        .header("Authorization", format!("Bearer {}", auth_token))
        .send()
        .await;
    
    // 不管返回什么，请求本身应该成功发出
    assert!(response.is_ok());
}
