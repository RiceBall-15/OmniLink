use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        // 认证
        crate::handlers::auth::register,
        crate::handlers::auth::login,
        crate::handlers::auth::get_me,
        // 消息
        crate::handlers::message::get_messages,
        crate::handlers::message::send_message,
        crate::handlers::message::edit_message,
        crate::handlers::message::recall_message_handler,
        crate::handlers::message::batch_send_messages,
        crate::handlers::message::batch_delete_messages,
        crate::handlers::message::batch_mark_as_read,
        // 会话
        crate::handlers::conversation::get_conversations,
        crate::handlers::conversation::create_conversation_handler,
        crate::handlers::conversation::search,
        crate::handlers::conversation::toggle_pin,
        crate::handlers::conversation::toggle_mute,
        crate::handlers::conversation::toggle_archive,
       // 加密
       crate::handlers::encryption::generate_keys,
       crate::handlers::encryption::encrypt_message,
       crate::handlers::encryption::decrypt_message,
       crate::handlers::encryption::key_exchange,
       crate::handlers::encryption::get_encrypted_messages,
        crate::handlers::encryption::register_public_key,
        crate::handlers::encryption::get_user_public_key,
        crate::handlers::encryption::batch_get_public_keys,
        // 健康检查
        crate::handlers::health::health_check_with_deps,
        // 联系人
        crate::handlers::contact::add_contact_handler,
        crate::handlers::contact::get_contacts_handler,
        crate::handlers::contact::search_users_handler,
        // 公告
        crate::handlers::announcement::create_announcement_handler,
        crate::handlers::announcement::get_all_announcements_handler,
        crate::handlers::announcement::get_active_announcements_handler,
        crate::handlers::announcement::mark_announcement_read_handler,
        // 快捷回复
        crate::handlers::quick_reply::create_quick_reply_handler,
        crate::handlers::quick_reply::get_quick_replies_handler,
        // 反馈
        crate::handlers::feedback::submit_feedback_handler,
        crate::handlers::feedback::get_all_feedbacks_handler,
        // 聊天导出
        crate::handlers::chat_export::create_export_job_handler,
        crate::handlers::chat_export::download_export_file_handler,
        // 消息重试
        crate::handlers::message_retry::retry_message_handler,
        crate::handlers::message_retry::get_failed_messages_handler,
    ),
    info(
        title = "OmniLink IM API",
        version = "0.1.0",
        description = "OmniLink 即时通讯系统 REST API\n\n支持用户认证、即时消息、会话管理、端到端加密等功能。",
        contact(name = "OmniLink Team", email = "dev@omnilink.com"),
        license(name = "MIT")
    ),
    components(schemas(
        // Auth models
        crate::models::auth::RegisterRequest,
        crate::models::auth::LoginRequest,
        crate::models::auth::LoginResponse,
        crate::models::auth::User,
        // Message models
        crate::models::message::SendMessageRequest,
        crate::models::message::EditMessageRequest,
        crate::models::message::Message,
        crate::models::message::MessageType,
        crate::models::message::MessageStatus,
        crate::models::message::OnlineStatus,
        // Conversation models
        crate::models::conversation::Conversation,
        crate::models::conversation::ConversationType,
        crate::models::conversation::CreateConversationRequest,
        // Health models
        crate::handlers::health::HealthCheckResponse,
        crate::handlers::health::Dependencies,
        crate::handlers::health::DependencyStatus,
    )),
    tags(
        (name = "auth", description = "用户认证 - 注册、登录、Token管理"),
        (name = "messages", description = "消息管理 - 发送、编辑、撤回、搜索消息"),
        (name = "conversations", description = "会话管理 - 创建、查询、管理会话"),
        (name = "encryption", description = "端到端加密 - 密钥管理、消息加解密"),
        (name = "health", description = "健康检查 - 服务状态、依赖检查"),
        (name = "contacts", description = "联系人管理 - 添加、查询、搜索联系人"),
        (name = "announcements", description = "系统公告 - 创建、查询、标记已读"),
        (name = "quick-replies", description = "快捷回复 - 创建、查询快捷回复"),
        (name = "feedbacks", description = "反馈管理 - 提交、查询反馈"),
        (name = "chat-export", description = "聊天导出 - 创建导出任务、下载文件"),
        (name = "message-retry", description = "消息重试 - 重试失败消息、查询失败列表"),
    )
)]
pub struct ApiDoc;
