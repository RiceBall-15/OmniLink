//! 端到端加密处理器模块
//!
//! 提供加密相关的 API 端点：
//! - `POST /api/im/encryption/keys` - 生成加密密钥对
//! - `GET /api/im/encryption/session-key/:id` - 获取会话密钥
//! - `POST /api/im/encryption/encrypt` - 加密消息
//! - `POST /api/im/encryption/decrypt` - 解密消息
//! - `GET /api/im/encryption/info` - 获取加密信息
//! - `POST /api/im/encryption/key-exchange` - 密钥交换
//! - `POST /api/im/encryption/store` - 存储加密消息
//! - `GET /api/im/encryption/messages/:id` - 获取加密消息历史

use axum::{
    extract::{Extension, State, Path},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;
use sqlx::PgPool;

use crate::models::auth::ApiResponse;
use common::crypto;

/// 生成用户身份密钥对
#[utoipa::path(
    post,
    path = "/api/im/encryption/keys/generate",
    tag = "encryption",
    responses(
        (status = 200, description = "生成成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn generate_keys(
    State(_pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
) -> impl IntoResponse {
    let key_pair = crypto::generate_identity_key_pair(user_id);
    
    // 保存公钥到数据库（可选）
    // 这里先返回密钥对，实际应用中私钥应加密存储
    
    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "publicKey": key_pair.public_key,
            "createdAt": key_pair.created_at,
            "message": "请安全保存私钥，私钥不会再次显示"
        }))),
    )
}

/// 获取加密会话密钥
pub async fn get_session_key(
    State(_pool): State<PgPool>,
    Extension(_user_id): Extension<Uuid>,
    Path(conversation_id): Path<String>,
) -> impl IntoResponse {
    let _conv_uuid = match Uuid::parse_str(&conversation_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error("INVALID_SESSION_ID", "无效的会话ID")),
            );
        }
    };
    
    // 生成新的会话密钥
    let session_key = crypto::generate_session_key();
    let session_key_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &session_key,
    );
    
    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "conversationId": conversation_id,
            "sessionKey": session_key_b64,
            "algorithm": "AES-256-GCM",
            "keyLength": 256
        }))),
    )
}

/// 加密消息
#[utoipa::path(
    post,
    path = "/api/im/encryption/encrypt",
    tag = "encryption",
    responses(
        (status = 200, description = "加密成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn encrypt_message(
    State(_pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let plaintext = match req.get("content").and_then(|v| v.as_str()) {
        Some(text) => text,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error("MISSING_MESSAGE", "缺少消息内容")),
            );
        }
    };
    
    let key_b64 = match req.get("sessionKey").and_then(|v| v.as_str()) {
        Some(key) => key,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error("MISSING_SESSION_KEY", "缺少会话密钥")),
            );
        }
    };
    
    let key = match base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        key_b64,
    ) {
        Ok(k) => k,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error("INVALID_KEY_FORMAT", "无效的密钥格式")),
            );
        }
    };
    
    match crypto::encrypt_message(plaintext.as_bytes(), &key) {
        Ok(encrypted) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "ciphertext": encrypted.ciphertext,
                "nonce": encrypted.nonce,
                "senderId": user_id.to_string(),
                "timestamp": encrypted.timestamp
            }))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error("ENCRYPT_FAILED", format!("加密失败: {}", e))),
        ),
    }
}

/// 解密消息
#[utoipa::path(
    post,
    path = "/api/im/encryption/decrypt",
    tag = "encryption",
    responses(
        (status = 200, description = "解密成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn decrypt_message(
    State(_pool): State<PgPool>,
    Extension(_user_id): Extension<Uuid>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let ciphertext = match req.get("ciphertext").and_then(|v| v.as_str()) {
        Some(text) => text,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error("MISSING_CIPHERTEXT", "缺少密文")),
            );
        }
    };
    
    let nonce = match req.get("nonce").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error("MISSING_NONCE", "缺少nonce")),
            );
        }
    };
    
    let key_b64 = match req.get("sessionKey").and_then(|v| v.as_str()) {
        Some(key) => key,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error("MISSING_SESSION_KEY", "缺少会话密钥")),
            );
        }
    };
    
    let key = match base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        key_b64,
    ) {
        Ok(k) => k,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error("INVALID_KEY_FORMAT", "无效的密钥格式")),
            );
        }
    };
    
    let encrypted = crypto::EncryptedMessage {
        ciphertext: ciphertext.to_string(),
        nonce: nonce.to_string(),
        sender_id: Uuid::nil(),
        timestamp: 0,
    };
    
    match crypto::decrypt_message(&encrypted, &key) {
        Ok(decrypted) => {
            let plaintext = String::from_utf8_lossy(&decrypted);
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "content": plaintext.to_string(),
                    "senderId": encrypted.sender_id.to_string()
                }))),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error("DECRYPT_FAILED", format!("解密失败: {}", e))),
        ),
    }
}

/// 获取加密信息
pub async fn get_encryption_info(
    State(_pool): State<PgPool>,
    Extension(_user_id): Extension<Uuid>,
) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "algorithms": ["AES-256-GCM"],
            "keyExchange": "X25519 (planned)",
            "keyLength": 256,
            "nonceLength": 96,
            "supportedOperations": [
                "encrypt_message",
                "decrypt_message",
                "generate_keys",
                "key_exchange"
            ]
        }))),
    )
}

/// 密钥交换请求
#[utoipa::path(
    post,
    path = "/api/im/encryption/key-exchange",
    tag = "encryption",
    responses(
        (status = 200, description = "交换成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn key_exchange(
    State(_pool): State<PgPool>,
    Extension(_user_id): Extension<Uuid>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let conversation_id = match req.get("conversationId").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error("MISSING_SESSION_ID", "缺少会话ID")),
            );
        }
    };
    
    let _public_key = match req.get("publicKey").and_then(|v| v.as_str()) {
        Some(key) => key,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error("MISSING_PUBLIC_KEY", "缺少公钥")),
            );
        }
    };
    
    // 生成新的会话密钥用于此会话
    let session_key = crypto::generate_session_key();
    let _session_key_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &session_key,
    );
    
    // 使用对方公钥加密会话密钥（简化实现）
    // 实际应用中应使用 X25519 DH 密钥交换
    let encrypted_session_key = crypto::encrypt_session_key(
        &session_key,
        &session_key, // 简化：使用会话密钥自身作为主密钥
    ).unwrap_or_default();
    
    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "conversationId": conversation_id,
            "encryptedSessionKey": encrypted_session_key,
            "algorithm": "AES-256-GCM",
            "expiresAt": chrono::Utc::now().checked_add_signed(chrono::Duration::hours(24)).unwrap().to_rfc3339()
        }))),
    )
}

/// 保存加密消息到数据库
pub async fn store_encrypted_message(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let conversation_id = match req.get("conversationId").and_then(|v| v.as_str()) {
        Some(id) => match Uuid::parse_str(id) {
            Ok(uuid) => uuid,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<serde_json::Value>::error("INVALID_SESSION_ID", "无效的会话ID")),
                );
            }
        },
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error("MISSING_SESSION_ID", "缺少会话ID")),
            );
        }
    };
    
    let ciphertext = match req.get("ciphertext").and_then(|v| v.as_str()) {
        Some(text) => text,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error("MISSING_CIPHERTEXT", "缺少密文")),
            );
        }
    };
    
    let nonce = match req.get("nonce").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error("MISSING_NONCE", "缺少nonce")),
            );
        }
    };
    
    let message_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    
    // 存储加密消息到数据库
    let result = sqlx::query(
        "INSERT INTO encrypted_messages (id, conversation_id, sender_id, ciphertext, nonce, created_at) VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(message_id)
    .bind(conversation_id)
    .bind(user_id)
    .bind(ciphertext)
    .bind(nonce)
    .bind(now.naive_utc())
    .execute(&pool)
    .await;
    
    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "messageId": message_id,
                "conversationId": conversation_id,
                "senderId": user_id,
                "storedAt": now.to_rfc3339()
            }))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error("STORE_FAILED", format!("存储加密消息失败: {}", e))),
        ),
    }
}

/// 注册用户公钥（E2E加密）
/// 
/// 用户注册身份公钥，用于后续的密钥交换和消息加密。
/// 支持多种密钥类型：identity（身份密钥）、signed_pre_key（签名预密钥）、one_time_pre_key（一次性预密钥）
#[utoipa::path(
    post,
    path = "/api/im/encryption/register-key",
    tag = "encryption",
    responses(
        (status = 200, description = "注册成功", body = ApiResponse<serde_json::Value>),
        (status = 400, description = "请求参数错误", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn register_public_key(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let public_key = match req.get("publicKey").and_then(|v| v.as_str()) {
        Some(key) => key.to_string(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error("MISSING_PUBLIC_KEY", "缺少公钥")),
            );
        }
    };

    let key_type = req.get("keyType")
        .and_then(|v| v.as_str())
        .unwrap_or("identity")
        .to_string();

    // 验证密钥类型
    if !["identity", "signed_pre_key", "one_time_pre_key"].contains(&key_type.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error("INVALID_KEY_TYPE", "无效的密钥类型，支持：identity, signed_pre_key, one_time_pre_key")),
        );
    }

    // 验证公钥格式（Base64）
    if base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &public_key).is_err() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error("INVALID_KEY_FORMAT", "公钥格式无效，需要Base64编码")),
        );
    }

    // 获取当前版本号
    let max_version: Result<(i32,), sqlx::Error> = sqlx::query_as(
        "SELECT COALESCE(MAX(key_version), 0) FROM user_public_keys WHERE user_id = $1 AND key_type = $2"
    )
    .bind(user_id)
    .bind(&key_type)
    .fetch_one(&pool)
    .await;

    let new_version = match max_version {
        Ok((v,)) => v + 1,
        Err(_) => 1,
    };

    // 将旧密钥设为非活跃
    let _ = sqlx::query(
        "UPDATE user_public_keys SET is_active = false WHERE user_id = $1 AND key_type = $2 AND is_active = true"
    )
    .bind(user_id)
    .bind(&key_type)
    .execute(&pool)
    .await;

    // 插入新公钥
    let key_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let expires_at = now + chrono::Duration::days(30); // 公钥30天过期

    let result = sqlx::query(
        "INSERT INTO user_public_keys (id, user_id, public_key, key_type, key_version, is_active, created_at, expires_at) VALUES ($1, $2, $3, $4, $5, true, $6, $7)"
    )
    .bind(key_id)
    .bind(user_id)
    .bind(&public_key)
    .bind(&key_type)
    .bind(new_version)
    .bind(now.naive_utc())
    .bind(expires_at.naive_utc())
    .execute(&pool)
    .await;

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "keyId": key_id,
                "userId": user_id,
                "keyType": key_type,
                "keyVersion": new_version,
                "expiresAt": expires_at.to_rfc3339(),
                "message": "公钥注册成功"
            }))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error("REGISTER_FAILED", format!("公钥注册失败: {}", e))),
        ),
    }
}

/// 获取用户公钥
/// 
/// 获取指定用户的活跃公钥，用于密钥交换。
/// 支持按密钥类型筛选。
#[utoipa::path(
    get,
    path = "/api/im/encryption/public-key/{target_user_id}",
    tag = "encryption",
    params(("target_user_id" = String, Path, description = "目标用户ID")),
    responses(
        (status = 200, description = "获取成功", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "用户公钥不存在", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn get_user_public_key(
    State(pool): State<PgPool>,
    Extension(_user_id): Extension<Uuid>,
    Path(target_user_id): Path<String>,
) -> impl IntoResponse {
    let target_uuid = match Uuid::parse_str(&target_user_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error("INVALID_USER_ID", "无效的用户ID")),
            );
        }
    };

    // 获取用户的活跃公钥
    let keys: Result<Vec<(Uuid, String, String, i32, chrono::NaiveDateTime)>, sqlx::Error> = sqlx::query_as(
        "SELECT id, public_key, key_type, key_version, created_at FROM user_public_keys WHERE user_id = $1 AND is_active = true ORDER BY key_type, key_version DESC"
    )
    .bind(target_uuid)
    .fetch_all(&pool)
    .await;

    match keys {
        Ok(key_list) if !key_list.is_empty() => {
            let result: Vec<serde_json::Value> = key_list.iter().map(|(id, public_key, key_type, version, created_at)| {
                serde_json::json!({
                    "keyId": id,
                    "publicKey": public_key,
                    "keyType": key_type,
                    "keyVersion": version,
                    "createdAt": created_at.and_utc().to_rfc3339()
                })
            }).collect();

            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "userId": target_user_id,
                    "keys": result,
                    "count": result.len()
                }))),
            )
        }
        Ok(_) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<serde_json::Value>::error("KEY_NOT_FOUND", "该用户尚未注册公钥")),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error("FETCH_FAILED", format!("获取公钥失败: {}", e))),
        ),
    }
}

/// 批量获取用户公钥
/// 
/// 批量获取多个用户的公钥，用于群聊加密场景。
#[utoipa::path(
    post,
    path = "/api/im/encryption/public-keys/batch",
    tag = "encryption",
    responses(
        (status = 200, description = "获取成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn batch_get_public_keys(
    State(pool): State<PgPool>,
    Extension(_user_id): Extension<Uuid>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let user_ids = match req.get("userIds").and_then(|v| v.as_array()) {
        Some(ids) => ids,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error("MISSING_USER_IDS", "缺少用户ID列表")),
            );
        }
    };

    let mut parsed_ids = Vec::new();
    for id in user_ids {
        if let Some(id_str) = id.as_str() {
            if let Ok(uuid) = Uuid::parse_str(id_str) {
                parsed_ids.push(uuid);
            }
        }
    }

    if parsed_ids.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error("NO_VALID_IDS", "没有有效的用户ID")),
        );
    }

    // 批量查询公钥
    let keys: Result<Vec<(Uuid, Uuid, String, String, i32, chrono::NaiveDateTime)>, sqlx::Error> = sqlx::query_as(
        "SELECT id, user_id, public_key, key_type, key_version, created_at FROM user_public_keys WHERE user_id = ANY($1) AND is_active = true ORDER BY user_id, key_type"
    )
    .bind(&parsed_ids)
    .fetch_all(&pool)
    .await;

    match keys {
        Ok(key_list) => {
            let mut result: std::collections::HashMap<String, Vec<serde_json::Value>> = std::collections::HashMap::new();
            
            for (id, uid, public_key, key_type, version, created_at) in key_list {
                let entry = serde_json::json!({
                    "keyId": id,
                    "publicKey": public_key,
                    "keyType": key_type,
                    "keyVersion": version,
                    "createdAt": created_at.and_utc().to_rfc3339()
                });
                result.entry(uid.to_string()).or_default().push(entry);
            }

            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "keys": result,
                    "totalUsers": parsed_ids.len(),
                    "foundUsers": result.len()
                }))),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error("BATCH_FETCH_FAILED", format!("批量获取公钥失败: {}", e))),
        ),
    }
}

/// 获取会话的加密消息历史
#[utoipa::path(
    get,
    path = "/api/im/encryption/messages/{conversation_id}",
    tag = "encryption",
    params(("conversation_id" = String, Path, description = "会话ID")),
    responses(
        (status = 200, description = "获取成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn get_encrypted_messages(
    State(pool): State<PgPool>,
    Extension(_user_id): Extension<Uuid>,
    Path(conversation_id): Path<String>,
) -> impl IntoResponse {
    let conv_uuid = match Uuid::parse_str(&conversation_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error("INVALID_SESSION_ID", "无效的会话ID")),
            );
        }
    };
    
    // 从数据库获取加密消息
    let messages: Result<Vec<(Uuid, Uuid, String, String, chrono::NaiveDateTime)>, sqlx::Error> = sqlx::query_as(
        "SELECT id, sender_id, ciphertext, nonce, created_at FROM encrypted_messages WHERE conversation_id = $1 ORDER BY created_at DESC LIMIT 100"
    )
    .bind(conv_uuid)
    .fetch_all(&pool)
    .await;
    
    match messages {
        Ok(msgs) => {
            let result: Vec<serde_json::Value> = msgs.iter().map(|(id, sender_id, ciphertext, nonce, created_at)| {
                serde_json::json!({
                    "messageId": id,
                    "senderId": sender_id,
                    "ciphertext": ciphertext,
                    "nonce": nonce,
                    "createdAt": created_at.and_utc().to_rfc3339()
                })
            }).collect();
            
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "conversationId": conversation_id,
                    "messages": result,
                    "count": result.len()
                }))),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error("FETCH_FAILED", format!("获取加密消息失败: {}", e))),
        ),
    }
}
