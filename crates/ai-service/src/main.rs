use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tracing::info;

pub async fn run() -> anyhow::Result<()> {
    info!("Starting AI service...");

    let app = Router::new()
        .route("/health", get(health_check));

    let listener = TcpListener::bind("0.0.0.0:8003").await?;
    info!("AI service listening on http://0.0.0.0:8003");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_check() -> &'static str {
    "AI service is healthy"
}