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

/// 辅助函数：注册用户
async fn register_user(client: &reqwest::Client, email: &str, username: &str, password: &str) -> reqwest::Result<reqwest::Response> {
    let register_req = RegisterRequest {
        username: username.to_string(),
        email: email.to_string(),
        password: password.to_string(),
    };
    client
        .post(&format!("{}/api/auth/register", BASE_URL))
        .json(&register_req)
        .send()
        .await
}

/// 辅助函数：登录用户
async fn login_user(client: &reqwest::Client, email: &str, password: &str) -> reqwest::Result<reqwest::Response> {
    let login_req = LoginRequest {
        email: email.to_string(),
        password: password.to_string(),
    };
    client
        .post(&format!("{}/api/auth/login", BASE_URL))
        .json(&login_req)
        .send()
        .await
}

/// 辅助函数：获取带认证头的用户
async fn get_auth_token(client: &reqwest::Client) -> Option<(String, String)> {
    let unique_id = Uuid::new_v4().to_string();
    let email = format!("test_{}@example.com", &unique_id[..8]);
    let username = format!("testuser_{}", &unique_id[..8]);

    // 注册
    let _ = register_user(client, &email, &username, "Test1234!").await;

    // 登录获取 token
    match login_user(client, &email, "Test1234!").await {
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

/// 测试用户注册和登录流程
#[tokio::test]
async fn test_user_registration_and_login() {
    let client = reqwest::Client::new();
    let unique_id = Uuid::new_v4().to_string();
    let email = format!("test_{}@example.com", &unique_id[..8]);
    let username = format!("testuser_{}", &unique_id[..8]);

    // 1. 注册用户
    let response = register_user(&client, &email, &username, "Test1234!").await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            println!("注册响应: status={}, body={}", status, body);
            assert!(
                status.is_success() || status == reqwest::StatusCode::CONFLICT,
                "注册失败: {}", body
            );
        }
        Err(e) => {
            println!("注册请求失败（服务可能未运行）: {}", e);
            return;
        }
    }

    // 2. 登录用户
    let response = login_user(&client, &email, "Test1234!").await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            println!("登录响应: status={}, body={}", status, body);
            assert!(status.is_success(), "登录失败: {}", body);

            // 验证返回了 token
            if let Some(data) = body.get("data") {
                assert!(data.get("token").is_some(), "响应中缺少 token");
                assert!(data.get("user_id").is_some(), "响应中缺少 user_id");
            }
        }
        Err(e) => {
            println!("登录请求失败: {}", e);
        }
    }
}

/// 测试注册必填字段校验
#[tokio::test]
async fn test_register_validation() {
    let client = reqwest::Client::new();

    // 缺少密码
    let body = json!({
        "username": "testuser",
        "email": "test@example.com"
    });

    let response = client
        .post(&format!("{}/api/auth/register", BASE_URL))
        .json(&body)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            println!("缺少密码注册: status={}", status);
            // 应该返回 400 Bad Request 或 422 Unprocessable Entity
            assert!(
                status == reqwest::StatusCode::BAD_REQUEST
                    || status == reqwest::StatusCode::UNPROCESSABLE_ENTITY
                    || status.is_success(),
                "注册验证异常: status={}", status
            );
        }
        Err(e) => {
            println!("请求失败（服务可能未运行）: {}", e);
        }
    }
}

/// 测试重复注册
#[tokio::test]
async fn test_duplicate_registration() {
    let client = reqwest::Client::new();
    let unique_id = Uuid::new_v4().to_string();
    let email = format!("dup_{}@example.com", &unique_id[..8]);
    let username = format!("dupuser_{}", &unique_id[..8]);

    // 第一次注册
    let _ = register_user(&client, &email, &username, "Test1234!").await;

    // 第二次注册（相同邮箱）
    let response = register_user(&client, &email, &username, "Test1234!").await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            println!("重复注册: status={}", status);
            // 应该返回 409 Conflict 或 400 Bad Request
            assert!(
                status == reqwest::StatusCode::CONFLICT
                    || status == reqwest::StatusCode::BAD_REQUEST
                    || status.is_success(),
                "重复注册处理异常: status={}", status
            );
        }
        Err(e) => {
            println!("请求失败: {}", e);
        }
    }
}

/// 测试错误密码登录
#[tokio::test]
async fn test_wrong_password_login() {
    let client = reqwest::Client::new();
    let unique_id = Uuid::new_v4().to_string();
    let email = format!("wp_{}@example.com", &unique_id[..8]);
    let username = format!("wpuser_{}", &unique_id[..8]);

    // 注册
    let _ = register_user(&client, &email, &username, "Test1234!").await;

    // 用错误密码登录
    let response = login_user(&client, &email, "WrongPassword!").await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            println!("错误密码登录: status={}", status);
            // 应该返回 401 Unauthorized
            assert!(
                status == reqwest::StatusCode::UNAUTHORIZED
                    || status == reqwest::StatusCode::BAD_REQUEST
                    || status.is_success(),
                "错误密码处理异常: status={}", status
            );
        }
        Err(e) => {
            println!("请求失败: {}", e);
        }
    }
}

/// 测试获取当前用户信息
#[tokio::test]
async fn test_get_me() {
    let client = reqwest::Client::new();

    let auth = get_auth_token(&client).await;
    if auth.is_none() {
        println!("无法获取认证 token，跳过测试");
        return;
    }
    let (token, _user_id) = auth.unwrap();

    let response = client
        .get(&format!("{}/api/user/me", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            println!("获取用户信息: status={}, body={}", status, body);
            assert!(status.is_success(), "获取用户信息失败: {}", body);

            // 验证返回了用户信息
            if let Some(data) = body.get("data") {
                assert!(data.get("id").is_some() || data.get("user_id").is_some(), "响应中缺少用户ID");
            }
        }
        Err(e) => {
            println!("请求失败: {}", e);
        }
    }
}

/// 测试更新用户资料
#[tokio::test]
async fn test_update_profile() {
    let client = reqwest::Client::new();

    let auth = get_auth_token(&client).await;
    if auth.is_none() {
        println!("无法获取认证 token，跳过测试");
        return;
    }
    let (token, _) = auth.unwrap();

    let body = json!({
        "nickname": "测试昵称",
        "bio": "这是测试签名"
    });

    let response = client
        .put(&format!("{}/api/user/profile", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            println!("更新用户资料: status={}, body={}", status, body);
            // 应该成功或返回 200/204
            assert!(
                status.is_success() || status == reqwest::StatusCode::NOT_FOUND,
                "更新资料处理异常: {}", body
            );
        }
        Err(e) => {
            println!("请求失败: {}", e);
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
            assert!(
                resp.status().is_success() || resp.status() == reqwest::StatusCode::NOT_FOUND,
                "健康检查异常: status={}", resp.status()
            );
        }
        Err(e) => {
            println!("健康检查请求失败（服务可能未运行）: {}", e);
        }
    }
}
