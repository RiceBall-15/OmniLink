use axum::{Router, routing::{get, post, delete}, middleware};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;
use std::collections::HashMap;

use common::{auth::TokenManager, db::DatabaseManager};
use crate::handlers::{
    chat, chat_stream, create_assistant, list_assistants, get_assistant,
    update_assistant, delete_assistant, get_token_usage, list_models,
};
use crate::middleware::auth_middleware;
use crate::repository::{AssistantRepository, TokenUsageRepository};
use crate::services::AIService;

pub async fn run() -> anyhow::Result<()> {
    info!("Starting AI service...");

    // 初始化数据库连接
    let db_manager = DatabaseManager::new(
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
        std::env::var("REDIS_URL").expect("REDIS_URL must be set"),
    ).await?;

    let pg_pool = db_manager.get_pg_pool();

    // 创建仓库
    let assistant_repository = Arc::new(AssistantRepository::new(pg_pool.clone()));
    let token_usage_repository = Arc::new(TokenUsageRepository::new(pg_pool));

    // 创建AI服务
    let ai_service = Arc::new(AIService::new(
        assistant_repository,
        token_usage_repository,
    ));

    // 初始化AI提供商（从环境变量读取API密钥）
    let mut api_keys = HashMap::new();

    if let Ok(openai_key) = std::env::var("OPENAI_API_KEY") {
        api_keys.insert("openai".to_string(), openai_key);
    }

    if let Ok(anthropic_key) = std::env::var("ANTHROPIC_API_KEY") {
        api_keys.insert("anthropic".to_string(), anthropic_key);
    }

    if let Ok(google_key) = std::env::var("GOOGLE_API_KEY") {
        api_keys.insert("google".to_string(), google_key);
    }

    ai_service.init_providers(api_keys).await?;

    // 创建Token管理器
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "your-secret-key".to_string());
    let token_manager = Arc::new(TokenManager::new(jwt_secret));

    // 构建路由
    let app = Router::new()
        // AI对话
        .route("/chat", post(chat))
        .route("/chat/stream", post(chat_stream))

        // AI助手管理
        .route("/assistants", post(create_assistant))
        .route("/assistants", get(list_assistants))
        .route("/assistants/:assistant_id", get(get_assistant))
        .route("/assistants/:assistant_id", post(update_assistant))
        .route("/assistants/:assistant_id", delete(delete_assistant))

        // Token使用统计
        .route("/usage", get(get_token_usage))

        // 模型列表
        .route("/models", get(list_models))

        // 添加认证中间件
        .layer(middleware::from_fn_with_state(
            token_manager.clone(),
            auth_middleware
        ))
        .with_state(ai_service);

    // 启动服务器
    let addr = std::env::var("AI_SERVICE_PORT")
        .unwrap_or_else(|_| "8003".to_string());
    let addr = format!("0.0.0.0:{}", addr);

    let listener = TcpListener::bind(&addr).await?;
    info!("AI service listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}