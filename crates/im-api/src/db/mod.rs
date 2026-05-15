//! 数据库访问层模块
//!
//! 封装所有数据库操作：
//! - `user`: 用户表 CRUD 操作
//! - `message`: 消息表 CRUD 操作（含搜索和统计）
//! - `conversation`: 会话表 CRUD 操作（含参与者、标签管理）

pub mod user;
pub mod message;
pub mod conversation;
pub mod contact;
pub mod message_retry;
pub mod announcement;
pub mod quick_reply;
pub mod feedback;
pub mod chat_export;
pub mod user_preferences;
pub mod webhook;
pub mod data_retention;
pub mod admin;
pub mod user_activity;
