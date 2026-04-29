use axum::{Router, routing};
use tokio::net::TcpListener;
use tracing::info;

pub struct GatewayServer {
    addr: String,
}

impl GatewayServer {
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            addr: addr.into(),
        }
    }

    pub async fn serve(self) -> anyhow::Result<()> {
        let app = self.router();

        let listener = TcpListener::bind(&self.addr).await?;
        info!("Gateway server listening on {}", self.addr);

        axum::serve(listener, app).await?;
        Ok(())
    }

    fn router(&self) -> Router {
        Router::new()
            .route("/health", routing::get(health_check))
            .route("/ws", routing::get(websocket_handler))
    }

    async fn websocket_handler(
        ws: axum::extract::WebSocketUpgrade,
    ) -> axum::response::Response {
        ws.on_upgrade(handle_socket)
    }
}

async fn handle_socket(mut socket: axum::extract::ws::WebSocket) {
    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(axum::extract::ws::Message::Text(text)) => {
                // Echo back
                if socket.send(axum::extract::ws::Message::Text(text)).await.is_err() {
                    break;
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
}

async fn health_check() -> &'static str {
    "Gateway OK"
}