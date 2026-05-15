//! API 请求处理器模块
//!
//! 包含所有 HTTP 端点的处理逻辑：
//! - `auth`: 用户认证（注册、登录、用户信息管理）
//! - `message`: 消息管理（发送、编辑、撤回、搜索）
//! - `conversation`: 会话管理（创建、列表、搜索、标签）
//! - `encryption`: 端到端加密（密钥管理、消息加解密）
//! - `health`: 健康检查（服务状态、依赖检查）

pub mod auth;
pub mod message;
pub mod conversation;
pub mod encryption;
pub mod health;
pub mod metrics;
pub mod audit;
pub mod scheduled_task;
pub mod contact;
pub mod message_retry;
pub mod announcement;
pub mod quick_reply;
pub mod feedback;
pub mod chat_export;
pub mod export_worker;
pub mod user_preferences;
pub mod webhook;
pub mod data_retention;
pub mod admin;
pub mod user_activity;
pub mod api_key;
