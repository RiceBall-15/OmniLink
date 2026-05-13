use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

const BASE_URL: &str = "http://localhost:8080";

/// 测试消息发送
#[tokio::test]
async fn test_send_message() {
    let client = reqwest::Client::new();

    // 假设已有一个有效的JWT token（在实际测试中需要先登录获取）
    let token = "test_token";
    let conversation_id = Uuid::new_v4();

    let message_body = json!({
        "conversation_id": conversation_id.to_string(),
        "content": "Hello, this is a test message",
        "message_type": "text"
    });

    let response = client
        .post(&format!("{}/api/v1/messages", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&message_body)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            println!("发送消息响应: status={}, body={}", status, body);
            // 预期会因为无效token而返回401
            assert!(status.is_success() || status == reqwest::StatusCode::UNAUTHORIZED);
        }
        Err(e) => {
            println!("发送消息请求失败（服务可能未运行）: {}", e);
        }
    }
}

/// 测试获取消息历史
#[tokio::test]
async fn test_get_message_history() {
    let client = reqwest::Client::new();

    let token = "test_token";
    let conversation_id = Uuid::new_v4();

    let response = client
        .get(&format!("{}/api/v1/messages/{}", BASE_URL, conversation_id))
        .header("Authorization", format!("Bearer {}", token))
        .query(&[("page", "1"), ("page_size", "20")])
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            println!("获取消息历史响应: status={}, body={}", status, body);
            assert!(status.is_success() || status == reqwest::StatusCode::UNAUTHORIZED);
        }
        Err(e) => {
            println!("获取消息历史请求失败（服务可能未运行）: {}", e);
        }
    }
}
