use axum::{
    Router,
    routing::{get, post, put},
    Extension,
    extract::{State, Path, Query},
    Json,
    response::IntoResponse,
};
use tokio::net::TcpListener;
use tracing::info;
use sqlx::PgPool;

// 导入模块 - 使用 im_api:: 前缀访问库模块
use im_api::handlers::auth;
use im_api::handlers::message;
use im_api::handlers::conversation;
use im_api::middleware::auth::AuthUser;
use im_api::models::auth::ApiResponse;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 加载环境变量
    dotenvy::dotenv().ok();

    info!("Starting IM API service...");

    // 初始化数据库连接池
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:***@localhost/omnilink".to_string());

    let pool = PgPool::connect(&database_url).await?;

    info!("Connected to PostgreSQL database");

    // 初始化数据库表
    init_database(&pool).await?;

    // 创建路由
    let app = Router::new()
        // 健康检查
        .route("/health", get(health_check))

        // 认证路由
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))

        // 用户路由（需要认证）
        .route("/api/user/me", get(get_me_with_auth).put(update_me_with_auth))

        // 会话路由（需要认证）
        .route("/api/im/conversations", get(get_conversations_with_auth).post(create_conversation_with_auth))
        .route("/api/im/conversations/:id/messages", get(get_messages_with_auth).post(send_message_with_auth))
        .route("/api/im/conversations/:id/messages/:msg_id", put(edit_message_with_auth))
        .route("/api/im/conversations/:id/messages/:msg_id/recall", post(recall_message_with_auth))
        .route("/api/im/conversations/:id/read", post(mark_as_read_with_auth))

        // 添加数据库连接池到状态
        .with_state(pool);

    let listener = TcpListener::bind("0.0.0.0:8002").await?;
    info!("IM API listening on http://0.0.0.0:8002");

    axum::serve(listener, app).await?;
    Ok(())
}

/// 健康检查
async fn health_check() -> &'static str {
    "IM API is healthy"
}

/// 获取当前用户信息（包装认证中间件）
async fn get_me_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
) -> impl IntoResponse {
    // 将 AuthUser 转换为 Extension<user_id> 供 handler 使用
    let user_id = auth.user_id;
    auth::get_me(State(pool), Extension(user_id)).await
}

/// 更新用户资料（包装认证中间件）
async fn update_me_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let user_id = auth.user_id;

    // 将 JSON 转换为 UpdateUserRequest
    let update_req: Result<im_api::models::auth::UpdateUserRequest, _> =
        serde_json::from_value(req);

    match update_req {
        Ok(req) => auth::update_me(State(pool), Extension(user_id), Json(req)).await,
        Err(e) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(im_api::models::auth::ApiResponse::<serde_json::Value>::error("INVALID_JSON", format!("无效的请求数据: {}", e))),
        ),
    }
}

/// 获取会话列表（包装认证中间件）
async fn get_conversations_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    conversation::get_conversations(State(pool), Extension(user_id)).await
}

/// 创建会话（包装认证中间件）
async fn create_conversation_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<im_api::models::conversation::CreateConversationRequest>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    conversation::create_conversation_handler(State(pool), Extension(user_id), Json(req)).await
}

/// 获取消息（包装认证中间件）
async fn get_messages_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
    Query(query): Query<message::GetMessagesQuery>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    message::get_messages(State(pool), Extension(user_id), Path(conversation_id), Query(query)).await
}

/// 发送消息（包装认证中间件）
async fn send_message_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
    Json(req): Json<im_api::models::message::SendMessageRequest>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    message::send_message(State(pool), Extension(user_id), Path(conversation_id), Json(req)).await
}

/// 编辑消息（包装认证中间件）
async fn edit_message_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path((conversation_id, message_id)): Path<(String, String)>,
    Json(req): Json<im_api::models::message::EditMessageRequest>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    message::edit_message(State(pool), Extension(user_id), Path((conversation_id, message_id)), Json(req)).await
}

/// 撤回消息（包装认证中间件）
async fn recall_message_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path((conversation_id, message_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    message::recall_message_handler(State(pool), Extension(user_id), Path((conversation_id, message_id))).await
}

/// 标记会话已读（包装认证中间件）
async fn mark_as_read_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    message::mark_as_read_handler(State(pool), Extension(user_id), Path(conversation_id)).await
}

/// 初始化数据库表
async fn init_database(pool: &PgPool) -> anyhow::Result<()> {
    info!("Initializing database tables...");

    // 创建用户表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY,
            username VARCHAR(20) UNIQUE NOT NULL,
            email VARCHAR(255) UNIQUE NOT NULL,
            password_hash VARCHAR(255) NOT NULL,
            avatar VARCHAR(500),
            created_at TIMESTAMP WITH TIME ZONE NOT NULL,
            updated_at TIMESTAMP WITH TIME ZONE NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
        CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
        "#,
    )
    .execute(pool)
    .await?;

    // 创建会话表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS conversations (
            id UUID PRIMARY KEY,
            type VARCHAR(20) NOT NULL CHECK (type IN ('direct', 'group', 'ai')),
            name VARCHAR(255),
            avatar VARCHAR(500),
            created_by UUID REFERENCES users(id),
            unread_count INTEGER DEFAULT 0 NOT NULL,
            is_pinned BOOLEAN DEFAULT FALSE NOT NULL,
            is_muted BOOLEAN DEFAULT FALSE NOT NULL,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL,
            updated_at TIMESTAMP WITH TIME ZONE NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_conversations_type ON conversations(type);
        CREATE INDEX IF NOT EXISTS idx_conversations_created_by ON conversations(created_by);
        CREATE INDEX IF NOT EXISTS idx_conversations_updated_at ON conversations(updated_at DESC);
        "#,
    )
    .execute(pool)
    .await?;

    // 创建会话参与者表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS conversation_participants (
            conversation_id UUID REFERENCES conversations(id) ON DELETE CASCADE,
            user_id UUID REFERENCES users(id) ON DELETE CASCADE,
            joined_at TIMESTAMP WITH TIME ZONE NOT NULL,
            PRIMARY KEY (conversation_id, user_id)
        );

        CREATE INDEX IF NOT EXISTS idx_conversation_participants_user_id ON conversation_participants(user_id);
        "#,
    )
    .execute(pool)
    .await?;

    // 创建消息表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            id UUID PRIMARY KEY,
            conversation_id UUID REFERENCES conversations(id) ON DELETE CASCADE,
            sender_id UUID REFERENCES users(id),
            content TEXT NOT NULL,
            type VARCHAR(20) NOT NULL DEFAULT 'text' CHECK (type IN ('text', 'image', 'file', 'system')),
            status VARCHAR(20) NOT NULL DEFAULT 'sent' CHECK (status IN ('sending', 'sent', 'delivered', 'read', 'failed')),
            reply_to UUID REFERENCES messages(id),
            metadata JSONB,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL,
            updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
            read_at TIMESTAMP WITH TIME ZONE
        );

        CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON messages(conversation_id);
        CREATE INDEX IF NOT EXISTS idx_messages_sender_id ON messages(sender_id);
        CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_messages_conversation_created ON messages(conversation_id, created_at DESC);
        "#,
    )
    .execute(pool)
    .await?;

    info!("Database tables initialized successfully");
    Ok(())
}
