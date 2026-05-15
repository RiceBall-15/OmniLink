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
use uuid::Uuid;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

// 导入模块 - 使用 im_api:: 前缀访问库模块
use im_api::handlers::auth;
use im_api::handlers::message;
use im_api::handlers::conversation;
use im_api::handlers::health::health_check_with_deps;
use im_api::handlers::encryption;
use im_api::handlers::metrics::{get_metrics, get_prometheus_metrics, init_start_time};
use im_api::handlers::audit;
use im_api::handlers::contact;
use im_api::handlers::message_retry;
use im_api::handlers::quick_reply;
use im_api::handlers::feedback;
use im_api::handlers::chat_export;
use im_api::handlers::user_preferences;
use im_api::handlers::webhook as webhook_handlers;
use im_api::handlers::data_retention;
use im_api::handlers::admin as admin_handlers;
use im_api::handlers::user_activity as activity_handlers;
use im_api::middleware::auth::AuthUser;
use im_api::middleware::error_capture::error_capture_middleware;
use im_api::middleware::security_headers::security_headers_middleware;
use im_api::middleware::etag::etag_middleware;
use im_api::middleware::rate_limit::{RateLimitConfig, RateLimitState, rate_limit_middleware, get_rate_limit_config, update_rate_limit_config};
use im_api::middleware::request_id::request_id_middleware;
use im_api::middleware::request_timing::request_timing_middleware;
use im_api::models::auth::ApiResponse;
use im_api::openapi::ApiDoc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化可动态调整的结构化日志（JSON格式）
    common::log_level::init_dynamic_logging("info");

    // 加载环境变量
    dotenvy::dotenv().ok();

    info!("Starting IM API service...");

    // 初始化启动时间（用于指标统计）
    init_start_time();

    // 初始化数据库连接池（优化配置）
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost/omnilink".to_string());

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)          // 最大连接数：适配2核2G服务器
        .min_connections(2)           // 最小连接数：保持基本连接池
        .acquire_timeout(std::time::Duration::from_secs(15))  // 获取连接超时：15秒
        .idle_timeout(std::time::Duration::from_secs(300))    // 空闲连接超时：5分钟
        .max_lifetime(std::time::Duration::from_secs(1800))   // 连接最大生命周期：30分钟
        .connect(&database_url)
        .await?;

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

    // 确保用户资料字段存在（兼容已有数据库）
    sqlx::query(
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS nickname VARCHAR(50)"
    )
    .execute(&pool)
    .await
    .ok();

    sqlx::query(
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS bio VARCHAR(500)"
    )
    .execute(&pool)
    .await
    .ok();

    sqlx::query(
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS status_message VARCHAR(100)"
    )
    .execute(&pool)
    .await
    .ok();

    // 创建 OpenAPI 文档
    let openapi = ApiDoc::openapi();

    // 创建速率限制状态
    let rate_limit_config = RateLimitConfig {
        max_requests: 100,
        window_duration: std::time::Duration::from_secs(60),
        whitelist_ips: vec!["127.0.0.1".to_string(), "::1".to_string()],
        authenticated_max_requests: Some(200), // 认证用户有更高的限额
    };
    let rate_limit_state = RateLimitState::new(rate_limit_config);

    // 创建路由
    let app = Router::new()
        // 健康检查（标准化版本，包含依赖检查）
        .route("/health", get(health_check_with_deps))
        .route("/metrics", get(get_metrics))
        .route("/metrics/prometheus", get(get_prometheus_metrics))

        // Swagger UI
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi))

        // 认证路由
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))

        // 用户路由（需要认证）
        .route("/api/user/me", get(get_me_with_auth).put(update_me_with_auth))
        .route("/api/user/profile", put(update_profile_with_auth))
        .route("/api/user/:id/profile", get(get_user_profile_with_auth))

        // 会话路由（需要认证）
        .route("/api/im/conversations", get(get_conversations_with_auth).post(create_conversation_with_auth))
        .route("/api/im/conversations/:id/messages", get(get_messages_with_auth).post(send_message_with_auth))
        .route("/api/im/conversations/:id/messages/:msg_id", put(edit_message_with_auth))
        .route("/api/im/conversations/:id/messages/:msg_id/recall", post(recall_message_with_auth))
        // 消息表情回应
        .route("/api/im/messages/:id/reactions", post(add_reaction_with_auth).delete(remove_reaction_with_auth).get(get_reactions_with_auth))

        // 消息收藏/书签
        .route("/api/im/messages/:id/bookmark", post(add_bookmark_with_auth).delete(remove_bookmark_with_auth).get(check_bookmark_with_auth))
        .route("/api/im/bookmarks", get(get_bookmarks_with_auth))

        // 草稿消息
        .route("/api/im/conversations/:id/draft", put(save_draft_with_auth).get(get_draft_with_auth).delete(delete_draft_with_auth))
        .route("/api/im/drafts", get(get_all_drafts_with_auth))

        // 定时消息
        .route("/api/im/messages/scheduled", post(create_scheduled_message_with_auth).get(get_scheduled_messages_with_auth))
        .route("/api/im/messages/scheduled/:id", get(get_scheduled_message_with_auth).put(update_scheduled_message_with_auth).delete(cancel_scheduled_message_with_auth))

        // 会话置顶消息
        .route("/api/im/conversations/:id/pinned-messages", get(get_pinned_messages_with_auth).post(pin_message_with_auth))
        .route("/api/im/conversations/:id/pinned-messages/:msg_id", delete(unpin_message_with_auth))
        .route("/api/im/messages/:msg_id/reactions/:emoji", delete(remove_reaction_with_auth))
        .route("/api/im/conversations/:id/read", post(mark_as_read_with_auth))

        // 消息搜索和统计
        .route("/api/im/conversations/:id/messages/search", get(search_messages_with_auth))
        .route("/api/im/messages/search", get(global_search_messages_with_auth))
        .route("/api/im/conversations/:id/messages/stats", get(get_message_stats_with_auth))

        // 消息投递状态跟踪
        .route("/api/im/messages/delivery-receipt", post(record_delivery_receipt_with_auth))
        .route("/api/im/messages/:id/delivery-receipts", get(get_delivery_receipts_with_auth))
        .route("/api/im/messages/:id/delivery-stats", get(get_delivery_stats_with_auth))

        // 批量操作
        .route("/api/im/messages/batch/send", post(batch_send_messages_with_auth))
        .route("/api/im/messages/batch/delete", post(batch_delete_messages_with_auth))
        .route("/api/im/messages/batch/mark-read", post(batch_mark_as_read_with_auth))

        // 阅后即焚
        .route("/api/im/conversations/:id/messages/:msg_id/read-burn", post(mark_single_message_read_with_auth))
        .route("/api/im/conversations/:id/expiring-messages", get(get_expiring_messages_with_auth))
        .route("/api/im/messages/cleanup-burn", post(cleanup_burn_messages_with_auth))

        // 系统公告
        .route("/api/announcements", get(get_active_announcements_with_auth))
        .route("/api/announcements/:id", get(get_announcement_with_auth))
        .route("/api/announcements/:id/read", post(mark_announcement_read_with_auth))
        .route("/api/announcements/unread-count", get(get_unread_announcement_count_with_auth))
        .route("/api/admin/announcements", get(get_all_announcements_with_auth).post(create_announcement_with_auth))
        .route("/api/admin/announcements/:id", put(update_announcement_with_auth).delete(delete_announcement_with_auth))

        // 快捷回复模板
        .route("/api/users/quick-replies", get(get_quick_replies_with_auth).post(create_quick_reply_with_auth))
        .route("/api/users/quick-replies/:id", get(get_quick_reply_with_auth).put(update_quick_reply_with_auth).delete(delete_quick_reply_with_auth))
        .route("/api/admin/quick-replies", post(create_global_quick_reply_with_auth))

        // 用户屏蔽
        .route("/api/users/:id/block", post(block_user_with_auth).delete(unblock_user_with_auth))
        .route("/api/users/blocked", get(get_blocked_list_with_auth))
        .route("/api/users/:id/block-status", get(check_block_status_with_auth))

        // 用户在线状态
        .route("/api/users/status", put(update_user_status_with_auth))
        .route("/api/users/:id/status", get(get_user_status_with_auth))

        // 群组管理
        .route("/api/im/conversations/:id/members", get(get_group_members_with_auth).post(add_group_members_with_auth))
        .route("/api/im/conversations/:id/members/:member_id", delete(remove_group_member_with_auth))
        .route("/api/im/conversations/:id/members/:member_id/role", put(update_member_role_with_auth))
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
        // 审计日志 API
        .route("/api/audit/logs", get(audit::get_audit_logs))
        .route("/api/audit/stats", get(audit::get_audit_stats))
        .route("/api/audit/user", get(audit::get_user_audit_logs))
        .route("/api/audit/cleanup", post(audit::cleanup_audit_logs))
        // 会话通知偏好 API
        .route("/api/im/conversations/:id/notification-settings", get(conversation::get_notification_settings))
        .route("/api/im/conversations/:id/notification-settings", put(conversation::update_notification_settings))
        .route("/api/im/conversations/:id/notification-settings", delete(conversation::reset_notification_settings))
        // 全局通知设置 API
        .route("/api/im/notifications/settings", get(conversation::get_global_notification_settings))
        .route("/api/im/notifications/settings", put(conversation::update_global_notification_settings))
        .route("/api/im/notifications/dnd-status", get(conversation::get_dnd_status))
        // 限流配置管理 API（热更新，无需重启）
        .route("/api/admin/rate-limit", get(get_rate_limit_config).put(update_rate_limit_config))
        // 日志级别动态调整 API
        .route("/api/admin/log-level", get(get_log_level).put(update_log_level));

        // 消息线程/话题回复 API
        let app = app
            .route("/api/im/messages/:id/thread", get(message::get_message_thread_handler))
            .route("/api/im/messages/:id/thread/count", get(message::get_thread_count_handler))
            .route("/api/im/conversations/:id/threads", get(message::get_conversation_threads_handler));

        // 消息重试队列 API
        let app = app
            .route("/api/im/messages/:id/retry", post(message_retry::retry_message_handler))
            .route("/api/im/messages/failed", get(message_retry::get_failed_messages_handler))
            .route("/api/im/messages/:id/retry-status", get(message_retry::get_retry_status_handler));

        // 联系人管理 API
        let app = app
            .route("/api/users/contacts", post(contact::add_contact_handler))
            .route("/api/users/contacts", get(contact::get_contacts_handler))
            .route("/api/users/contacts/:id", get(contact::get_contact_handler))
            .route("/api/users/contacts/:id", put(contact::update_contact_handler))
            .route("/api/users/contacts/:id", delete(contact::remove_contact_handler))
            .route("/api/users/search", get(contact::search_users_handler));

        // 用户反馈 API
        let app = app
            .route("/api/users/feedbacks", post(feedback::submit_feedback_handler))
            .route("/api/users/feedbacks", get(feedback::get_my_feedbacks_handler))
            .route("/api/users/feedbacks/:id", get(feedback::get_feedback_handler))
            .route("/api/admin/feedbacks", get(feedback::get_all_feedbacks_handler))
            .route("/api/admin/feedbacks/:id", put(feedback::update_feedback_handler))
            .route("/api/admin/feedbacks/:id", delete(feedback::delete_feedback_handler))
            .route("/api/admin/feedbacks/stats", get(feedback::get_feedback_stats_handler));

        // 聊天记录导出 API
        let app = app
            .route("/api/im/conversations/:id/export", post(chat_export::create_export_job_handler))
            .route("/api/im/exports/:id", get(chat_export::get_export_job_handler))
            .route("/api/im/exports/:id/download", get(chat_export::download_export_file_handler))
            .route("/api/im/exports", get(chat_export::list_user_export_jobs_handler));

        // 用户偏好设置 API
        let app = app
            .route("/api/users/preferences", get(user_preferences::get_preferences))
            .route("/api/users/preferences", put(user_preferences::set_preference))
            .route("/api/users/preferences", delete(user_preferences::delete_preference))
            .route("/api/users/preferences/batch", put(user_preferences::batch_set_preferences))
            .route("/api/users/preferences/category/:category", delete(user_preferences::delete_category))
            .route("/api/users/preferences/templates", get(user_preferences::get_templates))
            .route("/api/users/preferences/templates/apply", post(user_preferences::apply_templates));

        // Webhook 管理 API
        let app = app
            .route("/api/webhooks", post(webhook_handlers::create_webhook))
            .route("/api/webhooks", get(webhook_handlers::get_webhooks))
            .route("/api/webhooks/:id", get(webhook_handlers::get_webhook))
            .route("/api/webhooks/:id", put(webhook_handlers::update_webhook))
            .route("/api/webhooks/:id", delete(webhook_handlers::delete_webhook))
            .route("/api/webhooks/:id/deliveries", get(webhook_handlers::get_deliveries))
            // 数据保留策略 API
            .route("/api/admin/retention", post(data_retention::create_policy))
            .route("/api/admin/retention", get(data_retention::get_policies))
            .route("/api/admin/retention/:id", get(data_retention::get_policy))
            .route("/api/admin/retention/:id", put(data_retention::update_policy))
            .route("/api/admin/retention/:id", delete(data_retention::delete_policy))
            .route("/api/admin/retention/cleanup", post(data_retention::run_cleanup));

        // 管理员用户管理 API (Task 99)
        let app = app
            .route("/api/admin/users", get(admin_handlers::list_users))
            .route("/api/admin/users/:id", get(admin_handlers::get_user_detail))
            .route("/api/admin/users/:id/status", put(admin_handlers::update_user_status))
            .route("/api/admin/users/:id/force-logout", post(admin_handlers::force_logout_user))
            .route("/api/admin/users/:id/activity", get(admin_handlers::get_user_activity));

        // 用户活动追踪 API (Task 101) & 会话统计摘要 (Task 100)
        let app = app
            .route("/api/users/activity", get(activity_handlers::get_my_activity))
            .route("/api/im/conversations/:id/stats", get(activity_handlers::get_conversation_stats));

    // 克隆连接池用于后台定时消息处理任务
    let bg_pool = pool.clone();

    let app = app
        // 添加数据库连接池到状态
        .with_state(pool)
        // 添加限流状态到请求扩展（供管理 API 使用）
        .layer(axum::Extension(rate_limit_state.clone()))
        // 添加全局错误捕获中间件层（最外层，捕获所有未处理错误）
        .layer(axum::middleware::from_fn(error_capture_middleware))
        // 添加结构化请求日志中间件层
        .layer(tower_http::trace::TraceLayer::new_for_http()
            .make_span_with(tower_http::trace::DefaultMakeSpan::new()
                .level(tracing::Level::INFO)
                .include_headers(false))
            .on_request(tower_http::trace::DefaultOnRequest::new()
                .level(tracing::Level::INFO))
            .on_response(tower_http::trace::DefaultOnResponse::new()
                .level(tracing::Level::INFO))
            .on_failure(tower_http::trace::DefaultOnFailure::new()
                .level(tracing::Level::ERROR)))
        // 添加请求耗时中间件层
        .layer(axum::middleware::from_fn(request_timing_middleware))
        // 添加请求追踪中间件层
        .layer(axum::middleware::from_fn(request_id_middleware))
        // 添加速率限制中间件层
        .layer(axum::middleware::from_fn_with_state(
            rate_limit_state,
            rate_limit_middleware,
        ))
        // 添加安全头中间件层
        .layer(axum::middleware::from_fn(security_headers_middleware))
        // 添加 ETag 缓存验证中间件层
        .layer(axum::middleware::from_fn(etag_middleware))
        // 添加CORS中间件层
        .layer(tower_http::cors::CorsLayer::new()
            .allow_origin(tower_http::cors::Any)
            .allow_headers([
                axum::http::header::AUTHORIZATION,
                axum::http::header::ACCEPT,
                axum::http::header::CONTENT_TYPE,
            ])
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PUT,
                axum::http::Method::DELETE,
                axum::http::Method::PATCH,
                axum::http::Method::OPTIONS,
            ])
            .allow_credentials(true)
            .max_age(std::time::Duration::from_secs(3600))
        )
        // 添加 API 响应压缩中间件层（gzip/brotli）
        .layer(im_api::middleware::compression::create_compression_layer());

    // 启动定时消息后台处理任务
    im_api::handlers::scheduled_task::start_scheduled_message_processor(bg_pool.clone());
    info!("定时消息后台处理任务已启动");

    // 启动阅后即焚消息清理后台任务
    im_api::handlers::scheduled_task::start_burn_message_cleanup(bg_pool.clone());
    info!("阅后即焚消息清理后台任务已启动");

    // 启动聊天记录导出后台处理任务
    im_api::handlers::export_worker::start_export_worker(bg_pool);
    info!("聊天记录导出后台处理任务已启动");

    let listener = TcpListener::bind("0.0.0.0:8002").await?;
    info!("IM API listening on http://0.0.0.0:8002");

    axum::serve(listener, app).await?;
    Ok(())
}

/// 健康检查（简单版本，已被 health_check_with_deps 替代）
#[allow(dead_code)]
async fn health_check() -> &'static str {
    "IM API is healthy"
}

/// 获取当前日志级别
async fn get_log_level() -> impl IntoResponse {
    let level = common::log_level::get_log_level();
    Json(serde_json::json!({
        "code": 200,
        "message": "获取成功",
        "data": {
            "current_level": level,
            "available_levels": ["trace", "debug", "info", "warn", "error"],
            "supports_module_filter": true,
            "examples": [
                "info",
                "debug",
                "im_api=debug,tower_http=info",
                "im_api::handlers=trace,common=warn"
            ]
        }
    }))
}

/// 动态调整日志级别
async fn update_log_level(
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let level = match payload.get("level").and_then(|v| v.as_str()) {
        Some(l) => l,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "code": 400,
                    "message": "缺少 level 字段",
                    "example": { "level": "debug" }
                })),
            );
        }
    };

    match common::log_level::set_log_level(level) {
        Ok(()) => {
            info!(level = level, "日志级别已通过 API 更新");
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "code": 200,
                    "message": format!("日志级别已更新为: {}", level),
                    "data": { "level": level }
                })),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "code": 400,
                "message": e
            })),
        ),
    }
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

/// 更新用户扩展资料（包装认证中间件）
async fn update_profile_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let user_id = auth.user_id;

    // 将 JSON 转换为 UpdateUserProfileRequest
    let update_req: Result<im_api::models::auth::UpdateUserProfileRequest, _> =
        serde_json::from_value(req);

    match update_req {
        Ok(req) => auth::update_profile(State(pool), Extension(user_id), Json(req)).await,
        Err(e) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(im_api::models::auth::ApiResponse::<serde_json::Value>::error("INVALID_JSON", format!("无效的请求数据: {}", e))),
        ),
    }
}

/// 获取指定用户公开资料（包装认证中间件）
async fn get_user_profile_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(target_user_id): Path<String>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    auth::get_user_profile(State(pool), Extension(user_id), Path(target_user_id)).await
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
#[allow(dead_code)]
async fn forward_message_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path((conversation_id, message_id)): Path<(String, String)>,
    Json(req): Json<im_api::handlers::message::ForwardMessageRequest>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    message::forward_message(State(pool), Extension(user_id), Path((conversation_id, message_id)), Json(req)).await
}

/// 添加表情回应（包装认证中间件）
async fn add_reaction_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(message_id): Path<String>,
    Json(req): Json<im_api::models::message::AddReactionRequest>,
) -> impl IntoResponse {
    message::add_reaction(State(pool), auth, Path(message_id), Json(req)).await
}

/// 删除表情回应（包装认证中间件）
async fn remove_reaction_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path((message_id, emoji)): Path<(String, String)>,
) -> impl IntoResponse {
    message::remove_reaction(State(pool), auth, Path((message_id, emoji))).await
}

/// 获取表情回应列表（包装认证中间件）
async fn get_reactions_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(message_id): Path<String>,
) -> impl IntoResponse {
    message::get_reactions(State(pool), auth, Path(message_id)).await
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

/// 全局搜索消息（跨会话搜索，包装认证中间件）
async fn global_search_messages_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Query(query): Query<message::GlobalSearchMessagesQuery>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    message::search_all_messages(State(pool), Extension(user_id), Query(query)).await
}

/// 置顶消息（包装认证中间件）
async fn pin_message_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
    Json(request): Json<message::PinMessageRequest>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    message::pin_message(State(pool), Extension(user_id), Path(conversation_id), Json(request)).await
}

/// 取消置顶消息（包装认证中间件）
async fn unpin_message_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path((conversation_id, message_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    message::unpin_message(State(pool), Extension(user_id), Path((conversation_id, message_id))).await
}

/// 获取置顶消息列表（包装认证中间件）
async fn get_pinned_messages_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    message::get_pinned_messages(State(pool), Extension(user_id), Path(conversation_id)).await
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

/// 批量发送消息（包装认证中间件）
async fn batch_send_messages_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<im_api::models::message::BatchSendMessageRequest>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    message::batch_send_messages(Extension(pool), Extension(user_id), Json(req)).await
}

/// 批量删除消息（包装认证中间件）
async fn batch_delete_messages_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<im_api::models::message::BatchDeleteMessagesRequest>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    message::batch_delete_messages(Extension(pool), Extension(user_id), Json(req)).await
}

/// 批量标记已读（包装认证中间件）
async fn batch_mark_as_read_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<im_api::models::message::BatchMarkReadRequest>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    message::batch_mark_as_read(Extension(pool), Extension(user_id), Json(req)).await
}

/// 标记单条消息已读并处理阅后即焚（包装认证中间件）
async fn mark_single_message_read_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path((conversation_id, message_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    message::mark_single_message_read_handler(State(pool), Extension(user_id), Path((conversation_id, message_id))).await
}

/// 获取即将焚毁的消息列表（包装认证中间件）
async fn get_expiring_messages_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    message::get_expiring_messages_handler(State(pool), Extension(user_id), Path(conversation_id)).await
}

/// 清理过期阅后即焚消息（包装认证中间件）
async fn cleanup_burn_messages_with_auth(
    State(pool): State<PgPool>,
) -> impl IntoResponse {
    message::cleanup_burn_messages_handler(State(pool)).await
}

/// 记录消息投递状态（包装认证中间件）
async fn record_delivery_receipt_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<im_api::models::message::CreateDeliveryReceiptRequest>,
) -> impl IntoResponse {
    let user_id = auth.user_id.parse::<Uuid>().unwrap_or_default();
    message::record_delivery_receipt_handler(State(pool), Extension(user_id), Json(req)).await
}

/// 获取消息投递状态列表（包装认证中间件）
async fn get_delivery_receipts_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(message_id): Path<Uuid>,
) -> impl IntoResponse {
    let user_id = auth.user_id.parse::<Uuid>().unwrap_or_default();
    message::get_delivery_receipts_handler(State(pool), Extension(user_id), Path(message_id)).await
}

/// 获取消息投递统计（包装认证中间件）
async fn get_delivery_stats_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(message_id): Path<Uuid>,
    Query(params): Query<serde_json::Value>,
) -> impl IntoResponse {
    let user_id = auth.user_id.parse::<Uuid>().unwrap_or_default();
    message::get_delivery_stats_handler(State(pool), Extension(user_id), Path(message_id), Query(params)).await
}

/// 获取活跃公告列表（用户视图）
async fn get_active_announcements_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Query(params): Query<im_api::handlers::announcement::PaginationParams>,
) -> impl IntoResponse {
    im_api::handlers::announcement::get_active_announcements_handler(
        State(pool),
        auth,
        Query(params),
    )
    .await
}

/// 获取单个公告详情
async fn get_announcement_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> impl IntoResponse {
    im_api::handlers::announcement::get_announcement_handler(State(pool), auth, Path(id)).await
}

/// 标记公告为已读
async fn mark_announcement_read_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> impl IntoResponse {
    im_api::handlers::announcement::mark_announcement_read_handler(
        State(pool),
        auth,
        Path(id),
    )
    .await
}

/// 获取未读公告数量
async fn get_unread_announcement_count_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
) -> impl IntoResponse {
    im_api::handlers::announcement::get_unread_announcement_count_handler(State(pool), auth).await
}

/// 获取全部公告列表（管理员视图）
async fn get_all_announcements_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Query(params): Query<im_api::handlers::announcement::PaginationParams>,
) -> impl IntoResponse {
    im_api::handlers::announcement::get_all_announcements_handler(
        State(pool),
        auth,
        Query(params),
    )
    .await
}

/// 创建系统公告（管理员）
async fn create_announcement_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<im_api::models::announcement::CreateAnnouncementRequest>,
) -> impl IntoResponse {
    im_api::handlers::announcement::create_announcement_handler(State(pool), auth, Json(req)).await
}

/// 更新系统公告（管理员）
async fn update_announcement_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<im_api::handlers::announcement::UpdateAnnouncementRequest>,
) -> impl IntoResponse {
    im_api::handlers::announcement::update_announcement_handler(
        State(pool),
        auth,
        Path(id),
        Json(req),
    )
    .await
}

/// 删除系统公告（管理员）
async fn delete_announcement_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> impl IntoResponse {
    im_api::handlers::announcement::delete_announcement_handler(State(pool), auth, Path(id)).await
}

/// 屏蔽用户（包装认证中间件）
async fn block_user_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<im_api::models::auth::BlockUserRequest>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    auth::block_user_handler(State(pool), Extension(user_id), Json(req)).await
}

/// 取消屏蔽用户（包装认证中间件）
async fn unblock_user_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(blocked_user_id): Path<String>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    auth::unblock_user_handler(State(pool), Extension(user_id), Path(blocked_user_id)).await
}

/// 获取屏蔽列表（包装认证中间件）
async fn get_blocked_list_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    auth::get_blocked_list_handler(State(pool), Extension(user_id)).await
}

/// 检查屏蔽状态（包装认证中间件）
async fn check_block_status_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(other_user_id): Path<String>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    auth::check_block_status_handler(State(pool), Extension(user_id), Path(other_user_id)).await
}

/// 更新用户在线状态（包装认证中间件）
async fn update_user_status_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<im_api::models::message::UpdateStatusRequest>,
) -> impl IntoResponse {
    auth::update_user_status_handler(State(pool), auth, Json(req)).await
}

/// 获取用户在线状态详情（包装认证中间件）
async fn get_user_status_with_auth(
    State(pool): State<PgPool>,
    Path(user_id): Path<String>,
) -> impl IntoResponse {
    auth::get_user_status_handler(State(pool), Path(user_id)).await
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
    Path((id, uid)): Path<(String, String)>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    conversation::remove_group_member(State(pool), Extension(user_id), Path((id, uid))).await
}

/// 更新成员角色（包装认证中间件）
async fn update_member_role_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path((id, member_id)): Path<(String, String)>,
    Json(req): Json<im_api::models::conversation::UpdateMemberRoleRequest>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    conversation::update_member_role(State(pool), Extension(user_id), Path((id, member_id)), Json(req)).await
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

/// 收藏消息（包装认证中间件）
async fn add_bookmark_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(message_id): Path<String>,
    Json(req): Json<im_api::models::message::AddBookmarkRequest>,
) -> impl IntoResponse {
    message::add_bookmark_handler(State(pool), auth, Path(message_id), Json(req)).await
}

/// 取消收藏消息（包装认证中间件）
async fn remove_bookmark_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(message_id): Path<String>,
) -> impl IntoResponse {
    message::remove_bookmark_handler(State(pool), auth, Path(message_id)).await
}

/// 获取收藏列表（包装认证中间件）
async fn get_bookmarks_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Query(query): Query<im_api::models::message::BookmarkQuery>,
) -> impl IntoResponse {
    message::get_bookmarks_handler(State(pool), auth, Query(query)).await
}

/// 检查消息收藏状态（包装认证中间件）
async fn check_bookmark_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(message_id): Path<String>,
) -> impl IntoResponse {
    message::check_bookmark_handler(State(pool), auth, Path(message_id)).await
}

// ==================== 草稿消息 ====================

async fn save_draft_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
    Json(req): Json<im_api::models::message::SaveDraftRequest>,
) -> impl IntoResponse {
    message::save_draft_handler(State(pool), auth, Path(conversation_id), Json(req)).await
}

async fn get_draft_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
) -> impl IntoResponse {
    message::get_draft_handler(State(pool), auth, Path(conversation_id)).await
}

async fn delete_draft_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
) -> impl IntoResponse {
    message::delete_draft_handler(State(pool), auth, Path(conversation_id)).await
}

async fn get_all_drafts_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Query(query): Query<im_api::models::message::DraftQuery>,
) -> impl IntoResponse {
    message::get_all_drafts_handler(State(pool), auth, Query(query)).await
}

/// 创建定时消息
async fn create_scheduled_message_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<im_api::models::message::CreateScheduledMessageRequest>,
) -> impl IntoResponse {
    message::create_scheduled_message_handler(State(pool), auth, Json(req)).await
}

/// 获取定时消息详情
async fn get_scheduled_message_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(message_id): Path<String>,
) -> impl IntoResponse {
    message::get_scheduled_message_handler(State(pool), auth, Path(message_id)).await
}

/// 更新定时消息
async fn update_scheduled_message_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(message_id): Path<String>,
    Json(req): Json<im_api::models::message::UpdateScheduledMessageRequest>,
) -> impl IntoResponse {
    message::update_scheduled_message_handler(State(pool), auth, Path(message_id), Json(req)).await
}

/// 取消定时消息
async fn cancel_scheduled_message_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(message_id): Path<String>,
) -> impl IntoResponse {
    message::cancel_scheduled_message_handler(State(pool), auth, Path(message_id)).await
}

/// 获取定时消息列表
async fn get_scheduled_messages_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Query(query): Query<im_api::models::message::ScheduledMessageQuery>,
) -> impl IntoResponse {
    message::get_scheduled_messages_handler(State(pool), auth, Query(query)).await
}


// ===== 快捷回复包装函数 =====
async fn create_quick_reply_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(request): Json<im_api::models::quick_reply::CreateQuickReplyRequest>,
) -> impl IntoResponse {
    quick_reply::create_quick_reply_handler(State(pool), auth, Json(request)).await
}

async fn get_quick_replies_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Query(params): Query<quick_reply::QuickReplyQuery>,
) -> impl IntoResponse {
    quick_reply::get_quick_replies_handler(State(pool), auth, Query(params)).await
}

async fn get_quick_reply_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> impl IntoResponse {
    quick_reply::get_quick_reply_handler(State(pool), auth, Path(id)).await
}

async fn update_quick_reply_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(request): Json<im_api::models::quick_reply::UpdateQuickReplyRequest>,
) -> impl IntoResponse {
    quick_reply::update_quick_reply_handler(State(pool), auth, Path(id), Json(request)).await
}

async fn delete_quick_reply_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> impl IntoResponse {
    quick_reply::delete_quick_reply_handler(State(pool), auth, Path(id)).await
}

async fn create_global_quick_reply_with_auth(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(request): Json<im_api::models::quick_reply::CreateQuickReplyRequest>,
) -> impl IntoResponse {
    quick_reply::create_global_quick_reply_handler(State(pool), auth, Json(request)).await
}
/// 初始化数据库表
async fn init_database(pool: &PgPool) -> anyhow::Result<()> {
    info!("Initializing database tables...");

    // 启用 pg_trgm 扩展（用于文本搜索相似度计算）
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_trgm")
        .execute(pool)
        .await?;
    info!("pg_trgm extension enabled");

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

    // 创建消息收藏表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS message_bookmarks (
            id UUID PRIMARY KEY,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            message_id UUID NOT NULL,
            conversation_id UUID NOT NULL,
            note VARCHAR(500),
            created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
            UNIQUE(user_id, message_id)
        );

        CREATE INDEX IF NOT EXISTS idx_message_bookmarks_user_id ON message_bookmarks(user_id);
        CREATE INDEX IF NOT EXISTS idx_message_bookmarks_message_id ON message_bookmarks(message_id);
        CREATE INDEX IF NOT EXISTS idx_message_bookmarks_created_at ON message_bookmarks(created_at DESC);
        "#
    )
    .execute(pool)
    .await?;

    // 创建草稿消息表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS draft_messages (
            id UUID PRIMARY KEY,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            conversation_id UUID NOT NULL,
            content TEXT NOT NULL,
            type_ VARCHAR(20) NOT NULL DEFAULT 'text',
            reply_to UUID,
            metadata JSONB,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL,
            updated_at TIMESTAMP WITH TIME ZONE NOT NULL
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_draft_messages_user_conversation
            ON draft_messages(user_id, conversation_id);
        CREATE INDEX IF NOT EXISTS idx_draft_messages_user_id ON draft_messages(user_id);
        CREATE INDEX IF NOT EXISTS idx_draft_messages_updated_at ON draft_messages(updated_at DESC);
        "#
    )
    .execute(pool)
    .await?;

    // 创建定时消息表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS scheduled_messages (
            id UUID PRIMARY KEY,
            sender_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            conversation_id UUID NOT NULL,
            content TEXT NOT NULL,
            message_type VARCHAR(20) NOT NULL DEFAULT 'text',
            reply_to UUID,
            metadata JSONB,
            scheduled_at TIMESTAMP WITH TIME ZONE NOT NULL,
            status VARCHAR(20) NOT NULL DEFAULT 'pending',
            sent_at TIMESTAMP WITH TIME ZONE,
            error_message TEXT,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL,
            updated_at TIMESTAMP WITH TIME ZONE NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_scheduled_messages_sender_id ON scheduled_messages(sender_id);
        CREATE INDEX IF NOT EXISTS idx_scheduled_messages_status ON scheduled_messages(status);
        CREATE INDEX IF NOT EXISTS idx_scheduled_messages_scheduled_at ON scheduled_messages(scheduled_at);
        CREATE INDEX IF NOT EXISTS idx_scheduled_messages_pending ON scheduled_messages(scheduled_at) WHERE status = 'pending';
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

    // 创建会话置顶消息表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS pinned_messages (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
            message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
            pinned_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            pinned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(conversation_id, message_id)
        );

        CREATE INDEX IF NOT EXISTS idx_pinned_messages_conversation 
            ON pinned_messages(conversation_id, pinned_at DESC);
        CREATE INDEX IF NOT EXISTS idx_pinned_messages_message 
            ON pinned_messages(message_id);
        CREATE INDEX IF NOT EXISTS idx_pinned_messages_pinned_by 
            ON pinned_messages(pinned_by);
        "#
    )
    .execute(pool)
    .await?;

    info!("Pinned messages table initialized successfully");

    // 创建会话通知偏好表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS conversation_notification_preferences (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
            muted BOOLEAN NOT NULL DEFAULT false,
            sound VARCHAR(50) NOT NULL DEFAULT 'default',
            badge BOOLEAN NOT NULL DEFAULT true,
            mention_only BOOLEAN NOT NULL DEFAULT false,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(user_id, conversation_id)
        );

        CREATE INDEX IF NOT EXISTS idx_conv_notif_pref_user ON conversation_notification_preferences(user_id);
        CREATE INDEX IF NOT EXISTS idx_conv_notif_pref_conv ON conversation_notification_preferences(conversation_id);
        "#
    )
    .execute(pool)
    .await?;

    // 创建全局通知设置表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS global_notification_settings (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE UNIQUE,
            enabled BOOLEAN NOT NULL DEFAULT true,
            sound VARCHAR(50) NOT NULL DEFAULT 'default',
            badge BOOLEAN NOT NULL DEFAULT true,
            preview BOOLEAN NOT NULL DEFAULT true,
            dnd_start VARCHAR(5),
            dnd_end VARCHAR(5),
            dnd_timezone VARCHAR(50) DEFAULT 'UTC',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE INDEX IF NOT EXISTS idx_global_notif_user ON global_notification_settings(user_id);
        "#
    )
    .execute(pool)
    .await?;

    info!("Notification preferences tables initialized successfully");


    // 创建快捷回复模板表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS quick_replies (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            title VARCHAR(100) NOT NULL,
            content TEXT NOT NULL,
            category VARCHAR(50),
            sort_order INT NOT NULL DEFAULT 0,
            is_global BOOLEAN NOT NULL DEFAULT false,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE INDEX IF NOT EXISTS idx_quick_replies_user ON quick_replies(user_id);
        CREATE INDEX IF NOT EXISTS idx_quick_replies_global ON quick_replies(is_global) WHERE is_global = true;
        "#
    )
    .execute(pool)
    .await?;

    info!("Quick replies table initialized successfully");

    // 创建会话用户状态表（未读计数优化）
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS conversation_user_state (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
            user_id UUID NOT NULL,
            last_read_at TIMESTAMPTZ DEFAULT NOW(),
            unread_count INTEGER NOT NULL DEFAULT 0,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(conversation_id, user_id)
        );

        -- 单字段索引
        CREATE INDEX IF NOT EXISTS idx_conversation_user_state_user ON conversation_user_state(user_id);
        CREATE INDEX IF NOT EXISTS idx_conversation_user_state_conv ON conversation_user_state(conversation_id);

        -- 复合索引：优化批量未读计数查询 (user_id, conversation_id)
        CREATE INDEX IF NOT EXISTS idx_conversation_user_state_user_conv
            ON conversation_user_state(user_id, conversation_id);

        -- 复合索引：优化未读消息会话列表查询 (user_id, unread_count)
        CREATE INDEX IF NOT EXISTS idx_conversation_user_state_user_unread
            ON conversation_user_state(user_id, unread_count) WHERE unread_count > 0;
        "#
    )
    .execute(pool)
    .await?;

    info!("Conversation user state table initialized successfully");
    Ok(())
}
