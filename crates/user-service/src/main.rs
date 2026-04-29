use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tracing::info;

pub async fn run() -> anyhow::Result<()> {
    info!("Starting user service...");

    let app = Router::new()
        .route("/health", get(health_check));

    let listener = TcpListener::bind("0.0.0.0:8004").await?;
    info!("User service listening on http://0.0.0.0:8004");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_check() -> &'static str {
    "User service is healthy"
}