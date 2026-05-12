/// OmniLink Push Service
///
/// 负责处理跨平台推送消息服务
/// 支持平台：APNs (iOS), FCM (Android), Web Push
use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use common::db::DatabaseManager;
use common::error::Result;

use push_service::handlers::{self, AppState};
use push_service::services::PushService;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // 读取环境变量
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://omnilink:omnilink@localhost/omnilink".to_string());
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string());

    // 连接数据库
    let db = DatabaseManager::new(&database_url, &redis_url).await?;
    let pool = db.pg_pool().clone();

    // 创建推送服务实例
    let push_service = Arc::new(PushService::new(pool));

    // 构建应用状态
    let state = AppState { push_service };

    // 构建路由
    let app = Router::new()
        // 推送相关接口
        .route("/push/send", post(handlers::send_push))
        .route("/push/batch", post(handlers::batch_send_push))
        .route("/push/template", post(handlers::send_template_push))
        .route("/push/history", get(handlers::get_user_push_history))
        .route("/push/test", post(handlers::send_test_push))
        // 模板管理接口
        .route("/templates", post(handlers::create_template))
        .route("/templates", get(handlers::list_templates))
        .route("/templates/{name}", delete(handlers::delete_template))
        // 设备管理接口
        .route("/devices", post(handlers::register_device))
        .route("/devices", get(handlers::get_user_devices))
        .route("/devices/{id}", delete(handlers::unregister_device))
        // 通知偏好接口
        .route("/preferences", get(handlers::get_notification_preferences))
        .route("/preferences", post(handlers::update_notification_preferences))
        // 推送配置管理接口
        .route("/configs", get(handlers::get_push_configs))
        .route("/configs", post(handlers::upsert_push_config))
        .route("/configs/{key}", delete(handlers::delete_push_config))
        // 统计、监控和清理接口
        .route("/stats", get(handlers::get_push_stats))
        .route("/health", get(handlers::get_push_health))
        .route("/cleanup/{days}", post(handlers::cleanup_old_messages))
        // 健康检查
        .route("/ping", get(handlers::health_check))
        // 添加中间件
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        )
        .with_state(state);

    // 启动服务
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3005").await?;
    tracing::info!("Push service listening on http://0.0.0.0:3005");

    axum::serve(listener, app).await?;

    Ok(())
}
