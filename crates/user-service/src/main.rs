use axum::{Router, routing::{get, post, delete}, middleware};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

use common::{auth::TokenManager, db::DatabaseManager};
use crate::services::UserService;
use crate::handlers::*;
use crate::middleware::auth_middleware;

pub async fn run() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 加载配置
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://im_chat:password@localhost:5432/im_chat".to_string());
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://:password@localhost:6379/0".to_string());
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "your-secret-key-change-in-production".to_string());

    info!("Starting user service...");

    // 初始化数据库管理器
    let db_manager = DatabaseManager::new(&database_url, &redis_url).await?;
    let pool = db_manager.pg_pool().clone();
    let redis = db_manager.redis().clone();

    // 初始化Token管理器
    let token_manager = Arc::new(TokenManager::new(jwt_secret.as_bytes()));

    // 初始化用户服务
    let user_service = Arc::new(UserService::new(pool, redis, token_manager));

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
        .route("/user/devices/:device_id", delete(delete_device))
        .route("/user/account", delete(delete_account))
        .layer(middleware::from_fn_with_state(
            user_service.get_token_manager(),
            auth_middleware,
        ))
        .with_state(user_service);

    // 合并路由
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(middleware::from_fn(
            axum::middleware::middleware::from_fn(logging_middleware)
        ))
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

    info!("{} {} - {:?}ms", method, uri, duration.as_millis());

    response
}

async fn health_check() -> &'static str {
    "User service is healthy"
}

// 为UserService添加获取token_manager的方法
impl UserService {
    fn get_token_manager(&self) -> Arc<TokenManager> {
        self.token_manager.clone()
    }
}