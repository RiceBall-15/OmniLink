use axum::{Router, routing::{get, post, delete}, middleware};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

use common::{auth::TokenManager, db::DatabaseManager};
use crate::handlers::{AppState, *};
use crate::middleware::auth_middleware;

pub async fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://im_chat:***@localhost:5432/im_chat".to_string());
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://:password@localhost:6379/0".to_string());
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "your-secret-key-change-in-production".to_string());

    info!("Starting config service...");

    let db_manager = DatabaseManager::new(&database_url, &redis_url).await?;
    let pool = db_manager.pg_pool().clone();

    let token_manager = Arc::new(TokenManager::new(jwt_secret.as_bytes()));

    let config_service = Arc::new(crate::services::ConfigService::new(pool));

    // 预热缓存
    let _ = config_service.warmup_cache().await;

    let app_state = Arc::new(AppState {
        config_service,
    });

    let app = create_router(app_state, token_manager);

    let addr = "0.0.0.0:8008";
    let listener = TcpListener::bind(addr).await?;
    info!("Config service listening on http://{}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

fn create_router(app_state: Arc<AppState>, token_manager: Arc<TokenManager>) -> Router {
    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/config/:key", get(get_config))
        .route("/config", get(list_configs))
        .route("/config/batch", post(batch_get_configs))
        .route("/config/:key/subscriptions", get(get_subscriptions))
        .route("/config/subscriptions", post(add_subscription))
        .route("/config/subscriptions/:id", delete(remove_subscription))
        .route("/config/:key/history", get(get_config_history))
        .route("/config/warmup", post(warmup_cache));

    let protected_routes = Router::new()
        .route("/config/:key", post(set_config))
        .route("/config/:key", delete(delete_config))
        .route("/config/:key/restore/:version", post(restore_config_version))
        .layer(middleware::from_fn_with_state(
            token_manager.clone(),
            auth_middleware,
        ))
        .with_state(app_state);

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