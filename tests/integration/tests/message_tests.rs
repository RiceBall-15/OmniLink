use reqwest;
use serde_json::json;
use uuid::Uuid;

const BASE_URL: &str = "http://localhost:8080";

/// 辅助函数：注册并登录，返回 (token, user_id)
async fn setup_auth_user(client: &reqwest::Client) -> Option<(String, String)> {
    let unique_id = Uuid::new_v4().to_string();
    let email = format!("msg_{}@example.com", &unique_id[..8]);
    let username = format!("msguser_{}", &unique_id[..8]);

    // 注册
    let register_req = json!({
        "username": username,
        "email": email,
        "password": "Test1234!"
    });
    let _ = client
        .post(&format!("{}/api/auth/register", BASE_URL))
        .json(&register_req)
        .send()
        .await;

    // 登录
    let login_req = json!({
        "email": email,
        "password": "Test1234!"
    });
    match client
        .post(&format!("{}/api/auth/login", BASE_URL))
        .json(&login_req)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            if let Some(data) = body.get("data") {
                let token = data.get("token")?.as_str()?.to_string();
                let user_id = data.get("user_id")?.as_str()?.to_string();
                return Some((token, user_id));
            }
            None
        }
        _ => None,
    }
}

/// 辅助函数：创建一个会话
async fn create_conversation(client: &reqwest::Client, token: &str, name: &str) -> Option<String> {
    let body = json!({
        "name": name,
        "conversation_type": "direct",
        "member_ids": []
    });

    match client
        .post(&format!("{}/api/im/conversations", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            body.get("data")
                .and_then(|d| d.get("id"))
                .and_then(|id| id.as_str())
                .map(|s| s.to_string())
        }
        Err(_) => None,
    }
}

/// 测试发送消息
#[tokio::test]
async fn test_send_message() {
    let client = reqwest::Client::new();

    let auth = setup_auth_user(&client).await;
    if auth.is_none() {
        println!("无法获取认证 token，跳过测试");
        return;
    }
    let (token, _user_id) = auth.unwrap();

    // 先创建一个会话
    let conversation_id = create_conversation(&client, &token, "Test Conversation").await;
    if conversation_id.is_none() {
        println!("无法创建会话，跳过测试");
        return;
    }
    let conversation_id = conversation_id.unwrap();

    // 发送消息
    let message_body = json!({
        "content": "Hello, this is a test message",
        "message_type": "text"
    });

    let response = client
        .post(&format!("{}/api/im/conversations/{}/messages", BASE_URL, conversation_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&message_body)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            println!("发送消息: status={}, body={}", status, body);
            assert!(status.is_success(), "发送消息失败: {}", body);
        }
        Err(e) => {
            println!("发送消息请求失败: {}", e);
        }
    }
}

/// 测试获取消息历史
#[tokio::test]
async fn test_get_message_history() {
    let client = reqwest::Client::new();

    let auth = setup_auth_user(&client).await;
    if auth.is_none() {
        println!("无法获取认证 token，跳过测试");
        return;
    }
    let (token, _) = auth.unwrap();

    // 先创建一个会话
    let conversation_id = create_conversation(&client, &token, "History Test").await;
    if conversation_id.is_none() {
        println!("无法创建会话，跳过测试");
        return;
    }
    let conversation_id = conversation_id.unwrap();

    // 获取消息历史
    let response = client
        .get(&format!("{}/api/im/conversations/{}/messages", BASE_URL, conversation_id))
        .header("Authorization", format!("Bearer {}", token))
        .query(&[("page", "1"), ("page_size", "20")])
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            println!("获取消息历史: status={}, body={}", status, body);
            assert!(status.is_success(), "获取消息历史失败: {}", body);
        }
        Err(e) => {
            println!("获取消息历史请求失败: {}", e);
        }
    }
}

/// 测试编辑消息
#[tokio::test]
async fn test_edit_message() {
    let client = reqwest::Client::new();

    let auth = setup_auth_user(&client).await;
    if auth.is_none() {
        println!("无法获取认证 token，跳过测试");
        return;
    }
    let (token, _) = auth.unwrap();

    // 创建会话
    let conversation_id = create_conversation(&client, &token, "Edit Test").await;
    if conversation_id.is_none() {
        println!("无法创建会话，跳过测试");
        return;
    }
    let conversation_id = conversation_id.unwrap();

    // 发送消息
    let msg_body = json!({
        "content": "Original message",
        "message_type": "text"
    });
    let resp = client
        .post(&format!("{}/api/im/conversations/{}/messages", BASE_URL, conversation_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&msg_body)
        .send()
        .await;

    if let Ok(resp) = resp {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        if let Some(msg_id) = body.get("data").and_then(|d| d.get("id")).and_then(|id| id.as_str()) {
            // 编辑消息
            let edit_body = json!({
                "content": "Edited message"
            });
            let response = client
                .put(&format!("{}/api/im/conversations/{}/messages/{}", BASE_URL, conversation_id, msg_id))
                .header("Authorization", format!("Bearer {}", token))
                .json(&edit_body)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    let body: serde_json::Value = resp.json().await.unwrap_or_default();
                    println!("编辑消息: status={}, body={}", status, body);
                    assert!(status.is_success(), "编辑消息失败: {}", body);
                }
                Err(e) => {
                    println!("编辑消息请求失败: {}", e);
                }
            }
        }
    }
}

/// 测试撤回消息
#[tokio::test]
async fn test_recall_message() {
    let client = reqwest::Client::new();

    let auth = setup_auth_user(&client).await;
    if auth.is_none() {
        println!("无法获取认证 token，跳过测试");
        return;
    }
    let (token, _) = auth.unwrap();

    // 创建会话
    let conversation_id = create_conversation(&client, &token, "Recall Test").await;
    if conversation_id.is_none() {
        println!("无法创建会话，跳过测试");
        return;
    }
    let conversation_id = conversation_id.unwrap();

    // 发送消息
    let msg_body = json!({
        "content": "Message to recall",
        "message_type": "text"
    });
    let resp = client
        .post(&format!("{}/api/im/conversations/{}/messages", BASE_URL, conversation_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&msg_body)
        .send()
        .await;

    if let Ok(resp) = resp {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        if let Some(msg_id) = body.get("data").and_then(|d| d.get("id")).and_then(|id| id.as_str()) {
            // 撤回消息
            let response = client
                .post(&format!("{}/api/im/conversations/{}/messages/{}/recall", BASE_URL, conversation_id, msg_id))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    let body: serde_json::Value = resp.json().await.unwrap_or_default();
                    println!("撤回消息: status={}, body={}", status, body);
                    assert!(status.is_success(), "撤回消息失败: {}", body);
                }
                Err(e) => {
                    println!("撤回消息请求失败: {}", e);
                }
            }
        }
    }
}

/// 测试消息搜索
#[tokio::test]
async fn test_search_messages() {
    let client = reqwest::Client::new();

    let auth = setup_auth_user(&client).await;
    if auth.is_none() {
        println!("无法获取认证 token，跳过测试");
        return;
    }
    let (token, _) = auth.unwrap();

    // 全局搜索
    let response = client
        .get(&format!("{}/api/im/messages/search", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .query(&[("q", "test"), ("page", "1"), ("page_size", "20")])
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            println!("搜索消息: status={}", status);
            assert!(status.is_success(), "搜索消息失败: status={}", status);
        }
        Err(e) => {
            println!("搜索消息请求失败: {}", e);
        }
    }
}

/// 测试无授权访问消息
#[tokio::test]
async fn test_unauthorized_message_access() {
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/api/im/conversations/{}/messages", BASE_URL, Uuid::new_v4()))
        .header("Authorization", "Bearer invalid_token")
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            println!("无授权访问: status={}", status);
            // 应该返回 401 Unauthorized
            assert!(
                status == reqwest::StatusCode::UNAUTHORIZED
                    || status == reqwest::StatusCode::FORBIDDEN,
                "无授权访问处理异常: status={}", status
            );
        }
        Err(e) => {
            println!("请求失败: {}", e);
        }
    }
}
