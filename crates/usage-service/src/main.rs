use axum::{Router, routing::{get, post}, middleware};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

use common::{auth::TokenManager, db::DatabaseManager};
use crate::handlers::{AppState, *};
use crate::middleware::auth_middleware;

pub async fn run() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 加载配置
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://im_chat:***@localhost:5432/im_chat".to_string());
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://:password@localhost:6379/0".to_string());
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "your-secret-key-change-in-production".to_string());

    info!("Starting usage service...");

    // 初始化数据库管理器
    let db_manager = DatabaseManager::new(&database_url, &redis_url).await?;
    let pool = db_manager.pg_pool().clone();

    // 初始化Token管理器
    let token_manager = Arc::new(TokenManager::new(jwt_secret.as_bytes()));

    // 初始化用量服务
    let usage_service = Arc::new(crate::services::UsageService::new(pool));

    // 创建应用状态
    let app_state = Arc::new(AppState {
        usage_service,
    });

    // 创建路由
    let app = create_router(app_state, token_manager);

    // 启动服务
    let addr = "0.0.0.0:8006";
    let listener = TcpListener::bind(addr).await?;
    info!("Usage service listening on http://{}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

fn create_router(app_state: Arc<AppState>, token_manager: Arc<TokenManager>) -> Router {
    // 公开路由（不需要认证）
    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/internal/calculate-cost", post(calculate_cost));

    // 需要认证的路由
    let protected_routes = Router::new()
        .route("/usage/tokens", post(record_token_usage))
        .route("/usage/tokens", get(get_token_usage))
        .route("/usage/stats", get(get_user_stats))
        .route("/usage/api-calls", get(get_api_calls))
        .route("/admin/cleanup/:days", post(cleanup_old_records))
        .layer(middleware::from_fn_with_state(
            token_manager.clone(),
            auth_middleware,
        ))
        .with_state(app_state);

    // 合并路由
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(middleware::from_fn(logging_middleware))
}

async fn logging_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = req.method().clone();
    let uri = req.uri().clone();

    let start = std::time::Instant::now();
    let response = next.run(req).await;
    let duration = start.elapsed();

    info!("{} {} - {:?}", method, uri, duration);

    response
}