use axum::{
    Router,
    routing::{get, post, put, delete},
    Extension,
    extract::{State, Path, Query},
    Json,
    response::IntoResponse,
    http::StatusCode,
};
use tokio::net::TcpListener;
use tracing::info;
use sqlx::PgPool;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

// 导入模块 - 使用 im_api:: 前缀访问库模块
use im_api::handlers::auth;
use im_api::handlers::message;
use im_api::handlers::conversation;
use im_api::handlers::health::health_check_with_deps;
use im_api::handlers::encryption;
use im_api::middleware::auth::AuthUser;
use im_api::middleware::rate_limit::{RateLimitConfig, RateLimitState, rate_limit_middleware};
use im_api::middleware::request_id::request_id_middleware;
use im_api::models::auth::ApiResponse;
use im_api::openapi::ApiDoc;

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

    // 确保 is_archived 列存在（兼容已有数据库）
    sqlx::query(
        "ALTER TABLE conversations ADD COLUMN IF NOT EXISTS is_archived BOOLEAN DEFAULT FALSE NOT NULL"
    )
    .execute(&pool)
    .await
    .ok(); // 忽略错误，列可能已存在

    // 创建 OpenAPI 文档
    let openapi = ApiDoc::openapi();

    // 创建速率限制状态
    let rate_limit_config = RateLimitConfig {
        max_requests: 100,
        window_duration: std::time::Duration::from_secs(60),
    };
    let rate_limit_state = RateLimitState::new(rate_limit_config);

    // 创建路由
    let app = Router::new()
        // 健康检查（标准化版本，包含依赖检查）
        .route("/health", get(health_check_with_deps))

        // Swagger UI
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi))

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
        .route("/api/im/conversations/:id/messages/:msg_id/forward", post(forward_message_with_auth))
        .route("/api/im/conversations/:id/read", post(mark_as_read_with_auth))

        // 消息搜索和统计
        .route("/api/im/conversations/:id/messages/search", get(search_messages_with_auth))
        .route("/api/im/conversations/:id/messages/stats", get(get_message_stats_with_auth))

        // 群组管理
        .route("/api/im/conversations/:id/members", get(get_group_members_with_auth).post(add_group_members_with_auth))
        .route("/api/im/conversations/:id/members/:member_id", delete(remove_group_member_with_auth))
        .route("/api/im/conversations/:id/group", put(update_group_info_with_auth))
        .route("/api/im/conversations/:id/announcement", get(get_group_announcement_with_auth).put(update_group_announcement_with_auth))

        // 会话管理增强（置顶、免打扰、归档、搜索）
        .route("/api/im/conversations/:id/pin", put(toggle_pin_with_auth))
        .route("/api/im/conversations/:id/mute", put(toggle_mute_with_auth))
        .route("/api/im/conversations/:id/archive", put(toggle_archive_with_auth))
        .route("/api/im/conversations/search", get(search_conversations_with_auth))

        // 标签管理
        .route("/api/im/tags", get(get_tags_with_auth).post(create_tag_with_auth))
        .route("/api/im/tags/:tag_id", delete(delete_tag_with_auth))
        .route("/api/im/conversations/:id/tags/:tag_id", post(add_tag_to_conversation_with_auth).delete(remove_tag_from_conversation_with_auth))
        .route("/api/im/conversations/:id/tags", get(get_conversation_tags_with_auth))

        // 加密相关路由
        .route("/api/im/encryption/keys", post(generate_encryption_keys_with_auth))
        .route("/api/im/encryption/session-key/:conversation_id", get(get_session_key_with_auth))
        .route("/api/im/encryption/encrypt", post(encrypt_message_with_auth))
        .route("/api/im/encryption/decrypt", post(decrypt_message_with_auth))
        .route("/api/im/encryption/info", get(get_encryption_info_with_auth))
        .route("/api/im/encryption/key-exchange", post(key_exchange_with_auth))
        .route("/api/im/encryption/store", post(store_encrypted_message_with_auth))
        .route("/api/im/encryption/messages/:conversation_id", get(get_encrypted_messages_with_auth))

        // 添加数据库连接池到状态
        .with_state(pool)
        // 添加请求追踪中间件层
        .layer(axum::middleware::from_fn(request_id_middleware))
        // 添加速率限制中间件层
        .layer(axum::middleware::from_fn_with_state(
            rate_limit_state,
            rate_limit_middleware,
        ));

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
    Query(query): Query<im_api::models::conversation::GetConversationsQuery>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    conversation::get_conversations(State(pool), Extension(user_id), Query(query)).await
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

/// 转发消息（包装认证中间件）
async fn forward_message_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path((conversation_id, message_id)): Path<(String, String)>,
    Json(req): Json<im_api::handlers::message::ForwardMessageRequest>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    message::forward_message(State(pool), Extension(user_id), Path((conversation_id, message_id)), Json(req)).await
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

/// 搜索消息（包装认证中间件）
async fn search_messages_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
    Query(query): Query<message::SearchMessagesQuery>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    message::search_messages(State(pool), Extension(user_id), Path(conversation_id), Query(query)).await
}

/// 获取消息统计（包装认证中间件）
async fn get_message_stats_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    message::get_message_stats_handler(State(pool), Extension(user_id), Path(conversation_id)).await
}

/// 获取群组成员列表（包装认证中间件）
async fn get_group_members_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    conversation::get_group_members(State(pool), Extension(user_id), Path(conversation_id)).await
}

/// 添加群组成员（包装认证中间件）
async fn add_group_members_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    conversation::add_group_members(State(pool), Extension(user_id), Path(conversation_id), Json(req)).await
}

/// 移除群组成员（包装认证中间件）
async fn remove_group_member_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path((conversation_id, member_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    conversation::remove_group_member(State(pool), Extension(user_id), Path((conversation_id, member_id))).await
}

/// 更新群组信息（包装认证中间件）
async fn update_group_info_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    conversation::update_group_info(State(pool), Extension(user_id), Path(conversation_id), Json(req)).await
}

/// 获取群公告（包装认证中间件）
async fn get_group_announcement_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    conversation::get_group_announcement(State(pool), Extension(user_id), Path(conversation_id)).await
}

/// 更新群公告（包装认证中间件）
async fn update_group_announcement_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    let announcement = req.get("announcement").and_then(|v| v.as_str()).unwrap_or("");
    conversation::update_group_announcement_handler(State(pool), Extension(user_id), Path(conversation_id), announcement).await
}

/// 切换会话置顶状态（包装认证中间件）
async fn toggle_pin_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    conversation::toggle_pin(State(pool), Extension(user_id), Path(conversation_id), Json(req)).await
}

/// 切换会话免打扰状态（包装认证中间件）
async fn toggle_mute_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    conversation::toggle_mute(State(pool), Extension(user_id), Path(conversation_id), Json(req)).await
}

/// 切换会话归档状态（包装认证中间件）
async fn toggle_archive_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    conversation::toggle_archive(State(pool), Extension(user_id), Path(conversation_id), Json(req)).await
}

/// 搜索会话（包装认证中间件）
async fn search_conversations_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    axum::extract::Query(query): axum::extract::Query<im_api::models::conversation::SearchConversationsQuery>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    conversation::search(State(pool), Extension(user_id), axum::extract::Query(query)).await
}

/// 生成加密密钥对（包装认证中间件）
async fn generate_encryption_keys_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
) -> impl IntoResponse {
    let user_id = match auth.user_id.parse::<uuid::Uuid>() {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(ApiResponse::<serde_json::Value>::error("INVALID_USER_ID", "无效的用户ID"))).into_response(),
    };
    encryption::generate_keys(State(pool), Extension(user_id)).await.into_response()
}

/// 获取会话密钥（包装认证中间件）
async fn get_session_key_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
) -> impl IntoResponse {
    let user_id = match auth.user_id.parse::<uuid::Uuid>() {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(ApiResponse::<serde_json::Value>::error("INVALID_USER_ID", "无效的用户ID"))).into_response(),
    };
    encryption::get_session_key(State(pool), Extension(user_id), Path(conversation_id)).await.into_response()
}

/// 加密消息（包装认证中间件）
async fn encrypt_message_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let user_id = match auth.user_id.parse::<uuid::Uuid>() {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(ApiResponse::<serde_json::Value>::error("INVALID_USER_ID", "无效的用户ID"))).into_response(),
    };
    encryption::encrypt_message(State(pool), Extension(user_id), Json(req)).await.into_response()
}

/// 解密消息（包装认证中间件）
async fn decrypt_message_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let user_id = match auth.user_id.parse::<uuid::Uuid>() {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(ApiResponse::<serde_json::Value>::error("INVALID_USER_ID", "无效的用户ID"))).into_response(),
    };
    encryption::decrypt_message(State(pool), Extension(user_id), Json(req)).await.into_response()
}

/// 获取加密信息（包装认证中间件）
async fn get_encryption_info_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
) -> impl IntoResponse {
    let user_id = match auth.user_id.parse::<uuid::Uuid>() {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(ApiResponse::<serde_json::Value>::error("INVALID_USER_ID", "无效的用户ID"))).into_response(),
    };
    encryption::get_encryption_info(State(pool), Extension(user_id)).await.into_response()
}

/// 密钥交换（包装认证中间件）
async fn key_exchange_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let user_id = match auth.user_id.parse::<uuid::Uuid>() {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(ApiResponse::<serde_json::Value>::error("INVALID_USER_ID", "无效的用户ID"))).into_response(),
    };
    encryption::key_exchange(State(pool), Extension(user_id), Json(req)).await.into_response()
}

/// 存储加密消息（包装认证中间件）
async fn store_encrypted_message_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let user_id = match auth.user_id.parse::<uuid::Uuid>() {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(ApiResponse::<serde_json::Value>::error("INVALID_USER_ID", "无效的用户ID"))).into_response(),
    };
    encryption::store_encrypted_message(State(pool), Extension(user_id), Json(req)).await.into_response()
}

/// 获取加密消息历史（包装认证中间件）
async fn get_encrypted_messages_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
) -> impl IntoResponse {
    let user_id = match auth.user_id.parse::<uuid::Uuid>() {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(ApiResponse::<serde_json::Value>::error("INVALID_USER_ID", "无效的用户ID"))).into_response(),
    };
    encryption::get_encrypted_messages(State(pool), Extension(user_id), Path(conversation_id)).await.into_response()
}

/// 创建标签（包装认证中间件）
async fn create_tag_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<im_api::models::conversation::CreateTagRequest>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    conversation::create_tag_handler(State(pool), Extension(user_id), Json(req)).await
}

/// 获取用户的所有标签（包装认证中间件）
async fn get_tags_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    conversation::get_tags_handler(State(pool), Extension(user_id)).await
}

/// 删除标签（包装认证中间件）
async fn delete_tag_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(tag_id): Path<String>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    conversation::delete_tag_handler(State(pool), Extension(user_id), Path(tag_id)).await
}

/// 给会话添加标签（包装认证中间件）
async fn add_tag_to_conversation_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path((conversation_id, tag_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    conversation::add_tag_to_conversation_handler(State(pool), Extension(user_id), Path((conversation_id, tag_id))).await
}

/// 移除会话的标签（包装认证中间件）
async fn remove_tag_from_conversation_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path((conversation_id, tag_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    conversation::remove_tag_from_conversation_handler(State(pool), Extension(user_id), Path((conversation_id, tag_id))).await
}

/// 获取会话的所有标签（包装认证中间件）
async fn get_conversation_tags_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    conversation::get_conversation_tags_handler(State(pool), Extension(user_id), Path(conversation_id)).await
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
            is_archived BOOLEAN DEFAULT FALSE NOT NULL,
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
        -- 启用pg_trgm扩展（用于ILIKE搜索优化）
        CREATE EXTENSION IF NOT EXISTS pg_trgm;
        
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
        
        -- 创建消息内容搜索索引（GIN索引，用于ILIKE搜索）
        CREATE INDEX IF NOT EXISTS idx_messages_content_gin ON messages USING gin (content gin_trgm_ops);
        "#,
    )
    .execute(pool)
    .await?;

    info!("Database tables initialized successfully");

    // 创建标签表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS conversation_tags (
            id UUID PRIMARY KEY,
            user_id UUID REFERENCES users(id) ON DELETE CASCADE,
            name VARCHAR(50) NOT NULL,
            color VARCHAR(20),
            created_at TIMESTAMP WITH TIME ZONE NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_conversation_tags_user_id ON conversation_tags(user_id);
        "#
    )
    .execute(pool)
    .await?;

    // 创建会话-标签关联表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS conversation_tag_links (
            conversation_id UUID REFERENCES conversations(id) ON DELETE CASCADE,
            tag_id UUID REFERENCES conversation_tags(id) ON DELETE CASCADE,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL,
            PRIMARY KEY (conversation_id, tag_id)
        );

        CREATE INDEX IF NOT EXISTS idx_conversation_tag_links_tag_id ON conversation_tag_links(tag_id);
        "#
    )
    .execute(pool)
    .await?;


    // 创建加密消息存储表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS encrypted_messages (
            id UUID PRIMARY KEY,
            conversation_id UUID NOT NULL,
            sender_id UUID NOT NULL,
            ciphertext TEXT NOT NULL,
            nonce VARCHAR(64) NOT NULL,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_encrypted_messages_conversation_id ON encrypted_messages(conversation_id);
        CREATE INDEX IF NOT EXISTS idx_encrypted_messages_created_at ON encrypted_messages(created_at);
        "#
    )
    .execute(pool)
    .await?;
    // 创建推送设备表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS push_devices (
            id UUID PRIMARY KEY,
            user_id UUID REFERENCES users(id) ON DELETE CASCADE,
            device_id VARCHAR(255) NOT NULL,
            device_type VARCHAR(20) NOT NULL CHECK (device_type IN ('ios', 'android', 'web', 'desktop')),
            push_token VARCHAR(500) NOT NULL,
            is_active BOOLEAN DEFAULT TRUE,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL,
            updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
            UNIQUE(user_id, device_id)
        );

        CREATE INDEX IF NOT EXISTS idx_push_devices_user_id ON push_devices(user_id);
        "#
    )
    .execute(pool)
    .await?;

    // 创建推送配置表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS push_config (
            id UUID PRIMARY KEY,
            config_key VARCHAR(100) UNIQUE NOT NULL,
            config_value TEXT NOT NULL,
            description VARCHAR(500),
            created_at TIMESTAMP WITH TIME ZONE NOT NULL,
            updated_at TIMESTAMP WITH TIME ZONE NOT NULL
        );
        "#
    )
    .execute(pool)
    .await?;

    info!("Tag and push tables initialized successfully");
    Ok(())
}
