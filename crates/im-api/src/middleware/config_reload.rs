//! 配置热更新模块
//!
//! 监听配置文件变更，自动重新加载配置并通知订阅者。
//! 使用 `notify` crate 实现文件系统监听，`tokio::sync::broadcast` 实现变更通知。
//!
//! # 使用方法
//!
//! ```rust,no_run
//! use im_api::middleware::config_reload::{ConfigWatcher, AppConfig};
//!
//! // 创建配置监听器
//! let watcher = ConfigWatcher::new("config.toml").await?;
//!
//! // 获取当前配置
//! let config = watcher.get_config();
//!
//! // 订阅配置变更
//! let mut rx = watcher.subscribe();
//! tokio::spawn(async move {
//!     while let Ok(new_config) = rx.recv().await {
//!         println!("配置已更新: {:?}", new_config);
//!     }
//! });
//! ```

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{error, info, warn};

/// 应用配置
///
/// 包含所有可热更新的配置项
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppConfig {
    /// 日志级别
    pub log_level: String,
    /// 速率限制：最大请求数
    pub rate_limit_max_requests: u32,
    /// 速率限制：时间窗口（秒）
    pub rate_limit_window_secs: u64,
    /// 是否启用压缩
    pub compression_enabled: bool,
    /// 最小压缩大小（字节）
    pub compression_min_size: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            rate_limit_max_requests: 100,
            rate_limit_window_secs: 60,
            compression_enabled: true,
            compression_min_size: 1024,
        }
    }
}

/// 配置监听器
///
/// 监听配置文件变更，自动重新加载并通知订阅者
pub struct ConfigWatcher {
    /// 当前配置
    config: Arc<RwLock<AppConfig>>,
    /// 配置变更广播发送器
    sender: broadcast::Sender<AppConfig>,
    /// 配置文件路径
    config_path: PathBuf,
    /// 监听器句柄（用于停止监听）
    _watcher_handle: Option<tokio::task::JoinHandle<()>>,
}

impl ConfigWatcher {
    /// 创建新的配置监听器
    ///
    /// # Arguments
    /// * `config_path` - 配置文件路径
    ///
    /// # Returns
    /// 返回配置监听器实例
    pub async fn new(config_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let config_path = config_path.as_ref().to_path_buf();

        // 尝试加载初始配置，如果文件不存在则使用默认配置
        let initial_config = if config_path.exists() {
            Self::load_config(&config_path).unwrap_or_else(|e| {
                warn!("加载配置文件失败，使用默认配置: {}", e);
                AppConfig::default()
            })
        } else {
            info!("配置文件不存在，使用默认配置: {:?}", config_path);
            AppConfig::default()
        };

        let config = Arc::new(RwLock::new(initial_config.clone()));
        let (sender, _) = broadcast::channel(16);

        // 启动文件监听任务
        let watch_config = config.clone();
        let watch_sender = sender.clone();
        let watch_path = config_path.clone();

        let watcher_handle = tokio::spawn(async move {
            Self::watch_config_file(watch_path, watch_config, watch_sender).await;
        });

        Ok(Self {
            config,
            sender,
            config_path,
            _watcher_handle: Some(watcher_handle),
        })
    }

    /// 获取当前配置
    pub fn get_config(&self) -> AppConfig {
        // 使用 try_read 避免阻塞，如果失败则返回默认配置
        self.config
            .try_read()
            .map(|c| c.clone())
            .unwrap_or_default()
    }

    /// 订阅配置变更
    ///
    /// 返回一个接收器，当配置变更时会收到新的配置
    pub fn subscribe(&self) -> broadcast::Receiver<AppConfig> {
        self.sender.subscribe()
    }

    /// 手动重新加载配置
    ///
    /// 从文件重新加载配置并通知所有订阅者
    pub async fn reload(&self) -> anyhow::Result<()> {
        let new_config = Self::load_config(&self.config_path)?;
        let mut config = self.config.write().await;
        *config = new_config.clone();

        // 通知订阅者（忽略没有接收者的错误）
        let _ = self.sender.send(new_config);

        info!("配置已手动重新加载");
        Ok(())
    }

    /// 加载配置文件
    fn load_config(path: &Path) -> anyhow::Result<AppConfig> {
        let content = std::fs::read_to_string(path)?;

        // 根据文件扩展名选择解析方式
        let config = match path.extension().and_then(|e| e.to_str()) {
            Some("toml") => toml::from_str(&content)?,
            Some("json") => serde_json::from_str(&content)?,
            Some("yaml") | Some("yml") => serde_yaml::from_str(&content)?,
            _ => {
                warn!("不支持的配置文件格式，尝试 TOML 解析");
                toml::from_str(&content)?
            }
        };

        Ok(config)
    }

    /// 监听配置文件变更
    async fn watch_config_file(
        path: PathBuf,
        config: Arc<RwLock<AppConfig>>,
        sender: broadcast::Sender<AppConfig>,
    ) {
        use notify::RecursiveMode;
        use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
        use std::time::Duration;

        // 创建文件监听器（带防抖，避免频繁触发）
        let (tx, rx) = std::sync::mpsc::channel();

        let mut watcher = match new_debouncer(Duration::from_secs(2), tx) {
            Ok(w) => w,
            Err(e) => {
                error!("创建文件监听器失败: {}", e);
                return;
            }
        };

        // 监听配置文件所在目录（监听文件变更需要监听目录）
        if let Some(parent) = path.parent() {
            if let Err(e) = watcher.watcher().watch(parent, RecursiveMode::NonRecursive) {
                error!("监听配置目录失败: {}", e);
                return;
            }
        }

        info!("开始监听配置文件变更: {:?}", path);

        // 持续监听文件变更事件
        loop {
            match rx.recv() {
                Ok(Ok(events)) => {
                    for event in events {
                        // 只处理我们关心的配置文件
                        if event.path == path {
                            match event.kind {
                                DebouncedEventKind::Any => {
                                    info!("检测到配置文件变更，重新加载...");

                                    match Self::load_config(&path) {
                                        Ok(new_config) => {
                                            let mut cfg = config.write().await;
                                            *cfg = new_config.clone();

                                            // 通知订阅者
                                            if sender.send(new_config).is_err() {
                                                warn!("没有配置变更订阅者");
                                            }

                                            info!("配置重新加载成功");
                                        }
                                        Err(e) => {
                                            error!("重新加载配置失败: {}", e);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Ok(Err(e)) => {
                    error!("文件监听错误: {}", e);
                }
                Err(e) => {
                    error!("文件监听通道错误: {}", e);
                    break;
                }
            }
        }
    }
}

impl Drop for ConfigWatcher {
    fn drop(&mut self) {
        info!("配置监听器已停止");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.log_level, "info");
        assert_eq!(config.rate_limit_max_requests, 100);
        assert_eq!(config.rate_limit_window_secs, 60);
        assert!(config.compression_enabled);
        assert_eq!(config.compression_min_size, 1024);
    }

    #[test]
    fn test_config_serialization() {
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.log_level, config.log_level);
        assert_eq!(
            deserialized.rate_limit_max_requests,
            config.rate_limit_max_requests
        );
    }

    #[tokio::test]
    async fn test_config_watcher_creation() {
        // 创建临时配置文件
        let dir = std::env::temp_dir();
        let config_path = dir.join("test_config.json");
        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(b"{\"log_level\": \"debug\", \"rate_limit_max_requests\": 200, \"rate_limit_window_secs\": 30, \"compression_enabled\": false, \"compression_min_size\": 512}")
            .unwrap();

        let watcher = ConfigWatcher::new(&config_path).await.unwrap();
        let config = watcher.get_config();
        assert_eq!(config.log_level, "debug");
        assert_eq!(config.rate_limit_max_requests, 200);

        // 清理
        std::fs::remove_file(config_path).ok();
    }

    #[tokio::test]
    async fn test_config_subscribe() {
        let dir = std::env::temp_dir();
        let config_path = dir.join("test_config_subscribe.json");
        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(b"{\"log_level\": \"info\", \"rate_limit_max_requests\": 100, \"rate_limit_window_secs\": 60, \"compression_enabled\": true, \"compression_min_size\": 1024}")
            .unwrap();

        let watcher = ConfigWatcher::new(&config_path).await.unwrap();
        let mut rx = watcher.subscribe();

        // 修改配置文件
        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(b"{\"log_level\": \"debug\", \"rate_limit_max_requests\": 200, \"rate_limit_window_secs\": 30, \"compression_enabled\": false, \"compression_min_size\": 512}")
            .unwrap();

        // 手动触发重新加载
        watcher.reload().await.unwrap();

        // 检查是否收到通知
        let new_config = rx.recv().await.unwrap();
        assert_eq!(new_config.log_level, "debug");
        assert_eq!(new_config.rate_limit_max_requests, 200);

        // 清理
        std::fs::remove_file(config_path).ok();
    }
}
