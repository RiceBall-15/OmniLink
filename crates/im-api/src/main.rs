use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tracing::info;

pub async fn run() -> anyhow::Result<()> {
    info!("Starting IM API service...");

    let app = Router::new()
        .route("/health", get(health_check));

    let listener = TcpListener::bind("0.0.0.0:8002").await?;
    info!("IM API listening on http://0.0.0.0:8002");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_check() -> &'static str {
    "IM API is healthy"
}