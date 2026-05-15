//! 数据模型模块
//!
//! 定义 API 请求/响应的数据结构：
//! - `auth`: 用户认证相关模型（User, RegisterRequest, LoginRequest 等）
//! - `message`: 消息相关模型（Message, MessageType, MessageStatus 等）
//! - `conversation`: 会话相关模型（Conversation, ConversationType, Tag 等）

pub mod auth;
pub mod message;
pub mod conversation;
pub mod announcement;
pub mod quick_reply;
pub mod feedback;
pub mod chat_export;
pub mod user_preferences;
pub mod webhook;
pub mod data_retention;
pub mod admin;
pub mod user_activity;
pub mod api_key;

pub use auth::{ApiResponse, ApiError, ErrorCode};
