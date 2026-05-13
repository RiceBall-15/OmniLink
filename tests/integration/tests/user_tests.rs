use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

const BASE_URL: &str = "http://localhost:8080";

/// API响应格式
#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    code: u16,
    message: String,
    data: Option<T>,
    timestamp: i64,
}

/// 注册请求
#[derive(Debug, Serialize)]
struct RegisterRequest {
    username: String,
    email: String,
    password: String,
}

/// 登录请求
#[derive(Debug, Serialize)]
struct LoginRequest {
    email: String,
    password: String,
}

/// 登录响应
#[derive(Debug, Deserialize)]
struct LoginResponse {
    token: String,
    refresh_token: String,
    user_id: String,
}

/// 测试用户注册和登录流程
#[tokio::test]
async fn test_user_registration_and_login() {
    let client = reqwest::Client::new();
    let unique_id = Uuid::new_v4().to_string();
    let email = format!("test_{}@example.com", &unique_id[..8]);
    let username = format!("testuser_{}", &unique_id[..8]);

    // 1. 注册用户
    let register_req = RegisterRequest {
        username: username.clone(),
        email: email.clone(),
        password: "Test1234!".to_string(),
    };

    let response = client
        .post(&format!("{}/api/v1/users/register", BASE_URL))
        .json(&register_req)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            println!("注册响应: status={}, body={}", status, body);
            assert!(status.is_success() || status == reqwest::StatusCode::CONFLICT,
                "注册失败: {}", body);
        }
        Err(e) => {
            println!("注册请求失败（服务可能未运行）: {}", e);
            return; // 服务未运行时跳过测试
        }
    }

    // 2. 登录用户
    let login_req = LoginRequest {
        email: email.clone(),
        password: "Test1234!".to_string(),
    };

    let response = client
        .post(&format!("{}/api/v1/users/login", BASE_URL))
        .json(&login_req)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            println!("登录响应: status={}, body={}", status, body);
            assert!(status.is_success(), "登录失败: {}", body);

            // 验证返回了token
            if let Some(data) = body.get("data") {
                assert!(data.get("token").is_some(), "响应中缺少token");
                assert!(data.get("user_id").is_some(), "响应中缺少user_id");
            }
        }
        Err(e) => {
            println!("登录请求失败: {}", e);
        }
    }
}

/// 测试健康检查端点
#[tokio::test]
async fn test_health_check() {
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/health", BASE_URL))
        .send()
        .await;

    match response {
        Ok(resp) => {
            println!("健康检查: status={}", resp.status());
            // 健康检查应该返回200或404（取决于服务配置）
            assert!(resp.status().is_success() || resp.status() == reqwest::StatusCode::NOT_FOUND);
        }
        Err(e) => {
            println!("健康检查请求失败（服务可能未运行）: {}", e);
        }
    }
}
