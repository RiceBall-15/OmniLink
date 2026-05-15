//! 管理员用户管理数据库操作

use sqlx::PgPool;
use uuid::Uuid;

/// 用户管理查询结果
#[derive(Debug, sqlx::FromRow)]
pub struct AdminUserRow {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub status: Option<String>,
    pub online_status: Option<String>,
    pub last_active_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 获取用户列表（管理员 - 分页、搜索、筛选）
pub async fn get_users(
    pool: &PgPool,
    page: i64,
    limit: i64,
    search: Option<&str>,
    status: Option<&str>,
    sort_by: Option<&str>,
    sort_order: Option<&str>,
) -> Result<(Vec<AdminUserRow>, i64), String> {
    let offset = (page - 1) * limit;

    let mut where_clauses: Vec<String> = Vec::new();
    let mut bind_idx = 1;

    if search.is_some() {
        where_clauses.push(format!(
            "(username ILIKE ${} OR email ILIKE ${} OR nickname ILIKE ${})",
            bind_idx, bind_idx, bind_idx
        ));
        bind_idx += 1;
    }

    if status.is_some() {
        where_clauses.push(format!("COALESCE(status, 'active') = ${}", bind_idx));
        bind_idx += 1;
    }

    let where_str = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let sort_column = match sort_by.unwrap_or("created_at") {
        "username" => "username",
        "email" => "email",
        "last_active" => "last_active_at",
        "created_at" => "created_at",
        _ => "created_at",
    };

    let order = match sort_order.unwrap_or("desc") {
        "asc" => "ASC",
        _ => "DESC",
    };

    // Count total
    let count_query = format!("SELECT COUNT(*) FROM users {}", where_str);
    let mut count_sql = sqlx::query_scalar::<_, i64>(&count_query);
    if let Some(s) = search {
        let pattern = format!("%{}%", s);
        count_sql = count_sql.bind(pattern);
    }
    if let Some(st) = status {
        count_sql = count_sql.bind(st);
    }
    let total = count_sql
        .fetch_one(pool)
        .await
        .map_err(|e| format!("查询用户总数失败: {}", e))?;

    // Fetch users
    let data_query = format!(
        r#"SELECT id, username, email, nickname, avatar,
                  COALESCE(status, 'active') as status,
                  COALESCE(online_status, 'offline') as online_status,
                  last_active_at, created_at, updated_at
           FROM users {} ORDER BY {} {} LIMIT ${} OFFSET ${}"#,
        where_str, sort_column, order, bind_idx, bind_idx + 1
    );

    let mut data_sql = sqlx::query_as::<_, AdminUserRow>(&data_query);
    if let Some(s) = search {
        let pattern = format!("%{}%", s);
        data_sql = data_sql.bind(pattern);
    }
    if let Some(st) = status {
        data_sql = data_sql.bind(st);
    }
    data_sql = data_sql.bind(limit).bind(offset);

    let users = data_sql
        .fetch_all(pool)
        .await
        .map_err(|e| format!("查询用户列表失败: {}", e))?;

    Ok((users, total))
}

/// 更新用户状态（封禁/解封）
pub async fn update_user_status(
    pool: &PgPool,
    user_id: &Uuid,
    status: &str,
) -> Result<bool, String> {
    let result = sqlx::query(
        "UPDATE users SET status = $1, updated_at = NOW() WHERE id = $2"
    )
    .bind(status)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| format!("更新用户状态失败: {}", e))?;

    Ok(result.rows_affected() > 0)
}

/// 获取用户详情（管理员视图）
pub async fn get_user_detail(
    pool: &PgPool,
    user_id: &Uuid,
) -> Result<Option<AdminUserRow>, String> {
    let user = sqlx::query_as::<_, AdminUserRow>(
        r#"SELECT id, username, email, nickname, avatar,
                  COALESCE(status, 'active') as status,
                  COALESCE(online_status, 'offline') as online_status,
                  last_active_at, created_at, updated_at
           FROM users WHERE id = $1"#
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("查询用户详情失败: {}", e))?;

    Ok(user)
}

/// 获取用户消息统计（管理员用）
pub async fn get_user_message_stats(
    pool: &PgPool,
    user_id: &Uuid,
) -> Result<(i64, i64, i64, i64, i64), String> {
    let row = sqlx::query_as::<_, (i64, i64, i64, i64, i64)>(
        r#"SELECT
            COUNT(*) as total_messages,
            COUNT(*) FILTER (WHERE created_at >= NOW() - INTERVAL '1 day') as messages_today,
            COUNT(*) FILTER (WHERE created_at >= NOW() - INTERVAL '7 days') as messages_this_week,
            COUNT(*) FILTER (WHERE created_at >= NOW() - INTERVAL '30 days') as messages_this_month,
            COUNT(DISTINCT conversation_id) as active_conversations
           FROM messages WHERE sender_id = $1"#
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("查询用户消息统计失败: {}", e))?;

    Ok(row)
}

/// 获取用户高峰活动时段
pub async fn get_user_peak_hours(
    pool: &PgPool,
    user_id: &Uuid,
) -> Result<Vec<(i32, i64)>, String> {
    let rows = sqlx::query_as::<_, (i32, i64)>(
        r#"SELECT EXTRACT(HOUR FROM created_at)::integer as hour, COUNT(*) as cnt
           FROM messages WHERE sender_id = $1
           GROUP BY hour ORDER BY cnt DESC LIMIT 5"#
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("查询用户高峰时段失败: {}", e))?;

    Ok(rows)
}

/// 确保 users 表有 status 和 last_active_at 列
pub async fn ensure_user_columns(pool: &PgPool) -> Result<(), String> {
    sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS status VARCHAR(20) DEFAULT 'active'")
        .execute(pool)
        .await
        .map_err(|e| format!("添加 status 列失败: {}", e))?;

    sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS last_active_at TIMESTAMP WITH TIME ZONE")
        .execute(pool)
        .await
        .map_err(|e| format!("添加 last_active_at 列失败: {}", e))?;

    Ok(())
}

// ============================================================
// 仪表盘数据查询
// ============================================================

/// 仪表盘概览数据
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct DashboardOverview {
    pub total_users: i64,
    pub active_users_today: i64,
    pub active_users_week: i64,
    pub total_messages: i64,
    pub messages_today: i64,
    pub messages_this_week: i64,
    pub total_conversations: i64,
    pub active_conversations: i64,
    pub total_files: i64,
    pub files_today: i64,
    pub online_users: i64,
}

/// 获取仪表盘概览数据
pub async fn get_dashboard_overview(pool: &PgPool) -> Result<DashboardOverview, String> {
    let overview = sqlx::query_as::<_, DashboardOverview>(
        r#"SELECT
            (SELECT COUNT(*) FROM users) as total_users,
            (SELECT COUNT(DISTINCT sender_id) FROM messages WHERE created_at >= NOW() - INTERVAL '1 day') as active_users_today,
            (SELECT COUNT(DISTINCT sender_id) FROM messages WHERE created_at >= NOW() - INTERVAL '7 days') as active_users_week,
            (SELECT COUNT(*) FROM messages) as total_messages,
            (SELECT COUNT(*) FROM messages WHERE created_at >= NOW() - INTERVAL '1 day') as messages_today,
            (SELECT COUNT(*) FROM messages WHERE created_at >= NOW() - INTERVAL '7 days') as messages_this_week,
            (SELECT COUNT(*) FROM conversations) as total_conversations,
            (SELECT COUNT(*) FROM conversations WHERE updated_at >= NOW() - INTERVAL '7 days') as active_conversations,
            (SELECT COUNT(*) FROM files) as total_files,
            (SELECT COUNT(*) FROM files WHERE created_at >= NOW() - INTERVAL '1 day') as files_today,
            (SELECT COUNT(*) FROM users WHERE online_status = 'online') as online_users"#
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("查询仪表盘概览失败: {}", e))?;

    Ok(overview)
}

/// 用户增长趋势条目
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct GrowthTrendEntry {
    pub date: chrono::NaiveDate,
    pub count: i64,
}

/// 获取用户增长趋势
pub async fn get_user_growth_trend(
    pool: &PgPool,
    days: i64,
) -> Result<Vec<GrowthTrendEntry>, String> {
    let rows = sqlx::query_as::<_, GrowthTrendEntry>(
        r#"SELECT DATE(created_at) as date, COUNT(*) as count
           FROM users
           WHERE created_at >= NOW() - ($1 || ' days')::INTERVAL
           GROUP BY DATE(created_at)
           ORDER BY date ASC"#
    )
    .bind(days)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("查询用户增长趋势失败: {}", e))?;

    Ok(rows)
}

/// 消息量趋势条目
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct MessageTrendEntry {
    pub date: chrono::NaiveDate,
    pub count: i64,
}

/// 获取消息量趋势
pub async fn get_message_trend(
    pool: &PgPool,
    days: i64,
) -> Result<Vec<MessageTrendEntry>, String> {
    let rows = sqlx::query_as::<_, MessageTrendEntry>(
        r#"SELECT DATE(created_at) as date, COUNT(*) as count
           FROM messages
           WHERE created_at >= NOW() - ($1 || ' days')::INTERVAL
           GROUP BY DATE(created_at)
           ORDER BY date ASC"#
    )
    .bind(days)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("查询消息量趋势失败: {}", e))?;

    Ok(rows)
}

/// 系统资源统计
#[derive(Debug, serde::Serialize)]
pub struct SystemStats {
    pub database_size: Option<String>,
    pub total_tables: i64,
    pub uptime_seconds: i64,
    pub memory: MemoryInfo,
    pub cpu: CpuInfo,
    pub disk: DiskInfo,
}

/// 内存信息
#[derive(Debug, serde::Serialize)]
pub struct MemoryInfo {
    pub total_mb: u64,
    pub used_mb: u64,
    pub available_mb: u64,
    pub usage_percent: f64,
}

/// CPU 信息
#[derive(Debug, serde::Serialize)]
pub struct CpuInfo {
    pub cores: u32,
    pub load_avg_1m: f64,
    pub load_avg_5m: f64,
    pub load_avg_15m: f64,
}

/// 磁盘信息
#[derive(Debug, serde::Serialize)]
pub struct DiskInfo {
    pub total_gb: f64,
    pub used_gb: f64,
    pub available_gb: f64,
    pub usage_percent: f64,
}

/// 读取 /proc/meminfo 获取内存信息
fn read_memory_info() -> MemoryInfo {
    let content = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total_kb: u64 = 0;
    let mut available_kb: u64 = 0;

    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            total_kb = line.split_whitespace().nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        } else if line.starts_with("MemAvailable:") {
            available_kb = line.split_whitespace().nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        }
    }

    let total_mb = total_kb / 1024;
    let available_mb = available_kb / 1024;
    let used_mb = total_mb.saturating_sub(available_mb);
    let usage_percent = if total_mb > 0 {
        (used_mb as f64 / total_mb as f64) * 100.0
    } else {
        0.0
    };

    MemoryInfo {
        total_mb,
        used_mb,
        available_mb,
        usage_percent: (usage_percent * 100.0).round() / 100.0,
    }
}

/// 读取 /proc/loadavg 获取 CPU 负载信息
fn read_cpu_info() -> CpuInfo {
    let load_content = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let parts: Vec<&str> = load_content.split_whitespace().collect();

    let load_1m = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let load_5m = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let load_15m = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);

    // 获取 CPU 核心数
    let cores = std::thread::available_parallelism()
        .map(|p| p.get() as u32)
        .unwrap_or(1);

    CpuInfo {
        cores,
        load_avg_1m: load_1m,
        load_avg_5m: load_5m,
        load_avg_15m: load_15m,
    }
}

/// 读取磁盘使用信息
fn read_disk_info() -> DiskInfo {
    // 读取根分区使用情况
    let output = std::process::Command::new("df")
        .args(["-B1", "/"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();

    let mut total: u64 = 0;
    let mut used: u64 = 0;
    let mut available: u64 = 0;

    for line in output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            total = parts[1].parse().unwrap_or(0);
            used = parts[2].parse().unwrap_or(0);
            available = parts[3].parse().unwrap_or(0);
        }
    }

    let total_gb = total as f64 / (1024.0 * 1024.0 * 1024.0);
    let used_gb = used as f64 / (1024.0 * 1024.0 * 1024.0);
    let available_gb = available as f64 / (1024.0 * 1024.0 * 1024.0);
    let usage_percent = if total > 0 {
        (used as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    DiskInfo {
        total_gb: (total_gb * 100.0).round() / 100.0,
        used_gb: (used_gb * 100.0).round() / 100.0,
        available_gb: (available_gb * 100.0).round() / 100.0,
        usage_percent: (usage_percent * 100.0).round() / 100.0,
    }
}

/// 获取系统统计信息（不依赖数据库的静态信息）
pub fn get_system_stats() -> SystemStats {
    let uptime = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    SystemStats {
        database_size: None, // 需要超级用户权限查询
        total_tables: 0,
        uptime_seconds: uptime,
        memory: read_memory_info(),
        cpu: read_cpu_info(),
        disk: read_disk_info(),
    }
}
