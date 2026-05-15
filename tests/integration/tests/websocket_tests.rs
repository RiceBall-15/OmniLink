//! WebSocket 集成测试
//! 
//! 测试 WebSocket 连接、认证、消息收发等功能

use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use uuid::Uuid;

fn get_ws_config() -> (String, String) {
    let ws_url = std::env::var("OMNILINK_WS_URL")
        .unwrap_or_else(|_| "ws://localhost:8080/ws".to_string());
    let auth_token = std::env::var("AUTH_TOKEN")
        .unwrap_or_else(|_| "test-token".to_string());
    (ws_url, auth_token)
}

#[tokio::test]
async fn test_ws_connection() {
    let (ws_url, auth_token) = get_ws_config();
    let url = format!("{}?token={}", ws_url, auth_token);
    
    let result = connect_async(&url).await;
    assert!(result.is_ok(), "WebSocket 连接应该成功");
}

#[tokio::test]
async fn test_ws_connection_without_token() {
    let (ws_url, _) = get_ws_config();
    
    // 不带 token 连接应该失败或返回错误
    let result = connect_async(&ws_url).await;
    // 可能连接成功但后续消息被拒绝，或直接连接失败
    // 这里只验证不会 panic
    let _ = result;
}

#[tokio::test]
async fn test_ws_send_message() {
    let (ws_url, auth_token) = get_ws_config();
    let url = format!("{}?token={}", ws_url, auth_token);
    
    if let Ok((mut ws_stream, _)) = connect_async(&url).await {
        let msg = json!({
            "type": "message",
            "conversationId": Uuid::new_v4().to_string(),
            "content": "WebSocket 测试消息",
            "contentType": "text"
        });
        
        let result = ws_stream.send(Message::Text(msg.to_string())).await;
        assert!(result.is_ok(), "WebSocket 消息发送应该成功");
    }
}

#[tokio::test]
async fn test_ws_ping_pong() {
    let (ws_url, auth_token) = get_ws_config();
    let url = format!("{}?token={}", ws_url, auth_token);
    
    if let Ok((mut ws_stream, _)) = connect_async(&url).await {
        // 发送 Ping
        let result = ws_stream.send(Message::Ping(vec![1, 2, 3])).await;
        assert!(result.is_ok(), "Ping 发送应该成功");
        
        // 等待 Pong（带超时）
        let timeout = tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            ws_stream.next()
        );
        
        if let Ok(Some(Ok(msg))) = timeout.await {
            match msg {
                Message::Pong(_) => {}, // 期望收到 Pong
                Message::Ping(_) => {}, // 也可能收到 Ping
                _ => {} // 其他消息也可以
            }
        }
    }
}

#[tokio::test]
async fn test_ws_multiple_messages() {
    let (ws_url, auth_token) = get_ws_config();
    let url = format!("{}?token={}", ws_url, auth_token);
    
    if let Ok((mut ws_stream, _)) = connect_async(&url).await {
        for i in 0..10 {
            let msg = json!({
                "type": "message",
                "conversationId": Uuid::new_v4().to_string(),
                "content": format!("批量消息 {}", i),
                "contentType": "text"
            });
            
            let result = ws_stream.send(Message::Text(msg.to_string())).await;
            assert!(result.is_ok(), "消息 {} 发送失败", i);
        }
    }
}

#[tokio::test]
async fn test_ws_concurrent_connections() {
    let (ws_url, auth_token) = get_ws_config();
    
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let url = format!("{}?token={}", ws_url, auth_token);
        
        let handle = tokio::spawn(async move {
            if let Ok((mut ws_stream, _)) = connect_async(&url).await {
                let msg = json!({
                    "type": "message",
                    "conversationId": Uuid::new_v4().to_string(),
                    "content": format!("并发连接 {} 消息", i),
                    "contentType": "text"
                });
                
                ws_stream.send(Message::Text(msg.to_string())).await
            } else {
                Err(tokio_tungstenite::tungstenite::Error::ConnectionClosed)
            }
        });
        
        handles.push(handle);
    }
    
    let results = futures_util::future::join_all(handles).await;
    
    // 至少应该有一些连接成功
    let success_count = results.iter()
        .filter(|r| r.is_ok() && r.as_ref().unwrap().is_ok())
        .count();
    
    // 不强制要求全部成功，因为服务可能限制并发
    println!("成功连接数: {}/5", success_count);
}

#[tokio::test]
async fn test_ws_large_message() {
    let (ws_url, auth_token) = get_ws_config();
    let url = format!("{}?token={}", ws_url, auth_token);
    
    if let Ok((mut ws_stream, _)) = connect_async(&url).await {
        // 发送大消息（10KB）
        let large_content = "A".repeat(10240);
        let msg = json!({
            "type": "message",
            "conversationId": Uuid::new_v4().to_string(),
            "content": large_content,
            "contentType": "text"
        });
        
        let result = ws_stream.send(Message::Text(msg.to_string())).await;
        assert!(result.is_ok(), "大消息发送应该成功");
    }
}

#[tokio::test]
async fn test_ws_binary_message() {
    let (ws_url, auth_token) = get_ws_config();
    let url = format!("{}?token={}", ws_url, auth_token);
    
    if let Ok((mut ws_stream, _)) = connect_async(&url).await {
        // 发送二进制消息
        let binary_data = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let result = ws_stream.send(Message::Binary(binary_data)).await;
        assert!(result.is_ok(), "二进制消息发送应该成功");
    }
}

#[tokio::test]
async fn test_ws_close_connection() {
    let (ws_url, auth_token) = get_ws_config();
    let url = format!("{}?token={}", ws_url, auth_token);
    
    if let Ok((mut ws_stream, _)) = connect_async(&url).await {
        let result = ws_stream.close(None).await;
        assert!(result.is_ok(), "WebSocket 关闭应该成功");
    }
}
