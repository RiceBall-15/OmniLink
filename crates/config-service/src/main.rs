use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tracing::info;

pub async fn run() -> anyhow::Result<()> {
    info!("Starting config service...");

    let app = Router::new()
        .route("/health", get(health_check));

    let listener = TcpListener::bind("0.0.0.0:8008").await?;
    info!("Config service listening on http://0.0.0.0:8008");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_check() -> &'static str {
    "Config service is healthy"
}