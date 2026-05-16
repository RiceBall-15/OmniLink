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
            .route("/metrics", routing::get(metrics_handler))
            .route("/ws", routing::get(websocket_handler))
            .route("/api/quality/stats", routing::get(quality_stats_handler))
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

async fn metrics_handler() -> axum::Json<serde_json::Value> {
    // 返回基础系统指标
    let (mem_used, mem_total) = {
        #[cfg(target_os = "linux")]
        {
            if let Ok(info) = std::fs::read_to_string("/proc/meminfo") {
                let mut total = 0u64;
                let mut available = 0u64;
                for line in info.lines() {
                    if line.starts_with("MemTotal:") {
                        total = line.split_whitespace().nth(1)
                            .and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                    } else if line.starts_with("MemAvailable:") {
                        available = line.split_whitespace().nth(1)
                            .and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                    }
                }
                ((total - available) / 1024, total / 1024)
            } else {
                (0, 0)
            }
        }
        #[cfg(not(target_os = "linux"))]
        { (0, 0) }
    };

    axum::Json(serde_json::json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "system": {
            "memory_used_mb": mem_used,
            "memory_total_mb": mem_total,
            "memory_usage_percent": if mem_total > 0 { (mem_used as f64 / mem_total as f64 * 100.0) as u64 } else { 0 },
        }
    }))
}

/// 连接质量统计 API
///
/// 返回当前所有连接的质量指标摘要
async fn quality_stats_handler() -> axum::Json<serde_json::Value> {
    // 返回连接质量统计占位数据
    // 实际实现需要从 AckManager 获取连接质量数据
    axum::Json(serde_json::json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "connection_quality": {
            "total_connections": 0,
            "excellent": 0,
            "good": 0,
            "fair": 0,
            "poor": 0,
        },
        "message_stats": {
            "total_sent": 0,
            "total_acked": 0,
            "total_timeout": 0,
            "overall_loss_rate": 0.0,
        },
        "degradation_summary": {
            "full_speed": 0,
            "normal": 0,
            "batched": 0,
            "priority_only": 0,
        }
    }))
}