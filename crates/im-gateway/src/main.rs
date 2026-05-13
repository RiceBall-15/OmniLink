use common::Result;
use common::auth::TokenManager;
use common::db::DatabaseManager;
use axum::{
    routing::{get, post, put},
    Router,
    middleware as axum_middleware,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use im_gateway::handlers::ws::{AppState, websocket_handler};
use im_gateway::services::IMService;
use im_gateway::conversation_service::ConversationService;
use im_gateway::repository::{MessageRepository, ConversationRepository};
use im_gateway::user_repository::UserRepository;
use im_gateway::connection_manager::WSConnectionManager;
use im_gateway::status_manager::OnlineStatusManager;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 加载环境变量
    dotenvy::dotenv().ok();

    // 数据库连接
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/omnilink".to_string());
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1".to_string());
    
    let db_manager = DatabaseManager::new(&database_url, &redis_url).await?;

    // 创建仓库实例
    let message_repository = Arc::new(MessageRepository::new(db_manager.pg_pool().clone()));
    let conversation_repository = Arc::new(ConversationRepository::new(db_manager.pg_pool().clone()));
    let user_repository = Arc::new(UserRepository::new(db_manager.pg_pool().clone()));

    // 创建管理器实例
    let connection_manager = Arc::new(WSConnectionManager::new());

    // 创建带 Redis 的在线状态管理器
    tracing::info!("Initializing OnlineStatusManager with Redis backend");
    let status_manager = Arc::new(OnlineStatusManager::with_redis(db_manager.redis().clone()));

    // 创建服务实例
    let im_service = Arc::new(IMService::new(
        message_repository.clone(),
        user_repository.clone(),
        connection_manager.clone(),
        status_manager.clone(),
    ));

    let conv_service = Arc::new(ConversationService::new(
        conversation_repository.clone(),
        user_repository.clone(),
    ));

    // 创建token管理器
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "default-secret-change-me".to_string());
    let token_manager = Arc::new(TokenManager::new(jwt_secret.as_bytes()));

    // 创建应用状态
    let ws_state = Arc::new(AppState {
        connection_manager: connection_manager.clone(),
        status_manager: status_manager.clone(),
        token_manager: token_manager.clone(),
    });

    // 认证中间件
    let auth_layer = axum_middleware::from_fn_with_state(
        token_manager.clone(),
        im_gateway::middleware::auth_middleware,
    );

    // 消息路由 (使用 im_service)
    let im_routes = Router::new()
        .route("/messages", post(im_gateway::handlers::send_message))
        .route("/messages/history", get(im_gateway::handlers::get_message_history))
        .route("/messages/read", post(im_gateway::handlers::mark_read))
        .route("/messages/edit", put(im_gateway::handlers::edit_message))
        .route("/messages/recall", post(im_gateway::handlers::recall_message))
        .route("/users/online", get(im_gateway::handlers::get_online_users))
        .route("/users/status/batch", post(im_gateway::handlers::batch_status_query))
        .with_state(im_service);

    // 对话路由 (使用 conv_service)
    let conv_routes = Router::new()
        .route("/conversations", post(im_gateway::handlers::create_conversation))
        .route("/conversations/list", get(im_gateway::handlers::list_conversations))
        .route("/conversations/{id}", get(im_gateway::handlers::get_conversation))
        .with_state(conv_service);

    // WebSocket路由
    let ws_routes = Router::new()
        .route("/ws", get(websocket_handler))
        .with_state(ws_state);

    // 合并路由
    let app = Router::new()
        .merge(im_routes)
        .merge(conv_routes)
        .merge(ws_routes)
        .layer(auth_layer)
        .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    // 从 Redis 加载之前的状态
    status_manager.load_from_redis().await;

    // 启动状态清理后台任务
    let cleanup_status_manager = status_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            cleanup_status_manager.cleanup_expired().await;
        }
    });

    // 启动 WebSocket 连接池心跳清理后台任务
    // 每 60 秒检查一次，清理超过 300 秒（5分钟）未活动的连接
    let _heartbeat_handle = connection_manager.start_heartbeat_task(60, 300);
    tracing::info!("WebSocket heartbeat cleanup task started (interval: 60s, timeout: 300s)");

    // 启动服务器
    let addr = "0.0.0.0:3002";
    tracing::info!("IM Gateway starting on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
