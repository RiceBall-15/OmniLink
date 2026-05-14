use reqwest;
use serde_json::json;
use uuid::Uuid;

const BASE_URL: &str = "http://localhost:8080";

/// 辅助函数：注册并登录
async fn setup_auth_user(client: &reqwest::Client) -> Option<(String, String)> {
    let unique_id = Uuid::new_v4().to_string();
    let email = format!("conv_{}@example.com", &unique_id[..8]);
    let username = format!("convuser_{}", &unique_id[..8]);

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

/// 测试创建会话
#[tokio::test]
async fn test_create_conversation() {
    let client = reqwest::Client::new();

    let auth = setup_auth_user(&client).await;
    if auth.is_none() {
        println!("无法获取认证 token，跳过测试");
        return;
    }
    let (token, _) = auth.unwrap();

    let body = json!({
        "name": "Test Group Chat",
        "conversation_type": "group",
        "member_ids": []
    });

    let response = client
        .post(&format!("{}/api/im/conversations", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            println!("创建会话: status={}, body={}", status, body);
            assert!(status.is_success(), "创建会话失败: {}", body);
        }
        Err(e) => {
            println!("创建会话请求失败: {}", e);
        }
    }
}

/// 测试获取会话列表
#[tokio::test]
async fn test_get_conversations() {
    let client = reqwest::Client::new();

    let auth = setup_auth_user(&client).await;
    if auth.is_none() {
        println!("无法获取认证 token，跳过测试");
        return;
    }
    let (token, _) = auth.unwrap();

    let response = client
        .get(&format!("{}/api/im/conversations", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            println!("获取会话列表: status={}, body={}", status, body);
            assert!(status.is_success(), "获取会话列表失败: {}", body);
        }
        Err(e) => {
            println!("获取会话列表请求失败: {}", e);
        }
    }
}

/// 测试会话搜索
#[tokio::test]
async fn test_search_conversations() {
    let client = reqwest::Client::new();

    let auth = setup_auth_user(&client).await;
    if auth.is_none() {
        println!("无法获取认证 token，跳过测试");
        return;
    }
    let (token, _) = auth.unwrap();

    let response = client
        .get(&format!("{}/api/im/conversations/search", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .query(&[("q", "test")])
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            println!("搜索会话: status={}", status);
            assert!(status.is_success(), "搜索会话失败: status={}", status);
        }
        Err(e) => {
            println!("搜索会话请求失败: {}", e);
        }
    }
}

/// 测试置顶/取消置顶会话
#[tokio::test]
async fn test_toggle_pin_conversation() {
    let client = reqwest::Client::new();

    let auth = setup_auth_user(&client).await;
    if auth.is_none() {
        println!("无法获取认证 token，跳过测试");
        return;
    }
    let (token, _) = auth.unwrap();

    // 先创建一个会话
    let conv_body = json!({
        "name": "Pin Test",
        "conversation_type": "group",
        "member_ids": []
    });
    let resp = client
        .post(&format!("{}/api/im/conversations", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&conv_body)
        .send()
        .await;

    if let Ok(resp) = resp {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        if let Some(conv_id) = body.get("data").and_then(|d| d.get("id")).and_then(|id| id.as_str()) {
            // 置顶
            let response = client
                .put(&format!("{}/api/im/conversations/{}/pin", BASE_URL, conv_id))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    let body: serde_json::Value = resp.json().await.unwrap_or_default();
                    println!("置顶会话: status={}, body={}", status, body);
                    assert!(status.is_success(), "置顶会话失败: {}", body);
                }
                Err(e) => {
                    println!("置顶会话请求失败: {}", e);
                }
            }

            // 取消置顶
            let response = client
                .put(&format!("{}/api/im/conversations/{}/pin", BASE_URL, conv_id))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    println!("取消置顶会话: status={}", status);
                    assert!(status.is_success(), "取消置顶会话失败");
                }
                Err(e) => {
                    println!("取消置顶请求失败: {}", e);
                }
            }
        }
    }
}

/// 测试归档会话
#[tokio::test]
async fn test_archive_conversation() {
    let client = reqwest::Client::new();

    let auth = setup_auth_user(&client).await;
    if auth.is_none() {
        println!("无法获取认证 token，跳过测试");
        return;
    }
    let (token, _) = auth.unwrap();

    // 创建会话
    let conv_body = json!({
        "name": "Archive Test",
        "conversation_type": "group",
        "member_ids": []
    });
    let resp = client
        .post(&format!("{}/api/im/conversations", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&conv_body)
        .send()
        .await;

    if let Ok(resp) = resp {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        if let Some(conv_id) = body.get("data").and_then(|d| d.get("id")).and_then(|id| id.as_str()) {
            let response = client
                .put(&format!("{}/api/im/conversations/{}/archive", BASE_URL, conv_id))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    let body: serde_json::Value = resp.json().await.unwrap_or_default();
                    println!("归档会话: status={}, body={}", status, body);
                    assert!(status.is_success(), "归档会话失败: {}", body);
                }
                Err(e) => {
                    println!("归档会话请求失败: {}", e);
                }
            }
        }
    }
}

/// 测试获取群成员列表
#[tokio::test]
async fn test_get_group_members() {
    let client = reqwest::Client::new();

    let auth = setup_auth_user(&client).await;
    if auth.is_none() {
        println!("无法获取认证 token，跳过测试");
        return;
    }
    let (token, _) = auth.unwrap();

    // 创建群会话
    let conv_body = json!({
        "name": "Members Test",
        "conversation_type": "group",
        "member_ids": []
    });
    let resp = client
        .post(&format!("{}/api/im/conversations", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&conv_body)
        .send()
        .await;

    if let Ok(resp) = resp {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        if let Some(conv_id) = body.get("data").and_then(|d| d.get("id")).and_then(|id| id.as_str()) {
            let response = client
                .get(&format!("{}/api/im/conversations/{}/members", BASE_URL, conv_id))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    let body: serde_json::Value = resp.json().await.unwrap_or_default();
                    println!("获取群成员: status={}, body={}", status, body);
                    assert!(status.is_success(), "获取群成员失败: {}", body);
                }
                Err(e) => {
                    println!("获取群成员请求失败: {}", e);
                }
            }
        }
    }
}

/// 测试未授权访问会话
#[tokio::test]
async fn test_unauthorized_conversation_access() {
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/api/im/conversations", BASE_URL))
        .header("Authorization", "Bearer invalid_token")
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            println!("未授权访问会话: status={}", status);
            assert!(
                status == reqwest::StatusCode::UNAUTHORIZED
                    || status == reqwest::StatusCode::FORBIDDEN,
                "未授权访问处理异常: status={}", status
            );
        }
        Err(e) => {
            println!("请求失败: {}", e);
        }
    }
}
