//! 优雅停机处理
//!
//! 提供微服务的优雅停机支持：
//! - 停机阶段管理（Running → Draining → Callbacks → Shutdown）
//! - 连接计数跟踪
//! - 停机回调注册与执行
//! - 停机信号广播
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use common::graceful_shutdown::{GracefulShutdown, ShutdownPhase};
//! use std::time::Duration;
//!
//! # async fn example() {
//! // 创建优雅停机管理器
//! let shutdown = GracefulShutdown::builder()
//!     .shutdown_timeout(Duration::from_secs(30))
//!     .drain_timeout(Duration::from_secs(10))
//!     .build();
//!
//! // 注册停机回调
//! shutdown.on_shutdown(|| async {
//!     tracing::info!("执行清理操作");
//! }).await;
//!
//! // 监听停机信号（在后台任务中）
//! shutdown.listen_for_signals().await;
//!
//! // 等待停机完成
//! shutdown.wait_for_shutdown().await;
//!
//! // 在连接处理器中使用
//! let _guard = shutdown.enter_connection();
//! // ... 处理请求 ...
//! // guard drop 时自动退出连接
//! # }
//! ```

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, RwLock};

/// 停机阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShutdownPhase {
    /// 正常运行
    Running,
    /// 排空中（停止接受新连接，等待活跃连接完成）
    Draining,
    /// 执行停机回调
    ExecutingCallbacks,
    /// 已停机
    Shutdown,
}

impl std::fmt::Display for ShutdownPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShutdownPhase::Running => write!(f, "Running"),
            ShutdownPhase::Draining => write!(f, "Draining"),
            ShutdownPhase::ExecutingCallbacks => write!(f, "ExecutingCallbacks"),
            ShutdownPhase::Shutdown => write!(f, "Shutdown"),
        }
    }
}

/// 停机统计信息
#[derive(Debug, Clone)]
pub struct ShutdownStats {
    /// 当前阶段
    pub phase: ShutdownPhase,
    /// 活跃连接数
    pub active_connections: usize,
    /// 总处理请求数
    pub total_requests: usize,
    /// 停机已运行时长（秒）
    pub uptime_secs: u64,
}

/// 连接守卫
///
/// 当守卫被 drop 时，自动减少活跃连接计数。
pub struct ConnectionGuard {
    inner: Arc<ShutdownInner>,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.inner.active_connections.fetch_sub(1, Ordering::SeqCst);
    }
}

type ShutdownCallback = Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

use std::future::Future;
use std::pin::Pin;

/// 优雅停机构建器
pub struct GracefulShutdownBuilder {
    shutdown_timeout: Duration,
    drain_timeout: Duration,
    callback_timeout: Duration,
}

impl GracefulShutdownBuilder {
    /// 设置停机超时（从开始停机到强制关闭的时间）
    pub fn shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    /// 设置排空超时（等待活跃连接完成的时间）
    pub fn drain_timeout(mut self, timeout: Duration) -> Self {
        self.drain_timeout = timeout;
        self
    }

    /// 设置回调执行超时
    pub fn callback_timeout(mut self, timeout: Duration) -> Self {
        self.callback_timeout = timeout;
        self
    }

    /// 构建 GracefulShutdown 实例
    pub fn build(self) -> GracefulShutdown {
        let (shutdown_tx, _) = broadcast::channel::<()>(1);

        GracefulShutdown {
            inner: Arc::new(ShutdownInner {
                phase: RwLock::new(ShutdownPhase::Running),
                active_connections: AtomicUsize::new(0),
                total_requests: AtomicUsize::new(0),
                shutdown_signal: AtomicBool::new(false),
                shutdown_tx,
                callbacks: RwLock::new(Vec::new()),
                shutdown_started: RwLock::new(None),
                drain_completed: RwLock::new(None),
            }),
            shutdown_timeout: self.shutdown_timeout,
            drain_timeout: self.drain_timeout,
            callback_timeout: self.callback_timeout,
        }
    }
}

/// 内部状态
struct ShutdownInner {
    /// 当前停机阶段
    phase: RwLock<ShutdownPhase>,
    /// 活跃连接计数
    active_connections: AtomicUsize,
    /// 总处理请求数
    total_requests: AtomicUsize,
    /// 是否已发出停机信号
    shutdown_signal: AtomicBool,
    /// 停机信号广播
    shutdown_tx: broadcast::Sender<()>,
    /// 停机回调列表
    callbacks: RwLock<Vec<ShutdownCallback>>,
    /// 停机开始时间
    shutdown_started: RwLock<Option<Instant>>,
    /// 排空完成时间
    drain_completed: RwLock<Option<Instant>>,
}

/// 优雅停机管理器
///
/// 线程安全，可在多个异步任务间共享。
#[derive(Clone)]
pub struct GracefulShutdown {
    inner: Arc<ShutdownInner>,
    shutdown_timeout: Duration,
    drain_timeout: Duration,
    callback_timeout: Duration,
}

impl GracefulShutdown {
    /// 创建构建器
    pub fn builder() -> GracefulShutdownBuilder {
        GracefulShutdownBuilder {
            shutdown_timeout: Duration::from_secs(30),
            drain_timeout: Duration::from_secs(10),
            callback_timeout: Duration::from_secs(5),
        }
    }

    /// 获取当前停机阶段
    pub async fn phase(&self) -> ShutdownPhase {
        *self.inner.phase.read().await
    }

    /// 注册停机回调
    pub async fn on_shutdown<F, Fut>(&self, callback: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut callbacks = self.inner.callbacks.write().await;
        callbacks.push(Box::new(move || Box::pin(callback())));
    }

    /// 进入连接（增加活跃连接计数）
    ///
    /// 返回 ConnectionGuard，drop 时自动减少计数。
    /// 如果正在停机，返回 None。
    pub fn enter_connection(&self) -> Option<ConnectionGuard> {
        if self.inner.shutdown_signal.load(Ordering::SeqCst) {
            return None;
        }

        self.inner.active_connections.fetch_add(1, Ordering::SeqCst);
        self.inner.total_requests.fetch_add(1, Ordering::SeqCst);

        Some(ConnectionGuard {
            inner: self.inner.clone(),
        })
    }

    /// 订阅停机信号
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.inner.shutdown_tx.subscribe()
    }

    /// 获取活跃连接数
    pub fn active_connections(&self) -> usize {
        self.inner.active_connections.load(Ordering::SeqCst)
    }

    /// 获取总处理请求数
    pub fn total_requests(&self) -> usize {
        self.inner.total_requests.load(Ordering::SeqCst)
    }

    /// 是否正在停机
    pub fn is_shutting_down(&self) -> bool {
        self.inner.shutdown_signal.load(Ordering::SeqCst)
    }

    /// 等待停机完成
    pub async fn wait_for_shutdown(&self) {
        let mut rx = self.inner.shutdown_tx.subscribe();
        let _ = rx.recv().await;
    }

    /// 获取统计信息
    pub async fn stats(&self) -> ShutdownStats {
        let phase = *self.inner.phase.read().await;
        let active = self.active_connections();
        let total = self.total_requests();

        let uptime_secs = {
            let started = self.inner.shutdown_started.read().await;
            match *started {
                Some(t) => t.elapsed().as_secs(),
                None => 0,
            }
        };

        ShutdownStats {
            phase,
            active_connections: active,
            total_requests: total,
            uptime_secs,
        }
    }

    /// 触发停机
    ///
    /// 执行以下步骤：
    /// 1. 设置停机信号，停止接受新连接
    /// 2. 等待活跃连接完成（排空）
    /// 3. 执行停机回调
    /// 4. 标记停机完成
    ///
    /// 整个过程受 `shutdown_timeout` 约束，超时后强制进入 Shutdown 阶段。
    pub async fn shutdown(&self) {
        // 防止重复触发
        if self.inner.shutdown_signal.swap(true, Ordering::SeqCst) {
            tracing::warn!("停机已触发，忽略重复请求");
            return;
        }

        *self.inner.shutdown_started.write().await = Some(Instant::now());

        tracing::info!("开始优雅停机（总超时: {:?}）", self.shutdown_timeout);

        // 使用 overall shutdown_timeout 包裹整个停机流程
        let result = tokio::time::timeout(self.shutdown_timeout, self.shutdown_inner()).await;

        if result.is_err() {
            tracing::warn!(
                "停机总超时（{:?}），强制完成",
                self.shutdown_timeout
            );
        }

        // 确保最终阶段为 Shutdown
        {
            let mut phase = self.inner.phase.write().await;
            *phase = ShutdownPhase::Shutdown;
        }

        tracing::info!("优雅停机完成");
    }

    /// 内部停机逻辑，受 shutdown_timeout 超时保护
    async fn shutdown_inner(&self) {
        // 阶段 1: 排空中
        {
            let mut phase = self.inner.phase.write().await;
            *phase = ShutdownPhase::Draining;
        }

        // 广播停机信号
        let _ = self.inner.shutdown_tx.send(());

        tracing::info!(
            "停机信号已广播，当前活跃连接: {}",
            self.active_connections()
        );

        // 等待活跃连接完成或超时
        let drain_start = Instant::now();
        while self.active_connections() > 0 {
            if drain_start.elapsed() >= self.drain_timeout {
                tracing::warn!(
                    "排空超时，剩余 {} 个活跃连接",
                    self.active_connections()
                );
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        *self.inner.drain_completed.write().await = Some(Instant::now());

        // 阶段 2: 执行停机回调
        {
            let mut phase = self.inner.phase.write().await;
            *phase = ShutdownPhase::ExecutingCallbacks;
        }

        let callbacks = self.inner.callbacks.read().await;
        for (i, callback) in callbacks.iter().enumerate() {
            tracing::debug!("执行停机回调 {}/{}", i + 1, callbacks.len());
            let callback_future = callback();

            match tokio::time::timeout(self.callback_timeout, callback_future).await {
                Ok(_) => {
                    tracing::debug!("回调 {} 执行完成", i + 1);
                }
                Err(_) => {
                    tracing::warn!("回调 {} 执行超时", i + 1);
                }
            }
        }
    }

    /// 监听系统停机信号（SIGTERM、SIGINT）
    ///
    /// 在后台任务中调用此方法，当收到系统信号时自动触发停机。
    #[cfg(unix)]
    pub async fn listen_for_signals(&self) {
        use tokio::signal::unix::{signal, SignalKind};

        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("无法监听 Ctrl+C 信号");
        };

        let mut sigterm = signal(SignalKind::terminate())
            .expect("无法监听 SIGTERM 信号");

        let sigterm_wait = async {
            sigterm.recv().await;
        };

        tokio::select! {
            _ = ctrl_c => {
                tracing::info!("收到 SIGINT (Ctrl+C) 信号");
            }
            _ = sigterm_wait => {
                tracing::info!("收到 SIGTERM 信号");
            }
        }

        self.shutdown().await;
    }

    /// 监听系统停机信号（非 Unix 系统）
    #[cfg(not(unix))]
    pub async fn listen_for_signals(&self) {
        tokio::signal::ctrl_c()
            .await
            .expect("无法监听 Ctrl+C 信号");

        tracing::info!("收到停机信号");
        self.shutdown().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_initial_phase_is_running() {
        let shutdown = GracefulShutdown::builder().build();
        assert_eq!(shutdown.phase().await, ShutdownPhase::Running);
    }

    #[tokio::test]
    async fn test_connection_tracking() {
        let shutdown = GracefulShutdown::builder().build();

        let _guard1 = shutdown.enter_connection().unwrap();
        let _guard2 = shutdown.enter_connection().unwrap();

        assert_eq!(shutdown.active_connections(), 2);
        assert_eq!(shutdown.total_requests(), 2);

        drop(_guard1);
        assert_eq!(shutdown.active_connections(), 1);
    }

    #[tokio::test]
    async fn test_shutdown_transitions_phases() {
        let shutdown = GracefulShutdown::builder()
            .drain_timeout(Duration::from_millis(100))
            .build();

        shutdown.shutdown().await;

        assert_eq!(shutdown.phase().await, ShutdownPhase::Shutdown);
        assert!(shutdown.is_shutting_down());
    }

    #[tokio::test]
    async fn test_shutdown_executes_callbacks() {
        let shutdown = GracefulShutdown::builder()
            .drain_timeout(Duration::from_millis(100))
            .build();

        let executed = Arc::new(AtomicBool::new(false));

        shutdown
            .on_shutdown({
                let executed = executed.clone();
                move || {
                    let executed = executed.clone();
                    async move {
                        executed.store(true, Ordering::SeqCst);
                    }
                }
            })
            .await;

        shutdown.shutdown().await;
        assert!(executed.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_shutdown_waits_for_connections() {
        let shutdown = GracefulShutdown::builder()
            .drain_timeout(Duration::from_secs(2))
            .build();

        let _guard = shutdown.enter_connection().unwrap();

        let shutdown_clone = shutdown.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            // guard 在这里被 drop
        });

        let start = Instant::now();
        shutdown.shutdown().await;

        // 应该等待 guard drop
        assert!(start.elapsed() >= Duration::from_millis(100));
        assert_eq!(shutdown.phase().await, ShutdownPhase::Shutdown);
    }

    #[tokio::test]
    async fn test_shutdown_timeout_on_active_connections() {
        let shutdown = GracefulShutdown::builder()
            .drain_timeout(Duration::from_millis(100))
            .build();

        // 创建一个长时间存在的连接
        let _guard = shutdown.enter_connection().unwrap();

        let start = Instant::now();
        shutdown.shutdown().await;

        // 应该超时而不是无限等待
        assert!(start.elapsed() < Duration::from_secs(5));
        assert_eq!(shutdown.phase().await, ShutdownPhase::Shutdown);
    }

    #[tokio::test]
    async fn test_wait_for_shutdown() {
        let shutdown = GracefulShutdown::builder()
            .drain_timeout(Duration::from_millis(100))
            .build();

        let shutdown_clone = shutdown.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            shutdown_clone.shutdown().await;
        });

        // 等待停机完成
        shutdown.wait_for_shutdown().await;
        assert_eq!(shutdown.phase().await, ShutdownPhase::Shutdown);
    }

    #[tokio::test]
    async fn test_subscribe_receives_signal() {
        let shutdown = GracefulShutdown::builder()
            .drain_timeout(Duration::from_millis(100))
            .build();

        let mut rx = shutdown.subscribe();

        let shutdown_clone = shutdown.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            shutdown_clone.shutdown().await;
        });

        // 应该收到停机信号
        let result = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stats() {
        let shutdown = GracefulShutdown::builder().build();

        let _guard = shutdown.enter_connection().unwrap();

        let stats = shutdown.stats().await;
        assert_eq!(stats.phase, ShutdownPhase::Running);
        assert_eq!(stats.active_connections, 1);
        assert_eq!(stats.total_requests, 1);
    }

    #[tokio::test]
    async fn test_shutdown_is_idempotent() {
        let shutdown = GracefulShutdown::builder()
            .drain_timeout(Duration::from_millis(100))
            .build();

        // 第一次停机
        shutdown.shutdown().await;
        assert_eq!(shutdown.phase().await, ShutdownPhase::Shutdown);

        // 第二次停机应该被忽略
        shutdown.shutdown().await;
        assert_eq!(shutdown.phase().await, ShutdownPhase::Shutdown);
    }

    #[tokio::test]
    async fn test_callback_timeout() {
        let shutdown = GracefulShutdown::builder()
            .drain_timeout(Duration::from_millis(100))
            .callback_timeout(Duration::from_millis(50))
            .build();

        // 注册一个超时的回调
        shutdown
            .on_shutdown(|| async {
                tokio::time::sleep(Duration::from_secs(10)).await;
            })
            .await;

        let start = Instant::now();
        shutdown.shutdown().await;

        // 应该在超时后继续
        assert!(start.elapsed() < Duration::from_secs(5));
        assert_eq!(shutdown.phase().await, ShutdownPhase::Shutdown);
    }

    #[tokio::test]
    async fn test_builder_defaults() {
        let shutdown = GracefulShutdown::builder().build();
        assert_eq!(shutdown.shutdown_timeout, Duration::from_secs(30));
        assert_eq!(shutdown.drain_timeout, Duration::from_secs(10));
        assert_eq!(shutdown.callback_timeout, Duration::from_secs(5));
    }

    #[tokio::test]
    async fn test_builder_custom_values() {
        let shutdown = GracefulShutdown::builder()
            .shutdown_timeout(Duration::from_secs(60))
            .drain_timeout(Duration::from_secs(20))
            .callback_timeout(Duration::from_secs(10))
            .build();

        assert_eq!(shutdown.shutdown_timeout, Duration::from_secs(60));
        assert_eq!(shutdown.drain_timeout, Duration::from_secs(20));
        assert_eq!(shutdown.callback_timeout, Duration::from_secs(10));
    }

    #[tokio::test]
    async fn test_shutdown_phase_display() {
        assert_eq!(ShutdownPhase::Running.to_string(), "Running");
        assert_eq!(ShutdownPhase::Draining.to_string(), "Draining");
        assert_eq!(
            ShutdownPhase::ExecutingCallbacks.to_string(),
            "ExecutingCallbacks"
        );
        assert_eq!(ShutdownPhase::Shutdown.to_string(), "Shutdown");
    }
}
