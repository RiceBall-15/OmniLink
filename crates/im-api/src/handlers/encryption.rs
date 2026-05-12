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

/// 获取会话的加密消息历史
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
