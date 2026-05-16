//! OmniLink 公共库
//!
//! 提供跨服务共享的基础设施：
//! - `error`: 统一错误类型和处理
//! - `models`: 共享数据模型
//! - `utils`: 工具函数（验证、格式化等）
//! - `auth`: JWT 认证和密码管理
//! - `db`: 数据库连接管理
//! - `crypto`: 加密工具（ECDH 密钥交换、AES 加解密）
//! - `pool_monitor`: 连接池监控和健康检查
//! - `log_level`: 动态日志级别调整
//! - `system_metrics`: 系统指标收集（CPU、内存、磁盘）

pub mod error;
pub mod models;
pub mod utils;
pub mod auth;
pub mod db;
pub mod cache;
pub mod crypto;
pub mod security;
pub mod secrets;
pub mod pool_monitor;
pub mod audit;
pub mod config;
pub mod log_level;
pub mod validation;
pub mod tracing_setup;
pub mod circuit_breaker;
pub mod graceful_shutdown;
pub mod api_version;
pub mod structured_logging;

// V2.8: 新增模块
pub mod system_metrics;
pub mod health_aggregator;
pub mod message_delivery;
pub mod key_rotation;
pub mod pool_manager;
pub mod request_validation;

pub use error::{AppError, Result};
pub use auth::{Claims, TokenManager, PasswordManager, CryptoManager};
pub use db::{DatabaseManager};
pub use models::ApiResponse;
pub use pool_monitor::{PoolMonitor, PoolStats, HealthCheckResult, HealthStatus};
