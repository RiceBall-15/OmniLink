/// OmniLink Push Service
/// 
/// 负责处理跨平台推送消息服务
/// 支持平台：APNs (iOS), FCM (Android), 极光推送
use axum::{
    routing::{get, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use common::db::create_pool;
use common::error::Result;

mod handlers;
mod models;
mod repository;
mod services;

use handlers::{AppState, *};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // 连接数据库
    let pool = create_pool().await?;
    
    // 创建推送服务实例
    let push_service = Arc::new(services::PushService::new(pool.clone()));

    // 构建应用状态
    let state = AppState { push_service };

    // 构建路由
    let app = Router::new()
        // 推送相关接口
        .route("/push/send", post(send_push))
        .route("/push/batch", post(batch_send_push))
        .route("/push/template", post(send_template_push))
        .route("/push/history", get(get_user_push_history))
        
        // 模板管理接口
        .route("/templates", post(create_template))
        .route("/templates", get(list_templates))
        .route("/templates/:name", axum::routing::delete(delete_template))
        
        // 统计和清理接口
        .route("/stats", get(get_push_stats))
        .route("/cleanup/:days", post(cleanup_old_messages))
        
        // 健康检查
        .route("/health", get(health_check))
        
        // 添加中间件
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
        )
        .with_state(state);

    // 启动服务
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3005").await?;
    tracing::info!("Push service listening on http://0.0.0.0:3005");
    
    axum::serve(listener, app).await?;

    Ok(())
}