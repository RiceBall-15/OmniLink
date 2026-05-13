use axum::extract::{ws::{WebSocket, Message}, State, WebSocketUpgrade};
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

use crate::connection_manager::{WSConnection, WSConnectionManager, ConnectionId};
use crate::status_manager::OnlineStatusManager;
use crate::services::IMService;
use crate::models::{WSMessage, WSMessageType, WSConnectRequest};
use common::auth::TokenManager;

/// 应用共享状态
#[derive(Clone)]
pub struct AppState {
    pub connection_manager: Arc<WSConnectionManager>,
    pub status_manager: Arc<OnlineStatusManager>,
    pub token_manager: Arc<TokenManager>,
    pub im_service: Arc<IMService>,
}

/// WebSocket 路由处理器
pub async fn websocket_handler(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

/// 处理 WebSocket 连接
async fn handle_websocket(
    socket: WebSocket,
    state: Arc<AppState>,
) {
    let addr = "unknown".to_string();

    tracing::info!("New WebSocket connection from {}", addr);

    // 生成连接ID
    let connection_id = Uuid::new_v4();
    let mut user_id: Option<Uuid> = None;
    let mut authenticated = false;

    // 创建消息通道用于向客户端发送消息
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    // Split the socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // 启动发送任务
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // 启动心跳发送任务 (每30秒发送一次 PING)
    let connection_manager_clone = state.connection_manager.clone();
    let tx_clone = tx.clone();
    let connection_id_heartbeat = connection_id;
    let heartbeat_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

        loop {
            interval.tick().await;

            // 检查连接是否还存在
            if connection_manager_clone
                .get_connection(connection_id_heartbeat)
                .await
                .is_none()
            {
                tracing::info!("Heartbeat task stopped for connection {}", connection_id_heartbeat);
                break;
            }

            // 发送 PING 消息
            let ping_msg = WSMessage {
                message_type: WSMessageType::Ping,
                conversation_id: None,
                message_id: None,
                sender_id: None,
                content: None,
                timestamp: Some(Utc::now().timestamp()),
                data: None,
            };

            if let Ok(json) = serde_json::to_string(&ping_msg) {
                if tx_clone.send(Message::Text(json)).is_err() {
                    tracing::warn!(
                        "Failed to send ping to connection {}",
                        connection_id_heartbeat
                    );
                    break;
                }
                tracing::debug!("Sent ping to connection {}", connection_id_heartbeat);
            }
        }
    });

    // 处理接收消息
    while let Some(msg_result) = receiver.next().await {
        match msg_result {
            Ok(msg) => {
                if let Ok(text) = msg.to_text() {
                    tracing::debug!("Received message from {}: {}", addr, text);

                    // 解析 WebSocket 消息
                    if let Ok(ws_msg) = serde_json::from_str::<WSMessage>(text) {
                        // 更新最后活跃时间
                        state.connection_manager.update_last_active(connection_id).await;

                        if !authenticated {
                            // 处理认证
                            handle_auth_message(
                                &ws_msg,
                                &state,
                                connection_id,
                                &addr,
                                &tx,
                                &mut user_id,
                                &mut authenticated,
                            )
                            .await;
                        } else {
                            // 处理已认证的消息
                            handle_authenticated_message(
                                &ws_msg,
                                &state,
                                connection_id,
                                user_id.unwrap(),
                                &tx,
                            )
                            .await;
                        }
                    } else {
                        tracing::warn!("Invalid message format from {}: {}", addr, text);
                        send_error(
                            &tx,
                            "Invalid message format".to_string(),
                            "format_error".to_string(),
                        );
                    }
                } else if matches!(msg, Message::Close(_)) {
                    tracing::info!("WebSocket close signal from {}", addr);
                    break;
                }
            }
            Err(e) => {
                tracing::error!("WebSocket error from {}: {:?}", addr, e);
                break;
            }
        }
    }

    // 清理资源
    tracing::info!("Cleaning up connection {} from {}", connection_id, addr);

    // 停止心跳任务
    heartbeat_task.abort();

    // 移除连接
    if let Some(conn) = state.connection_manager.remove_connection(connection_id).await {
        // 如果这是用户的最后一个连接，更新在线状态为离线
        if !state.connection_manager.is_online(conn.user_id).await {
            state.status_manager.set_offline(conn.user_id).await;
            tracing::info!("User {} is now offline (last connection closed)", conn.user_id);

            // 广播离线状态变更给所有连接的用户
            let status_change_msg = WSMessage {
                message_type: WSMessageType::StatusChange,
                conversation_id: None,
                message_id: None,
                sender_id: Some(conn.user_id),
                content: None,
                timestamp: Some(Utc::now().timestamp()),
                data: Some(serde_json::json!({
                    "user_id": conn.user_id,
                    "status": "offline",
                    "last_seen": Utc::now().timestamp(),
                })),
            };
            state.connection_manager.broadcast(status_change_msg).await;
        }
    }

    // 停止发送任务
    send_task.abort();
}

/// 处理认证消息
async fn handle_auth_message(
    ws_msg: &WSMessage,
    state: &Arc<AppState>,
    connection_id: ConnectionId,
    addr: &str,
    tx: &tokio::sync::mpsc::UnboundedSender<Message>,
    user_id: &mut Option<Uuid>,
    authenticated: &mut bool,
) {
    if ws_msg.message_type != WSMessageType::Connect {
        send_error(tx, "Authentication required".to_string(), "auth_required".to_string());
        return;
    }

    // 从消息数据中提取 token
    if let Some(data) = &ws_msg.data {
        if let Ok(connect_req) = serde_json::from_value::<WSConnectRequest>(data.clone()) {
            // 验证 token
            match state.token_manager.verify_token(&connect_req.token) {
                Ok(claims) => {
                    let uid = claims.sub; // Claims.sub 是用户ID
                    *user_id = Some(uid);
                    *authenticated = true;

                    tracing::info!("User {} authenticated from {}", uid, addr);

                    // 创建连接对象
                    let now = Utc::now().timestamp();

                    let connection = WSConnection {
                        connection_id,
                        user_id: uid,
                        conversation_id: connect_req.conversation_id,
                        addr: addr.to_string(),
                        sender: tx.clone(),
                        connected_at: now,
                        last_active_at: now,
                    };

                    // 添加连接到管理器
                    state.connection_manager.add_connection(connection).await;

                    // 如果指定了会话ID，设置到连接中
                    if let Some(conv_id) = connect_req.conversation_id {
                        state.connection_manager
                            .set_conversation(connection_id, conv_id)
                            .await;
                    }

                    // 设置用户在线
                    state.status_manager.set_online(uid, Some(format!("ws:{}", addr))).await;

                    // 广播上线状态变更给所有连接的用户
                    let status_change_msg = WSMessage {
                        message_type: WSMessageType::StatusChange,
                        conversation_id: None,
                        message_id: None,
                        sender_id: Some(uid),
                        content: None,
                        timestamp: Some(Utc::now().timestamp()),
                        data: Some(serde_json::json!({
                            "user_id": uid,
                            "status": "online",
                            "last_seen": Utc::now().timestamp(),
                        })),
                    };
                    state.connection_manager.broadcast(status_change_msg).await;

                    // 发送连接成功消息
                    let response = WSMessage {
                        message_type: WSMessageType::Connected,
                        conversation_id: connect_req.conversation_id,
                        message_id: None,
                        sender_id: Some(uid),
                        content: Some("Connected successfully".to_string()),
                        timestamp: Some(Utc::now().timestamp()),
                        data: None,
                    };

                    if let Ok(json) = serde_json::to_string(&response) {
                        let _ = tx.send(Message::Text(json));
                    }

                    // 推送离线消息给用户（异步执行，不阻塞连接）
                    let im_service = state.im_service.clone();
                    let uid_for_offline = uid;
                    tokio::spawn(async move {
                        if let Err(e) = im_service.deliver_offline_messages(uid_for_offline).await {
                            tracing::warn!("Failed to deliver offline messages to user {}: {:?}", uid_for_offline, e);
                        }
                    });
                }
                Err(e) => {
                    tracing::warn!("Token validation failed from {}: {}", addr, e);
                    // 根据错误类型发送不同的错误信息
                    let (error_msg, error_code) = match e {
                        common::AppError::Auth(msg) if msg.contains("expired") => {
                            ("Token expired, please refresh".to_string(), "token_expired".to_string())
                        }
                        _ => {
                            ("Invalid token".to_string(), "auth_failed".to_string())
                        }
                    };
                    send_error(tx, error_msg, error_code);
                }
            }
        } else {
            send_error(
                tx,
                "Invalid connect request format".to_string(),
                "invalid_request".to_string(),
            );
        }
    } else {
        send_error(
            tx,
            "Token required in connect message".to_string(),
            "token_required".to_string(),
        );
    }
}

/// 处理已认证的消息
async fn handle_authenticated_message(
    ws_msg: &WSMessage,
    state: &Arc<AppState>,
    connection_id: ConnectionId,
    user_id: Uuid,
    tx: &tokio::sync::mpsc::UnboundedSender<Message>,
) {
    match ws_msg.message_type {
        WSMessageType::Ping => {
            // 客户端发送的 PING，回复 PONG
            let pong = WSMessage {
                message_type: WSMessageType::Pong,
                conversation_id: ws_msg.conversation_id,
                message_id: None,
                sender_id: None,
                content: None,
                timestamp: Some(Utc::now().timestamp()),
                data: None,
            };

            if let Ok(json) = serde_json::to_string(&pong) {
                let _ = tx.send(Message::Text(json));
            }
            tracing::debug!("Replied pong to user {} (connection: {})", user_id, connection_id);
        }

        WSMessageType::Pong => {
            // 收到客户端的 PONG，确认心跳正常
            tracing::debug!("Received pong from user {} (connection: {})", user_id, connection_id);
            // 活跃时间已在接收消息时更新
        }

        WSMessageType::Message => {
            // 处理普通消息 - 转发到对话中的其他用户（过滤屏蔽关系）
            tracing::info!(
                "Received message from user {} in conversation {:?}",
                user_id,
                ws_msg.conversation_id
            );

            if let Some(conversation_id) = ws_msg.conversation_id {
                // 获取屏蔽关系
                let block_manager = state.im_service.block_manager();
                let blocked_by_sender = block_manager.get_blocked_list(user_id).await;
                let blocked_senders = block_manager.get_blocked_by_list(user_id).await;

                // 广播消息到对话中的其他用户（过滤屏蔽用户）
                let broadcast_msg = WSMessage {
                    message_type: WSMessageType::Message,
                    conversation_id: Some(conversation_id),
                    message_id: ws_msg.message_id,
                    sender_id: Some(user_id),
                    content: ws_msg.content.clone(),
                    timestamp: Some(Utc::now().timestamp()),
                    data: ws_msg.data.clone(),
                };
                let sent = state.connection_manager.send_to_conversation_filtered(
                    conversation_id,
                    user_id,
                    &blocked_by_sender,
                    &blocked_senders,
                    broadcast_msg,
                ).await;
                tracing::debug!("Message broadcast to {} recipients (after block filter)", sent);
            }
        }

        WSMessageType::Typing => {
            // 处理输入状态 - 通知会话中的其他用户
            if let Some(conversation_id) = ws_msg.conversation_id {
                tracing::debug!("User {} is typing in conversation {}", user_id, conversation_id);
                let typing_msg = WSMessage {
                    message_type: WSMessageType::Typing,
                    conversation_id: Some(conversation_id),
                    message_id: None,
                    sender_id: Some(user_id),
                    content: None,
                    timestamp: Some(Utc::now().timestamp()),
                    data: None,
                };
                state.connection_manager.send_to_conversation_except(
                    conversation_id,
                    user_id,
                    typing_msg,
                ).await;
            }
        }

        WSMessageType::Read => {
            // 处理已读回执 - 通知发送者消息已读
            if let Some(message_id) = ws_msg.message_id {
                tracing::info!("User {} marked message {} as read", user_id, message_id);
                if let Some(conversation_id) = ws_msg.conversation_id {
                    // 广播已读事件到对话
                    let read_msg = WSMessage {
                        message_type: WSMessageType::Read,
                        conversation_id: Some(conversation_id),
                        message_id: Some(message_id),
                        sender_id: Some(user_id),
                        content: None,
                        timestamp: Some(Utc::now().timestamp()),
                        data: Some(serde_json::json!({
                            "read_by": user_id,
                            "message_id": message_id,
                        })),
                    };
                    state.connection_manager.send_to_conversation(conversation_id, read_msg).await;
                }
            }
        }

        WSMessageType::Edit => {
            // 处理消息编辑 - 广播编辑事件到会话
            if let (Some(conversation_id), Some(message_id)) = (ws_msg.conversation_id, ws_msg.message_id) {
                tracing::info!("User {} editing message {} in conversation {}", user_id, message_id, conversation_id);
                let edit_msg = WSMessage {
                    message_type: WSMessageType::Edit,
                    conversation_id: Some(conversation_id),
                    message_id: Some(message_id),
                    sender_id: Some(user_id),
                    content: ws_msg.content.clone(),
                    timestamp: Some(Utc::now().timestamp()),
                    data: Some(serde_json::json!({
                        "message_id": message_id,
                        "conversation_id": conversation_id,
                    })),
                };
                state.connection_manager.send_to_conversation(conversation_id, edit_msg).await;
            } else {
                tracing::warn!("Edit message missing conversation_id or message_id from user {}", user_id);
            }
        }

        WSMessageType::Recall => {
            // 处理消息撤回 - 广播撤回事件到会话
            if let (Some(conversation_id), Some(message_id)) = (ws_msg.conversation_id, ws_msg.message_id) {
                tracing::info!("User {} recalling message {} in conversation {}", user_id, message_id, conversation_id);
                let recall_msg = WSMessage {
                    message_type: WSMessageType::Recall,
                    conversation_id: Some(conversation_id),
                    message_id: Some(message_id),
                    sender_id: Some(user_id),
                    content: None,
                    timestamp: Some(Utc::now().timestamp()),
                    data: Some(serde_json::json!({
                        "message_id": message_id,
                        "conversation_id": conversation_id,
                    })),
                };
                state.connection_manager.send_to_conversation(conversation_id, recall_msg).await;
            } else {
                tracing::warn!("Recall message missing conversation_id or message_id from user {}", user_id);
            }
        }

        WSMessageType::StatusChange => {
            // 处理状态变更请求 - 用户主动更改状态 (away, busy, 等)
            if let Some(data) = &ws_msg.data {
                if let Ok(status_req) = serde_json::from_value::<crate::models::StatusChangeRequest>(data.clone()) {
                    let new_status = crate::status_manager::UserStatus::from_str(&status_req.status);
                    tracing::info!("User {} changing status to {:?}", user_id, new_status);
                    state.status_manager.update_status(user_id, new_status.clone()).await;

                    // 广播状态变更给所有连接的用户
                    let status_change_msg = WSMessage {
                        message_type: WSMessageType::StatusChange,
                        conversation_id: None,
                        message_id: None,
                        sender_id: Some(user_id),
                        content: None,
                        timestamp: Some(Utc::now().timestamp()),
                        data: Some(serde_json::json!({
                            "user_id": user_id,
                            "status": format!("{:?}", new_status).to_lowercase(),
                            "last_seen": Utc::now().timestamp(),
                        })),
                    };
                    state.connection_manager.broadcast(status_change_msg).await;
                }
            }
        }

        WSMessageType::TokenRefresh => {
            // 处理 Token 刷新 - 客户端发送新 token 来替换旧的
            tracing::info!("User {} requesting token refresh", user_id);
            if let Some(data) = &ws_msg.data {
                if let Ok(refresh_req) = serde_json::from_value::<crate::models::TokenRefreshRequest>(data.clone()) {
                    // 验证新 token 的有效性
                    match state.token_manager.verify_token(&refresh_req.token) {
                        Ok(new_claims) => {
                            // 确认新 token 属于同一用户
                            if new_claims.sub == user_id {
                                tracing::info!("Token refreshed successfully for user {}", user_id);
                                // 发送刷新成功响应
                                let response = WSMessage {
                                    message_type: WSMessageType::RefreshOk,
                                    conversation_id: None,
                                    message_id: None,
                                    sender_id: Some(user_id),
                                    content: Some("Token refreshed successfully".to_string()),
                                    timestamp: Some(Utc::now().timestamp()),
                                    data: Some(serde_json::json!({
                                        "expires_at": new_claims.exp,
                                    })),
                                };
                                if let Ok(json) = serde_json::to_string(&response) {
                                    let _ = tx.send(Message::Text(json));
                                }
                            } else {
                                tracing::warn!("Token refresh failed: user mismatch (expected {}, got {})", user_id, new_claims.sub);
                                send_error(tx, "Token user mismatch".to_string(), "token_mismatch".to_string());
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Token refresh failed: invalid new token: {}", e);
                            send_error(tx, "Invalid refresh token".to_string(), "invalid_token".to_string());
                        }
                    }
                } else {
                    send_error(tx, "Invalid refresh request format".to_string(), "invalid_request".to_string());
                }
            } else {
                send_error(tx, "Token required for refresh".to_string(), "token_required".to_string());
            }
        }

        _ => {
            tracing::warn!(
                "Unhandled message type from user {}: {:?}",
                user_id,
                ws_msg.message_type
            );
        }
    }
}

/// 发送错误消息
fn send_error(
    tx: &tokio::sync::mpsc::UnboundedSender<Message>,
    message: String,
    error_code: String,
) {
    let error_msg = WSMessage {
        message_type: WSMessageType::Error,
        conversation_id: None,
        message_id: None,
        sender_id: None,
        content: Some(message),
        timestamp: Some(Utc::now().timestamp()),
        data: Some(serde_json::json!({ "code": error_code })),
    };

    if let Ok(json) = serde_json::to_string(&error_msg) {
        let _ = tx.send(Message::Text(json));
    }
}
