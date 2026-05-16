//! IM Gateway - WebSocket 网关库
//!
//! 提供 WebSocket 连接管理、消息路由和在线状态同步功能。
//!
//! ## 模块结构
//! - `models`: 数据模型定义
//! - `handlers`: HTTP 和 WebSocket 请求处理器
//! - `services`: 业务逻辑服务
//! - `conversation_service`: 会话服务
//! - `repository`: 数据仓库层
//! - `user_repository`: 用户数据仓库
//! - `middleware`: 认证中间件
//! - `connection_manager`: WebSocket 连接管理器
//! - `status_manager`: 用户在线状态管理器

pub mod models;
pub mod handlers;
pub mod services;
pub mod conversation_service;
pub mod repository;
pub mod user_repository;
pub mod middleware;
pub mod connection_manager;
pub mod status_manager;
pub mod offline_queue;
pub mod connection_quality;
pub mod block_manager;
pub mod presence_channel;
pub mod message_batcher;
pub mod notification_pipeline;
pub mod metrics;
