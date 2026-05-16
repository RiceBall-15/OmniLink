//! 系统指标收集模块
//!
//! 收集 CPU、内存、磁盘使用率等系统级指标，
//! 用于管理端点和监控仪表板。

use serde::{Deserialize, Serialize};
use std::time::Instant;
use tokio::sync::RwLock;

/// 系统指标快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// CPU 使用率百分比 (0-100)
    pub cpu_usage_percent: f64,
    /// CPU 核心数
    pub cpu_cores: u32,
    /// 系统负载 (1/5/15 分钟)
    pub load_average: LoadAverage,
    /// 内存指标
    pub memory: MemoryMetrics,
    /// 磁盘指标
    pub disk: DiskMetrics,
    /// 进程指标
    pub process: ProcessMetrics,
    /// 指标采集时间
    pub collected_at: chrono::DateTime<chrono::Utc>,
    /// 指标采集耗时 (微秒)
    pub collection_duration_us: u64,
}

/// 系统负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadAverage {
    pub one_min: f64,
    pub five_min: f64,
    pub fifteen_min: f64,
}

/// 内存指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetrics {
    /// 总内存 (bytes)
    pub total_bytes: u64,
    /// 已用内存 (bytes)
    pub used_bytes: u64,
    /// 可用内存 (bytes)
    pub available_bytes: u64,
    /// 内存使用率百分比
    pub usage_percent: f64,
    /// 缓存内存 (bytes)
    pub cached_bytes: u64,
    /// Buffer 内存 (bytes)
    pub buffers_bytes: u64,
}

/// 磁盘指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskMetrics {
    /// 根分区总空间 (bytes)
    pub total_bytes: u64,
    /// 已用空间 (bytes)
    pub used_bytes: u64,
    /// 可用空间 (bytes)
    pub available_bytes: u64,
    /// 使用率百分比
    pub usage_percent: f64,
}

/// 进程指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMetrics {
    /// 当前进程 ID
    pub pid: u32,
    /// 进程内存使用 (bytes, RSS)
    pub rss_bytes: u64,
    /// 进程虚拟内存 (bytes)
    pub vsize_bytes: u64,
    /// 进程 CPU 使用率百分比
    pub cpu_percent: f64,
    /// 打开的文件描述符数
    pub open_fds: u32,
    /// 线程数
    pub thread_count: u32,
}

/// 历史指标记录（用于计算 CPU 使用率）
#[allow(dead_code)]
struct CpuSnapshot {
    idle: u64,
    total: u64,
    timestamp: Instant,
}

/// 系统指标收集器
#[allow(dead_code)]
pub struct SystemMetricsCollector {
    /// 上一次 CPU 快照
    last_cpu: RwLock<Option<CpuSnapshot>>,
    /// 进程启动时间
    start_time: Instant,
    /// 进程 ID
    pid: u32,
}

impl SystemMetricsCollector {
    /// 创建新的指标收集器
    pub fn new() -> Self {
        Self {
            last_cpu: RwLock::new(None),
            start_time: Instant::now(),
            pid: std::process::id(),
        }
    }

    /// 收集系统指标快照
    pub async fn collect(&self) -> SystemMetrics {
        let start = Instant::now();

        let cpu_cores = num_cpus();
        let cpu_usage = self.cpu_usage().await.unwrap_or(0.0);
        let load_average = read_load_average().unwrap_or(LoadAverage {
            one_min: 0.0,
            five_min: 0.0,
            fifteen_min: 0.0,
        });
        let memory = read_memory_metrics().unwrap_or(MemoryMetrics {
            total_bytes: 0,
            used_bytes: 0,
            available_bytes: 0,
            usage_percent: 0.0,
            cached_bytes: 0,
            buffers_bytes: 0,
        });
        let disk = read_disk_metrics().unwrap_or(DiskMetrics {
            total_bytes: 0,
            used_bytes: 0,
            available_bytes: 0,
            usage_percent: 0.0,
        });
        let process = read_process_metrics(self.pid).unwrap_or(ProcessMetrics {
            pid: self.pid,
            rss_bytes: 0,
            vsize_bytes: 0,
            cpu_percent: 0.0,
            open_fds: 0,
            thread_count: 0,
        });

        let duration = start.elapsed();

        SystemMetrics {
            cpu_usage_percent: cpu_usage,
            cpu_cores,
            load_average,
            memory,
            disk,
            process,
            collected_at: chrono::Utc::now(),
            collection_duration_us: duration.as_micros() as u64,
        }
    }

    /// 计算 CPU 使用率（需要两次采样）
    async fn cpu_usage(&self) -> Option<f64> {
        let (idle, total) = read_cpu_times()?;

        let mut last = self.last_cpu.write().await;
        let result = if let Some(ref prev) = *last {
            let idle_delta = idle.saturating_sub(prev.idle);
            let total_delta = total.saturating_sub(prev.total);
            if total_delta > 0 {
                Some((1.0 - (idle_delta as f64 / total_delta as f64)) * 100.0)
            } else {
                Some(0.0)
            }
        } else {
            // 第一次采样，返回0
            Some(0.0)
        };

        *last = Some(CpuSnapshot {
            idle,
            total,
            timestamp: Instant::now(),
        });

        result
    }
}

impl Default for SystemMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// 读取 /proc/stat 中的 CPU 时间
fn read_cpu_times() -> Option<(u64, u64)> {
    let content = std::fs::read_to_string("/proc/stat").ok()?;
    let first_line = content.lines().next()?;

    // 第一行格式: cpu  user nice system idle iowait irq softirq steal
    let fields: Vec<u64> = first_line
        .split_whitespace()
        .skip(1) // 跳过 "cpu" 标签
        .filter_map(|s| s.parse::<u64>().ok())
        .collect();

    if fields.len() < 4 {
        return None;
    }

    let idle = fields[3]; // idle 是第4个字段
    let total: u64 = fields.iter().sum();

    Some((idle, total))
}

/// 读取 CPU 核心数
fn num_cpus() -> u32 {
    std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .map(|content| {
            content
                .lines()
                .filter(|line| line.starts_with("processor"))
                .count() as u32
        })
        .unwrap_or(1)
}

/// 读取系统负载
fn read_load_average() -> Option<LoadAverage> {
    let content = std::fs::read_to_string("/proc/loadavg").ok()?;
    let fields: Vec<&str> = content.split_whitespace().collect();

    if fields.len() < 3 {
        return None;
    }

    Some(LoadAverage {
        one_min: fields[0].parse().ok()?,
        five_min: fields[1].parse().ok()?,
        fifteen_min: fields[2].parse().ok()?,
    })
}

/// 读取 /proc/meminfo 中的内存信息
fn read_memory_metrics() -> Option<MemoryMetrics> {
    let content = std::fs::read_to_string("/proc/meminfo").ok()?;

    let get_kb = |key: &str| -> Option<u64> {
        content
            .lines()
            .find(|line| line.starts_with(key))
            .and_then(|line| {
                line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse::<u64>().ok())
            })
    };

    let total_kb = get_kb("MemTotal:")?;
    let available_kb = get_kb("MemAvailable:").unwrap_or_else(|| {
        let free = get_kb("MemFree:").unwrap_or(0);
        let buffers = get_kb("Buffers:").unwrap_or(0);
        let cached = get_kb("Cached:").unwrap_or(0);
        free + buffers + cached
    });
    let cached_kb = get_kb("Cached:").unwrap_or(0);
    let buffers_kb = get_kb("Buffers:").unwrap_or(0);

    let total_bytes = total_kb * 1024;
    let available_bytes = available_kb * 1024;
    let used_bytes = total_bytes.saturating_sub(available_bytes);
    let usage_percent = if total_bytes > 0 {
        (used_bytes as f64 / total_bytes as f64) * 100.0
    } else {
        0.0
    };

    Some(MemoryMetrics {
        total_bytes,
        used_bytes,
        available_bytes,
        usage_percent,
        cached_bytes: cached_kb * 1024,
        buffers_bytes: buffers_kb * 1024,
    })
}

/// 读取磁盘使用率（根分区）
fn read_disk_metrics() -> Option<DiskMetrics> {
    let output = std::process::Command::new("df")
        .args(["-B1", "/"])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().nth(1)?; // 跳过标题行

    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 5 {
        return None;
    }

    let total_bytes: u64 = fields[1].parse().ok()?;
    let used_bytes: u64 = fields[2].parse().ok()?;
    let available_bytes: u64 = fields[3].parse().ok()?;
    let usage_percent_str = fields[4].trim_end_matches('%');
    let usage_percent: f64 = usage_percent_str.parse().ok()?;

    Some(DiskMetrics {
        total_bytes,
        used_bytes,
        available_bytes,
        usage_percent,
    })
}

/// 读取进程指标
fn read_process_metrics(pid: u32) -> Option<ProcessMetrics> {
    let status_path = format!("/proc/{}/status", pid);
    let status_content = std::fs::read_to_string(&status_path).ok()?;

    let get_status_value = |key: &str| -> Option<u64> {
        status_content
            .lines()
            .find(|line| line.starts_with(key))
            .and_then(|line| {
                line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse::<u64>().ok())
            })
    };

    // VmRSS: 常驻内存 (KB)
    let rss_bytes = get_status_value("VmRSS:").unwrap_or(0) * 1024;
    // VmSize: 虚拟内存 (KB)
    let vsize_bytes = get_status_value("VmSize:").unwrap_or(0) * 1024;
    // Threads
    let thread_count = get_status_value("Threads:").unwrap_or(1) as u32;

    // 读取打开的文件描述符数
    let fd_path = format!("/proc/{}/fd", pid);
    let open_fds = std::fs::read_dir(&fd_path)
        .map(|entries| entries.count() as u32)
        .unwrap_or(0);

    // 读取进程 CPU 使用率 (从 /proc/[pid]/stat)
    let stat_path = format!("/proc/{}/stat", pid);
    let cpu_percent = std::fs::read_to_string(&stat_path)
        .ok()
        .and_then(|content| {
            let fields: Vec<&str> = content.split_whitespace().collect();
            if fields.len() >= 15 {
                let utime: u64 = fields[13].parse().ok()?;
                let stime: u64 = fields[14].parse().ok()?;
                let total_time = utime + stime;
                // 简化的CPU使用率计算
                Some((total_time as f64 / 100.0) % 100.0)
            } else {
                None
            }
        })
        .unwrap_or(0.0);

    Some(ProcessMetrics {
        pid,
        rss_bytes,
        vsize_bytes,
        cpu_percent,
        open_fds,
        thread_count,
    })
}

/// 服务响应时间记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceResponseTime {
    pub service_name: String,
    pub endpoint: String,
    pub response_time_ms: u64,
    pub status: ResponseStatus,
    pub checked_at: chrono::DateTime<chrono::Utc>,
}

/// 响应状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResponseStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Timeout,
    Unknown,
}

/// 服务响应时间统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseTimeStats {
    pub service_name: String,
    pub avg_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub min_ms: u64,
    pub max_ms: u64,
    pub sample_count: usize,
}

/// 请求吞吐量统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputStats {
    /// 每秒请求数 (最近1分钟平均)
    pub requests_per_sec: f64,
    /// 总请求数
    pub total_requests: u64,
    /// 活跃连接数
    pub active_connections: u32,
    /// 统计开始时间
    pub since: chrono::DateTime<chrono::Utc>,
}

/// 吞吐量追踪器
pub struct ThroughputTracker {
    /// 请求计数器
    counter: std::sync::atomic::AtomicU64,
    /// 统计开始时间
    start_time: Instant,
}

impl ThroughputTracker {
    pub fn new() -> Self {
        Self {
            counter: std::sync::atomic::AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    /// 记录一个请求
    pub fn record_request(&self) {
        self.counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// 获取吞吐量统计
    pub fn stats(&self, active_connections: u32) -> ThroughputStats {
        let total = self.counter.load(std::sync::atomic::Ordering::Relaxed);
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let rps = if elapsed > 0.0 {
            total as f64 / elapsed
        } else {
            0.0
        };

        ThroughputStats {
            requests_per_sec: rps,
            total_requests: total,
            active_connections,
            since: chrono::Utc::now() - chrono::Duration::seconds(elapsed as i64),
        }
    }
}

impl Default for ThroughputTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_metrics_collector_creation() {
        let collector = SystemMetricsCollector::new();
        assert_eq!(collector.pid, std::process::id());
    }

    #[tokio::test]
    async fn test_collect_metrics() {
        let collector = SystemMetricsCollector::new();
        // 第一次采集
        let metrics = collector.collect().await;
        assert!(metrics.cpu_cores > 0);
        assert!(metrics.memory.total_bytes > 0);
        assert!(metrics.collected_at <= chrono::Utc::now());

        // 第二次采集应该有 CPU 使用率数据
        tokio::time::sleep(Duration::from_millis(100)).await;
        let metrics2 = collector.collect().await;
        assert!(metrics2.cpu_usage_percent >= 0.0);
        assert!(metrics2.cpu_usage_percent <= 100.0);
    }

    #[test]
    fn test_read_cpu_times() {
        let result = read_cpu_times();
        // 在 Linux 上应该能读取到
        if cfg!(target_os = "linux") {
            assert!(result.is_some());
            let (idle, total) = result.unwrap();
            assert!(total > idle);
        }
    }

    #[test]
    fn test_num_cpus() {
        let cpus = num_cpus();
        assert!(cpus > 0);
        assert!(cpus <= 1024); // 合理上限
    }

    #[test]
    fn test_read_load_average() {
        let result = read_load_average();
        if cfg!(target_os = "linux") {
            assert!(result.is_some());
            let load = result.unwrap();
            assert!(load.one_min >= 0.0);
            assert!(load.five_min >= 0.0);
            assert!(load.fifteen_min >= 0.0);
        }
    }

    #[test]
    fn test_read_memory_metrics() {
        let result = read_memory_metrics();
        if cfg!(target_os = "linux") {
            assert!(result.is_some());
            let mem = result.unwrap();
            assert!(mem.total_bytes > 0);
            assert!(mem.available_bytes > 0);
            assert!(mem.usage_percent >= 0.0 && mem.usage_percent <= 100.0);
        }
    }

    #[test]
    fn test_read_disk_metrics() {
        let result = read_disk_metrics();
        assert!(result.is_some());
        let disk = result.unwrap();
        assert!(disk.total_bytes > 0);
        assert!(disk.usage_percent >= 0.0 && disk.usage_percent <= 100.0);
    }

    #[test]
    fn test_read_process_metrics() {
        let pid = std::process::id();
        let result = read_process_metrics(pid);
        if cfg!(target_os = "linux") {
            assert!(result.is_some());
            let proc = result.unwrap();
            assert_eq!(proc.pid, pid);
            assert!(proc.rss_bytes > 0);
            assert!(proc.thread_count > 0);
        }
    }

    #[test]
    fn test_throughput_tracker() {
        let tracker = ThroughputTracker::new();
        assert_eq!(tracker.stats(0).total_requests, 0);

        tracker.record_request();
        tracker.record_request();
        tracker.record_request();

        let stats = tracker.stats(5);
        assert_eq!(stats.total_requests, 3);
        assert_eq!(stats.active_connections, 5);
        assert!(stats.requests_per_sec >= 0.0);
    }

    #[test]
    fn test_metrics_serialization() {
        let metrics = SystemMetrics {
            cpu_usage_percent: 45.2,
            cpu_cores: 4,
            load_average: LoadAverage {
                one_min: 1.5,
                five_min: 1.0,
                fifteen_min: 0.8,
            },
            memory: MemoryMetrics {
                total_bytes: 8 * 1024 * 1024 * 1024,
                used_bytes: 4 * 1024 * 1024 * 1024,
                available_bytes: 4 * 1024 * 1024 * 1024,
                usage_percent: 50.0,
                cached_bytes: 1024 * 1024 * 1024,
                buffers_bytes: 512 * 1024 * 1024,
            },
            disk: DiskMetrics {
                total_bytes: 100 * 1024 * 1024 * 1024,
                used_bytes: 50 * 1024 * 1024 * 1024,
                available_bytes: 50 * 1024 * 1024 * 1024,
                usage_percent: 50.0,
            },
            process: ProcessMetrics {
                pid: 1234,
                rss_bytes: 100 * 1024 * 1024,
                vsize_bytes: 500 * 1024 * 1024,
                cpu_percent: 5.0,
                open_fds: 100,
                thread_count: 10,
            },
            collected_at: chrono::Utc::now(),
            collection_duration_us: 500,
        };

        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("cpu_usage_percent"));
        assert!(json.contains("memory"));
        assert!(json.contains("disk"));

        let deserialized: SystemMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.cpu_cores, 4);
        assert_eq!(deserialized.memory.total_bytes, 8 * 1024 * 1024 * 1024);
    }
}
