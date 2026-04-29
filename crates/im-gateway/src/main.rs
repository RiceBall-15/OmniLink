use tokio::net::TcpListener;
use axum::{Router, routing::get};
use tracing::info;

pub async fn run() -> anyhow::Result<()> {
    info!("Starting IM Gateway service...");

    let app = Router::new()
        .route("/health", get(health_check));

    let listener = TcpListener::bind("0.0.0.0:8001").await?;
    info!("IM Gateway listening on http://0.0.0.0:8001");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_check() -> &'static str {
    "IM Gateway is healthy"
}