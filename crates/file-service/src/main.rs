use axum::{Router, routing::{get, post, delete, put}, middleware};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

use common::{auth::TokenManager, db::DatabaseManager};
use file_service::handlers::{AppState, *};
use file_service::middleware::auth_middleware;
use file_service::progress::UploadProgressTracker;
use file_service::presign::PresignConfig;
use file_service::storage::MinioConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    let file_service = Arc::new(file_service::services::FileService::new(pool));

    // 初始化上传进度追踪器（最大保留 1000 条记录）
    let progress_tracker = UploadProgressTracker::new(1000);

    // 初始化预签名 URL 配置（仅 MinIO 模式）
    let presign_config = if std::env::var("STORAGE_TYPE").unwrap_or_default() == "minio" {
        let minio_config = MinioConfig::from_env();
        Some(PresignConfig::from_minio_config(&minio_config))
    } else {
        None
    };

    let app_state = Arc::new(AppState {
        file_service,
        progress_tracker,
        presign_config,
    });

    let app = create_router(app_state, token_manager);

    let addr = "0.0.0.0:8007";
    let listener = TcpListener::bind(addr).await?;
    info!("File service listening on http://{}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

fn create_router(app_state: Arc<AppState>, token_manager: Arc<TokenManager>) -> Router {
    // Public routes (no auth needed)
    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/api/files/share/{share_token}", get(download_shared_file))
        .route("/api/files/share/{share_token}/info", get(get_share_info));

    // Protected routes (require auth) - state will be applied at the end
    let protected_routes = Router::new()
        // 文件上传/下载
        .route("/api/files/upload", post(upload_file))
        .route("/api/files/batch-upload", post(batch_upload_files))
        .route("/api/files/{file_id}", get(download_file))
        .route("/api/files/{file_id}", delete(delete_file))
        .route("/api/files/{file_id}", put(update_file))
        .route("/api/files", get(list_files))
        .route("/api/files/{file_id}/thumbnail", get(get_thumbnail))
        .route("/api/files/{file_id}/preview", get(get_file_preview))
        .route("/api/files/stats/storage", get(get_storage_stats))
        // 文件分享
        .route("/api/files/{file_id}/shares", post(create_share))
        .route("/api/files/{file_id}/shares", get(get_file_shares))
        .route("/api/files/shares/{share_id}", delete(delete_share))
        // 预签名 URL
        .route("/api/files/presign/upload", post(get_presigned_upload_url))
        .route("/api/files/presign/{file_id}/download", get(get_presigned_download_url))
        // 上传进度追踪
        .route("/api/files/upload-progress/{upload_id}", get(get_upload_progress))
        .route("/api/files/upload-progress/{upload_id}", put(update_upload_progress))
        .route("/api/files/upload-progress/batch", post(get_batch_upload_progress))
        .route("/api/files/upload-progress/{upload_id}/complete", post(complete_upload))
        .route("/api/files/upload-progress/{upload_id}/fail", post(fail_upload))
        .layer(middleware::from_fn_with_state(
            token_manager.clone(),
            auth_middleware,
        ));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(middleware::from_fn(logging_middleware))
        .with_state(app_state)
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
