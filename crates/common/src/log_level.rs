use std::sync::OnceLock;
use tracing_subscriber::{
    fmt, layer::SubscriberExt, reload, util::SubscriberInitExt, EnvFilter,
};

/// 全局日志级别重载句柄
static LOG_RELOAD_HANDLE: OnceLock<reload::Handle<EnvFilter, tracing_subscriber::Registry>> =
    OnceLock::new();

/// 初始化可动态调整的日志系统
///
/// 与默认的 `tracing_subscriber::fmt().init()` 不同，此函数创建一个
/// 支持运行时动态修改日志级别的订阅者。
///
/// # 参数
/// - `default_level`: 默认日志级别，例如 "info"、"debug"
pub fn init_dynamic_logging(default_level: &str) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_level));

    let (filter_layer, reload_handle) = reload::Layer::new(env_filter);

    let subscriber = tracing_subscriber::registry()
        .with(filter_layer)
        .with(
            fmt::layer()
                .json()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true),
        );

    subscriber.init();

    LOG_RELOAD_HANDLE
        .set(reload_handle)
        .expect("日志重载句柄已初始化");
}

/// 获取当前日志级别
///
/// 如果无法获取句柄，返回 "unknown"
pub fn get_log_level() -> String {
    match LOG_RELOAD_HANDLE.get() {
        Some(handle) => handle
            .with_current(|filter| {
                // EnvFilter 没有直接获取当前级别的 API
                // 返回配置字符串的简化版本
                format!("{}", filter)
            })
            .unwrap_or_else(|_| "unknown".to_string()),
        None => "not_initialized".to_string(),
    }
}

/// 动态调整日志级别
///
/// # 参数
/// - `level`: 新的日志级别，支持以下格式：
///   - 简单级别: "trace", "debug", "info", "warn", "error"
///   - 模块级别: "im_api=debug,tower_http=info"
///   - RUST_LOG 格式: "im_api::handlers=debug,common=trace"
///
/// # 返回
/// - `Ok(())` 设置成功
/// - `Err(String)` 设置失败的原因
pub fn set_log_level(level: &str) -> Result<(), String> {
    let handle = LOG_RELOAD_HANDLE
        .get()
        .ok_or("日志重载句柄未初始化")?;

    let new_filter = EnvFilter::try_new(level)
        .map_err(|e| format!("无效的日志级别 '{}': {}", level, e))?;

    handle
        .reload(new_filter)
        .map_err(|e| format!("更新日志级别失败: {}", e))?;

    tracing::info!(level = level, "日志级别已动态更新");
    Ok(())
}
