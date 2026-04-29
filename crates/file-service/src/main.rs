use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tracing::info;

pub async fn run() -> anyhow::Result<()> {
    info!("Starting file service...");

    let app = Router::new()
        .route("/health", get(health_check));

    let listener = TcpListener::bind("0.0.0.0:8005").await?;
    info!("File service listening on http://0.0.0.0:8005");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_check() -> &'static str {
    "File service is healthy"
}