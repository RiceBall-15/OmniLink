use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;
use sqlx::PgPool;

use crate::models::auth::ApiResponse;
use common::crypto;

/// 生成用户身份密钥对
pub async fn generate_keys(
    State(pool): State<PgPool>,
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
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
    Path(conversation_id): Path<String>,
) -> impl IntoResponse {
    let conv_uuid = match Uuid::parse_str(&conversation_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error("无效的会话ID")),
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
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let plaintext = match req.get("content").and_then(|v| v.as_str()) {
        Some(text) => text,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error("缺少消息内容")),
            );
        }
    };
    
    let key_b64 = match req.get("sessionKey").and_then(|v| v.as_str()) {
        Some(key) => key,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error("缺少会话密钥")),
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
                Json(ApiResponse::<()>::error("无效的密钥格式")),
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
            Json(ApiResponse::<()>::error(&format!("加密失败: {}", e))),
        ),
    }
}

/// 解密消息
pub async fn decrypt_message(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let ciphertext = match req.get("ciphertext").and_then(|v| v.as_str()) {
        Some(text) => text,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error("缺少密文")),
            );
        }
    };
    
    let nonce = match req.get("nonce").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error("缺少nonce")),
            );
        }
    };
    
    let key_b64 = match req.get("sessionKey").and_then(|v| v.as_str()) {
        Some(key) => key,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error("缺少会话密钥")),
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
                Json(ApiResponse::<()>::error("无效的密钥格式")),
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
            Json(ApiResponse::<()>::error(&format!("解密失败: {}", e))),
        ),
    }
}

/// 获取加密信息
pub async fn get_encryption_info(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
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
