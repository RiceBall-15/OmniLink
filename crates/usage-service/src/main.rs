use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tracing::info;

pub async fn run() -> anyhow::Result<()> {
    info!("Starting usage service...");

    let app = Router::new()
        .route("/health", get(health_check));

    let listener = TcpListener::bind("0.0.0.0:8006").await?;
    info!("Usage service listening on http://0.0.0.0:8006");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_check() -> &'static str {
    "Usage service is healthy"
}