use axum::{Router, routing::{get, post, delete, put}, middleware};
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

    info!("Starting file service...");

    let db_manager = DatabaseManager::new(&database_url, &redis_url).await?;
    let pool = db_manager.pg_pool().clone();

    let token_manager = Arc::new(TokenManager::new(jwt_secret.as_bytes()));

    let file_service = Arc::new(crate::services::FileService::new(pool));

    let app_state = Arc::new(AppState {
        file_service,
    });

    let app = create_router(app_state, token_manager);

    let addr = "0.0.0.0:8007";
    let listener = TcpListener::bind(addr).await?;
    info!("File service listening on http://{}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

fn create_router(app_state: Arc<AppState>, token_manager: Arc<TokenManager>) -> Router {
    let public_routes = Router::new()
        .route("/health", get(health_check));

    let protected_routes = Router::new()
        .route("/files/upload", post(upload_file))
        .route("/files/batch-upload", post(batch_upload_files))
        .route("/files/:file_id", get(download_file))
        .route("/files/:file_id", delete(delete_file))
        .route("/files/:file_id", put(update_file))
        .route("/files", get(list_files))
        .route("/files/:file_id/thumbnail", get(get_thumbnail))
        .route("/files/stats/storage", get(get_storage_stats))
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