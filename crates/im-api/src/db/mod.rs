//! 数据库访问层模块
//!
//! 封装所有数据库操作：
//! - `user`: 用户表 CRUD 操作
//! - `message`: 消息表 CRUD 操作（含搜索和统计）
//! - `conversation`: 会话表 CRUD 操作（含参与者、标签管理）

pub mod user;
pub mod message;
pub mod conversation;
