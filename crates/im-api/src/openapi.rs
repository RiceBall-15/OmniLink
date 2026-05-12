use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
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
    )),
    tags(
        (name = "auth", description = "用户认证 - 注册、登录、Token管理"),
        (name = "messages", description = "消息管理 - 发送、编辑、撤回、搜索消息"),
        (name = "conversations", description = "会话管理 - 创建、查询、管理会话"),
        (name = "encryption", description = "端到端加密 - 密钥管理、消息加解密"),
    )
)]
pub struct ApiDoc;
