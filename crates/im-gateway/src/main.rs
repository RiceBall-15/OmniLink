use axum::{Router, routing::{get, post, delete}, middleware};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::{StreamExt, SinkExt};
use tracing::info;

use common::{auth::TokenManager, db::DatabaseManager};
use crate::handlers::{
    send_message, create_conversation, list_conversations, get_conversation,
    get_message_history, mark_read, get_online_users,
};
use crate::services::IMService;
use crate::conversation_service::ConversationService;
use crate::repository::{MessageRepository, ConversationRepository};
use crate::connection_manager::WSConnectionManager;
use crate::status_manager::OnlineStatusManager;
use crate::models::{WSConnectRequest, WSMessage, WSMessageType};
use crate::middleware::auth_middleware;

pub async fn run() -> anyhow::Result<()> {
    info!("Starting IM Gateway service...");

    // 初始化数据库连接
    let db_manager = DatabaseManager::new(
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
        std::env::var("REDIS_URL").expect("REDIS_URL must be set"),
    ).await?;

    let pg_pool = db_manager.get_pg_pool();

    // 创建仓库
    let message_repository = Arc::new(MessageRepository::new(pg_pool.clone()));
    let conversation_repository = Arc::new(ConversationRepository::new(pg_pool));

    // 创建管理器
    let connection_manager = Arc::new(WSConnectionManager::new());
    let status_manager = Arc::new(OnlineStatusManager::new());

    // 创建服务
    let im_service = Arc::new(IMService::new(
        message_repository.clone(),
        connection_manager.clone(),
        status_manager.clone(),
    ));

    let conversation_service = Arc::new(ConversationService::new(conversation_repository));

    // 创建Token管理器
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "your-secret-key".to_string());
    let token_manager = Arc::new(TokenManager::new(jwt_secret));

    // HTTP路由
    let app = Router::new()
        // 消息
        .route("/messages", post(send_message))
        .route("/messages/history/:conversation_id", get(get_message_history))
        .route("/messages/read", post(mark_read))

        // 对话
        .route("/conversations", post(create_conversation))
        .route("/conversations", get(list_conversations))
        .route("/conversations/:conversation_id", get(get_conversation))

        // 在线用户
        .route("/online-users", get(get_online_users))

        // WebSocket路由
        .route("/ws", axum::routing::any(websocket_handler))

        // 添加认证中间件
        .layer(middleware::from_fn_with_state(
            token_manager.clone(),
            auth_middleware
        ))
        .with_state(im_service);

    // 启动HTTP服务器
    let http_addr = std::env::var("IM_GATEWAY_PORT")
        .unwrap_or_else(|_| "8001".to_string());
    let http_addr = format!("0.0.0.0:{}", http_addr);

    let listener = TcpListener::bind(&http_addr).await?;
    info!("IM Gateway HTTP service listening on {}", http_addr);

    // 启动WebSocket服务器
    let ws_addr = std::env::var("IM_GATEWAY_WS_PORT")
        .unwrap_or_else(|_| "8010".to_string());
    let ws_addr = format!("0.0.0.0:{}", ws_addr);

    let ws_listener = TcpListener::bind(&ws_addr).await?;
    info!("IM Gateway WebSocket service listening on {}", ws_addr);

    // 启动清理任务
    tokio::spawn(cleanup_task(connection_manager.clone(), status_manager.clone()));

    // 运行HTTP服务器
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("HTTP server error: {:?}", e);
        }
    });

    // 运行WebSocket服务器
    if let Err(e) = axum::serve(ws_listener, ws_router()).await {
        tracing::error!("WebSocket server error: {:?}", e);
    }

    Ok(())
}

/// WebSocket路由
fn ws_router() -> Router {
    Router::new().route("/ws", axum::routing::any(websocket_handler))
}

/// WebSocket处理器
async fn websocket_handler(
    ws: axum::extract::WebSocketUpgrade,
) -> axum::response::Response {
    ws.on_upgrade(handle_websocket)
}

/// 处理WebSocket连接
async fn handle_websocket(mut socket: axum::extract::ws::WebSocket) {
    let addr = socket
        .peer_addr()
        .ok()
        .map(|a| a.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    tracing::info!("WebSocket connection from {}", addr);

    let mut authenticated = false;
    let mut user_id = None;

    while let Some(msg_result) = socket.next().await {
        match msg_result {
            Ok(msg) => {
                if msg.is_text() {
                    let text = msg.to_text().unwrap_or("");

                    if let Ok(ws_msg) = serde_json::from_str::<WSMessage>(text) {
                        if !authenticated {
                            // 处理认证
                            if let WSMessageType::Connect = ws_msg.message_type {
                                if let Some(data) = ws_msg.data {
                                    if let Ok(connect_req) = serde_json::from_value::<WSConnectRequest>(data) {
                                        // 验证token
                                        // TODO: 实际验证token
                                        authenticated = true;
                                        user_id = Some(Uuid::new_v4()); // 暂时生成随机ID

                                        tracing::info!("User authenticated: {:?}", user_id);

                                        // 发送连接成功消息
                                        let response = WSMessage {
                                            message_type: WSMessageType::Connected,
                                            conversation_id: ws_msg.conversation_id,
                                            message_id: None,
                                            sender_id: user_id,
                                            content: Some("Connected successfully".to_string()),
                                            timestamp: Some(chrono::Utc::now().timestamp()),
                                            data: None,
                                        };

                                        if let Ok(json) = serde_json::to_string(&response) {
                                            let _ = socket.send(Message::Text(json)).await;
                                        }
                                    }
                                }
                            }
                        } else {
                            // 处理其他消息
                            match ws_msg.message_type {
                                WSMessageType::Message => {
                                    tracing::info!("Received message from user: {:?}", user_id);
                                    // 处理消息
                                }
                                WSMessageType::Ping => {
                                    // 回复pong
                                    let pong = WSMessage {
                                        message_type: WSMessageType::Pong,
                                        conversation_id: None,
                                        message_id: None,
                                        sender_id: None,
                                        content: None,
                                        timestamp: Some(chrono::Utc::now().timestamp()),
                                        data: None,
                                    };

                                    if let Ok(json) = serde_json::to_string(&pong) {
                                        let _ = socket.send(Message::Text(json)).await;
                                    }
                                }
                                _ => {
                                    tracing::warn!("Unhandled message type: {:?}", ws_msg.message_type);
                                }
                            }
                        }
                    }
                } else if msg.is_close() {
                    tracing::info!("WebSocket close from {}", addr);
                    break;
                }
            }
            Err(e) => {
                tracing::error!("WebSocket error: {:?}", e);
                break;
            }
        }
    }

    if authenticated {
        if let Some(uid) = user_id {
            tracing::info!("User {} disconnected", uid);
            // TODO: 移除连接，更新状态
        }
    }
}

/// 清理任务
async fn cleanup_task(
    connection_manager: Arc<WSConnectionManager>,
    status_manager: Arc<OnlineStatusManager>,
) {
    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

    loop {
        tracing::info!("Running cleanup task");

        // 清理过期的在线状态
        status_manager.cleanup_expired().await;

        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}