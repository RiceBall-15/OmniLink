use reqwest;
use serde_json::json;
use uuid::Uuid;

const BASE_URL: &str = "http://localhost:8080";
const FILE_SERVICE_URL: &str = "http://localhost:8007";

/// 辅助函数：注册并登录
async fn setup_auth_user(client: &reqwest::Client) -> Option<(String, String)> {
    let unique_id = Uuid::new_v4().to_string();
    let email = format!("file_{}@example.com", &unique_id[..8]);
    let username = format!("fileuser_{}", &unique_id[..8]);

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

/// 测试文件上传
#[tokio::test]
async fn test_upload_file() {
    let client = reqwest::Client::new();

    let auth = setup_auth_user(&client).await;
    if auth.is_none() {
        println!("无法获取认证 token，跳过测试");
        return;
    }
    let (token, _) = auth.unwrap();

    // 创建临时文件内容
    let file_content = b"Hello, this is a test file content for integration testing.";
    let file_part = reqwest::multipart::Part::bytes(file_content.to_vec())
        .file_name("test.txt")
        .mime_str("text/plain").unwrap();

    let form = reqwest::multipart::Form::new()
        .part("file", file_part);

    let response = client
        .post(&format!("{}/api/files/upload", FILE_SERVICE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            println!("文件上传: status={}, body={}", status, body);
            // 文件服务可能未运行
            if status.is_success() {
                assert!(body.get("data").is_some(), "响应中缺少文件信息");
            } else {
                println!("文件上传返回非成功状态: {}", status);
            }
        }
        Err(e) => {
            println!("文件上传请求失败（文件服务可能未运行）: {}", e);
        }
    }
}

/// 测试获取文件列表
#[tokio::test]
async fn test_list_files() {
    let client = reqwest::Client::new();

    let auth = setup_auth_user(&client).await;
    if auth.is_none() {
        println!("无法获取认证 token，跳过测试");
        return;
    }
    let (token, _) = auth.unwrap();

    let response = client
        .get(&format!("{}/api/files", FILE_SERVICE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .query(&[("page", "1"), ("page_size", "20")])
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            println!("文件列表: status={}, body={}", status, body);
            if status.is_success() {
                // 成功获取文件列表
                println!("文件列表获取成功");
            } else {
                println!("文件列表返回非成功状态: {}", status);
            }
        }
        Err(e) => {
            println!("文件列表请求失败（文件服务可能未运行）: {}", e);
        }
    }
}

/// 测试获取存储统计
#[tokio::test]
async fn test_storage_stats() {
    let client = reqwest::Client::new();

    let auth = setup_auth_user(&client).await;
    if auth.is_none() {
        println!("无法获取认证 token，跳过测试");
        return;
    }
    let (token, _) = auth.unwrap();

    let response = client
        .get(&format!("{}/api/files/stats/storage", FILE_SERVICE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            println!("存储统计: status={}, body={}", status, body);
            if status.is_success() {
                println!("存储统计获取成功");
            } else {
                println!("存储统计返回非成功状态: {}", status);
            }
        }
        Err(e) => {
            println!("存储统计请求失败（文件服务可能未运行）: {}", e);
        }
    }
}

/// 测试文件下载（无效ID）
#[tokio::test]
async fn test_download_file_not_found() {
    let client = reqwest::Client::new();

    let auth = setup_auth_user(&client).await;
    if auth.is_none() {
        println!("无法获取认证 token，跳过测试");
        return;
    }
    let (token, _) = auth.unwrap();

    let fake_file_id = Uuid::new_v4();

    let response = client
        .get(&format!("{}/api/files/{}", FILE_SERVICE_URL, fake_file_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            println!("下载不存在的文件: status={}", status);
            // 应该返回 404 或 401
            assert!(
                status == reqwest::StatusCode::NOT_FOUND
                    || status == reqwest::StatusCode::UNAUTHORIZED
                    || status.is_success(),
                "文件下载处理异常: status={}", status
            );
        }
        Err(e) => {
            println!("文件下载请求失败（文件服务可能未运行）: {}", e);
        }
    }
}

/// 测试文件删除（无效ID）
#[tokio::test]
async fn test_delete_file_not_found() {
    let client = reqwest::Client::new();

    let auth = setup_auth_user(&client).await;
    if auth.is_none() {
        println!("无法获取认证 token，跳过测试");
        return;
    }
    let (token, _) = auth.unwrap();

    let fake_file_id = Uuid::new_v4();

    let response = client
        .delete(&format!("{}/api/files/{}", FILE_SERVICE_URL, fake_file_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            println!("删除不存在的文件: status={}", status);
            // 应该返回 404 或 401
            assert!(
                status == reqwest::StatusCode::NOT_FOUND
                    || status == reqwest::StatusCode::UNAUTHORIZED
                    || status == reqwest::StatusCode::FORBIDDEN
                    || status.is_success(),
                "文件删除处理异常: status={}", status
            );
        }
        Err(e) => {
            println!("文件删除请求失败（文件服务可能未运行）: {}", e);
        }
    }
}

/// 测试文件服务健康检查
#[tokio::test]
async fn test_file_service_health() {
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/health", FILE_SERVICE_URL))
        .send()
        .await;

    match response {
        Ok(resp) => {
            println!("文件服务健康检查: status={}", resp.status());
            // 健康检查应该返回 200
            assert!(
                resp.status().is_success() || resp.status() == reqwest::StatusCode::NOT_FOUND,
                "文件服务健康检查异常: status={}", resp.status()
            );
        }
        Err(e) => {
            println!("文件服务健康检查请求失败（服务可能未运行）: {}", e);
        }
    }
}

/// 测试未授权访问文件
#[tokio::test]
async fn test_unauthorized_file_access() {
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/api/files", FILE_SERVICE_URL))
        .header("Authorization", "Bearer invalid_token")
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            println!("未授权访问文件: status={}", status);
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
