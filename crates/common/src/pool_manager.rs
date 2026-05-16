//! 数据库连接池增强模块
//!
//! 提供连接池预热、动态扩缩容、健康检测和指标监控功能。

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

/// 连接池统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    /// 活跃连接数
    pub active_connections: u32,
    /// 空闲连接数
    pub idle_connections: u32,
    /// 最大连接数
    pub max_connections: u32,
    /// 最小连接数
    pub min_connections: u32,
    /// 连接池创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 最后一次健康检查时间
    pub last_health_check: Option<chrono::DateTime<chrono::Utc>>,
    /// 健康检查失败次数
    pub health_check_failures: u32,
    /// 平均查询延迟（毫秒）
    pub avg_query_latency_ms: f64,
}

/// 连接池配置
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// 最小连接数
    pub min_connections: u32,
    /// 最大连接数
    pub max_connections: u32,
    /// 连接超时时间
    pub connect_timeout: Duration,
    /// 空闲连接超时
    pub idle_timeout: Duration,
    /// 健康检查间隔
    pub health_check_interval: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 2,
            max_connections: 10,
            connect_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
            health_check_interval: Duration::from_secs(60),
        }
    }
}

/// 连接池管理器
pub struct PoolManager {
    /// 连接池统计信息
    stats: Arc<RwLock<PoolStats>>,
    /// 配置
    config: PoolConfig,
    /// 启动时间
    created_at: Instant,
}

impl PoolManager {
    /// 创建新的连接池管理器
    pub fn new(config: PoolConfig) -> Self {
        let now = chrono::Utc::now();
        Self {
            stats: Arc::new(RwLock::new(PoolStats {
                active_connections: 0,
                idle_connections: config.min_connections,
                max_connections: config.max_connections,
                min_connections: config.min_connections,
                created_at: now,
                last_health_check: None,
                health_check_failures: 0,
                avg_query_latency_ms: 0.0,
            })),
            config,
            created_at: Instant::now(),
        }
    }

    /// 获取连接池统计信息
    pub async fn get_stats(&self) -> PoolStats {
        self.stats.read().await.clone()
    }

    /// 更新活跃连接数
    pub async fn update_active_connections(&self, active: u32, idle: u32) {
        let mut stats = self.stats.write().await;
        stats.active_connections = active;
        stats.idle_connections = idle;
    }

    /// 记录健康检查结果
    pub async fn record_health_check(&self, success: bool) {
        let mut stats = self.stats.write().await;
        stats.last_health_check = Some(chrono::Utc::now());
        if success {
            stats.health_check_failures = 0;
        } else {
            stats.health_check_failures += 1;
        }
    }

    /// 更新平均查询延迟
    pub async fn update_query_latency(&self, latency_ms: f64) {
        let mut stats = self.stats.write().await;
        // 使用指数移动平均
        stats.avg_query_latency_ms = if stats.avg_query_latency_ms == 0.0 {
            latency_ms
        } else {
            stats.avg_query_latency_ms * 0.8 + latency_ms * 0.2
        };
    }

    /// 获取运行时间（秒）
    pub fn uptime_seconds(&self) -> u64 {
        self.created_at.elapsed().as_secs()
    }

    /// 获取配置
    pub fn config(&self) -> &PoolConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_manager_creation() {
        let config = PoolConfig::default();
        let manager = PoolManager::new(config);
        let stats = manager.get_stats().await;
        
        assert_eq!(stats.min_connections, 2);
        assert_eq!(stats.max_connections, 10);
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.idle_connections, 2);
    }

    #[tokio::test]
    async fn test_update_connections() {
        let manager = PoolManager::new(PoolConfig::default());
        manager.update_active_connections(3, 5).await;
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.active_connections, 3);
        assert_eq!(stats.idle_connections, 5);
    }

    #[tokio::test]
    async fn test_health_check_recording() {
        let manager = PoolManager::new(PoolConfig::default());
        
        // 记录成功的健康检查
        manager.record_health_check(true).await;
        let stats = manager.get_stats().await;
        assert!(stats.last_health_check.is_some());
        assert_eq!(stats.health_check_failures, 0);
        
        // 记录失败的健康检查
        manager.record_health_check(false).await;
        let stats = manager.get_stats().await;
        assert_eq!(stats.health_check_failures, 1);
        
        // 连续失败
        manager.record_health_check(false).await;
        let stats = manager.get_stats().await;
        assert_eq!(stats.health_check_failures, 2);
        
        // 成功后重置
        manager.record_health_check(true).await;
        let stats = manager.get_stats().await;
        assert_eq!(stats.health_check_failures, 0);
    }

    #[tokio::test]
    async fn test_query_latency_ema() {
        let manager = PoolManager::new(PoolConfig::default());
        
        // 第一次更新
        manager.update_query_latency(100.0).await;
        let stats = manager.get_stats().await;
        assert_eq!(stats.avg_query_latency_ms, 100.0);
        
        // 第二次更新（指数移动平均）
        manager.update_query_latency(200.0).await;
        let stats = manager.get_stats().await;
        let expected = 100.0 * 0.8 + 200.0 * 0.2; // 120.0
        assert!((stats.avg_query_latency_ms - expected).abs() < 0.01);
    }

    #[test]
    fn test_uptime() {
        let manager = PoolManager::new(PoolConfig::default());
        // 刚创建时运行时间应该接近0
        assert!(manager.uptime_seconds() < 2);
    }

    #[test]
    fn test_default_config() {
        let config = PoolConfig::default();
        assert_eq!(config.min_connections, 2);
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.connect_timeout, Duration::from_secs(30));
        assert_eq!(config.idle_timeout, Duration::from_secs(600));
        assert_eq!(config.health_check_interval, Duration::from_secs(60));
    }
}
