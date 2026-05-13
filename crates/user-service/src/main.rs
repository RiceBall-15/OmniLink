use axum::{Router, routing::{get, post, delete}, middleware};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

use common::db::DatabaseManager;
use user_service::services::UserService;
use user_service::handlers::*;
use user_service::middleware::auth_middleware;
use user_service::jwt::JwtManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 加载配置
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://im_chat:***@localhost:5432/im_chat".to_string());
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://:password@localhost:6379/0".to_string());
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "your-secret-key".to_string());

    info!("Starting user service...");

    // 初始化数据库管理器
    let db_manager = DatabaseManager::new(&database_url, &redis_url).await?;
    let pool = db_manager.pg_pool().clone();
    let redis = db_manager.redis().clone();

    // 初始化 JWT 管理器
    let jwt_manager = Arc::new(JwtManager::new(jwt_secret.as_bytes()));

    // 初始化用户服务
    let user_service = Arc::new(UserService::new(pool, redis, jwt_manager));

    // 创建路由
    let app = create_router(user_service);

    // 启动服务
    let addr = "0.0.0.0:8004";
    let listener = TcpListener::bind(addr).await?;
    info!("User service listening on http://{}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

fn create_router(user_service: Arc<UserService>) -> Router {
    // 公开路由（不需要认证）
    let public_routes = Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh_token))
        .route("/health", get(health_check));

    // 需要认证的路由
    let protected_routes = Router::new()
        .route("/auth/logout", post(logout))
        .route("/user/profile", get(get_profile))
        .route("/user/profile", post(update_profile))
        .route("/user/password", post(change_password))
        .route("/user/devices", get(get_devices))
        .route("/user/devices/{device_id}", delete(delete_device))
        .route("/user/account", delete(delete_account))
        .route("/user/block/{user_id}", post(block_user))
        .route("/user/block/{user_id}", delete(unblock_user))
        .route("/user/block/{user_id}", get(check_blocked))
        .route("/user/blocked", get(get_blocked_users))
        .layer(middleware::from_fn_with_state(
            user_service.get_token_manager(),
            auth_middleware,
        ));

    // 合并路由并设置状态
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(middleware::from_fn(logging_middleware))
        .with_state(user_service)
}

async fn logging_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = req.method().clone();
    let uri = req.uri().clone();

    let start = std::time::Instant::now();
    let response = next.run(req).await;
    let duration = start.elapsed();

    info!("{} {} - {:?}", method, uri, duration.as_millis());

    response
}

async fn health_check() -> &'static str {
    "User service is healthy"
}
