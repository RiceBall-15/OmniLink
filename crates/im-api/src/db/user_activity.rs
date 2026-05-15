//! 用户活动追踪数据库操作

use sqlx::PgPool;
use uuid::Uuid;

/// 用户活动统计结果
#[derive(Debug, sqlx::FromRow)]
pub struct ActivityStatsRow {
    pub total_messages: i64,
    pub messages_today: i64,
    pub messages_this_week: i64,
    pub messages_this_month: i64,
    pub active_conversations: i64,
}

/// 按小时统计结果
#[derive(Debug, sqlx::FromRow)]
pub struct HourlyActivityRow {
    pub hour: i32,
    pub count: i64,
}

/// 获取用户活动统计
pub async fn get_user_activity_stats(
    pool: &PgPool,
    user_id: &Uuid,
) -> Result<ActivityStatsRow, String> {
    let stats = sqlx::query_as::<_, ActivityStatsRow>(
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
    .map_err(|e| format!("查询用户活动统计失败: {}", e))?;

    Ok(stats)
}

/// 获取用户按小时分布的活动模式
pub async fn get_user_activity_pattern(
    pool: &PgPool,
    user_id: &Uuid,
    days: i32,
) -> Result<Vec<HourlyActivityRow>, String> {
    let rows = sqlx::query_as::<_, HourlyActivityRow>(
        r#"SELECT EXTRACT(HOUR FROM created_at)::integer as hour, COUNT(*) as count
           FROM messages
           WHERE sender_id = $1 AND created_at >= NOW() - ($2 || ' days')::INTERVAL
           GROUP BY hour ORDER BY hour"#
    )
    .bind(user_id)
    .bind(days)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("查询活动模式失败: {}", e))?;

    Ok(rows)
}

/// 获取用户最近的活动记录（基于消息记录）
pub async fn get_recent_activities(
    pool: &PgPool,
    user_id: &Uuid,
    limit: i64,
) -> Result<Vec<(String, String, chrono::DateTime<chrono::Utc>)>, String> {
    let rows = sqlx::query_as::<_, (String, String, chrono::DateTime<chrono::Utc>)>(
        r#"SELECT
            'message' as activity_type,
            CASE
                WHEN LENGTH(content) > 50 THEN LEFT(content, 50) || '...'
                ELSE content
            END as description,
            created_at
           FROM messages WHERE sender_id = $1
           ORDER BY created_at DESC LIMIT $2"#
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("查询最近活动失败: {}", e))?;

    Ok(rows)
}

/// 获取用户文件上传统计
pub async fn get_user_file_count(
    pool: &PgPool,
    user_id: &Uuid,
) -> Result<i64, String> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM files WHERE uploader_id = $1"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("查询文件上传统计失败: {}", e))?;

    Ok(count)
}

/// 更新用户最后活跃时间
pub async fn update_last_active(
    pool: &PgPool,
    user_id: &Uuid,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE users SET last_active_at = NOW(), updated_at = NOW() WHERE id = $1"
    )
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| format!("更新最后活跃时间失败: {}", e))?;

    Ok(())
}

/// 获取会话统计摘要（增强版）
pub async fn get_conversation_stats_summary(
    pool: &PgPool,
    conversation_id: &Uuid,
) -> Result<ConversationStatsSummary, String> {
    // 基础统计
    let basic = sqlx::query_as::<_, BasicStatsRow>(
        r#"SELECT
            COUNT(*) as total_messages,
            COUNT(DISTINCT sender_id) as active_members,
            MIN(created_at) as first_message_at,
            MAX(created_at) as last_message_at
           FROM messages WHERE conversation_id = $1"#
    )
    .bind(conversation_id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("获取会话基础统计失败: {}", e))?;

    // 高峰时段
    let peak_hours = sqlx::query_as::<_, HourlyActivityRow>(
        r#"SELECT EXTRACT(HOUR FROM created_at)::integer as hour, COUNT(*) as count
           FROM messages WHERE conversation_id = $1
           GROUP BY hour ORDER BY count DESC LIMIT 5"#
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("获取高峰时段失败: {}", e))?;

    // 消息类型分布
    let type_distribution = sqlx::query_as::<_, TypeDistRow>(
        r#"SELECT type as msg_type, COUNT(*) as count
           FROM messages WHERE conversation_id = $1
           GROUP BY type ORDER BY count DESC"#
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("获取消息类型分布失败: {}", e))?;

    // 按日统计
    let daily_stats = sqlx::query_as::<_, TimeSeriesRow>(
        r#"SELECT DATE(created_at) as date, COUNT(*) as count
           FROM messages WHERE conversation_id = $1
             AND created_at >= NOW() - INTERVAL '30 days'
           GROUP BY date ORDER BY date"#
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("获取每日统计失败: {}", e))?;

    Ok(ConversationStatsSummary {
        total_messages: basic.total_messages,
        active_members: basic.active_members,
        first_message_at: basic.first_message_at.map(|d| d.to_string()),
        last_message_at: basic.last_message_at.map(|d| d.to_string()),
        peak_hours: peak_hours.into_iter().map(|r| (r.hour, r.count)).collect(),
        type_distribution: type_distribution.into_iter().map(|r| (r.msg_type, r.count)).collect(),
        daily_stats: daily_stats.into_iter().map(|r| (r.date.to_string(), r.count)).collect(),
    })
}

#[derive(Debug, sqlx::FromRow)]
struct BasicStatsRow {
    pub total_messages: i64,
    pub active_members: i64,
    pub first_message_at: Option<chrono::NaiveDateTime>,
    pub last_message_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, sqlx::FromRow)]
struct TypeDistRow {
    pub msg_type: String,
    pub count: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct TimeSeriesRow {
    pub date: chrono::NaiveDate,
    pub count: i64,
}

/// 会话统计摘要
#[derive(Debug)]
pub struct ConversationStatsSummary {
    pub total_messages: i64,
    pub active_members: i64,
    pub first_message_at: Option<String>,
    pub last_message_at: Option<String>,
    pub peak_hours: Vec<(i32, i64)>,
    pub type_distribution: Vec<(String, i64)>,
    pub daily_stats: Vec<(String, i64)>,
}
