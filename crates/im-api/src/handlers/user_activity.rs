//! 用户活动追踪 API Handler
//!
//! 提供用户活动追踪端点：
//! - `GET /api/users/activity` — 用户活动统计
//! - `GET /api/im/conversations/:id/stats` — 会话统计摘要（增强版）

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::user_activity;
use crate::middleware::auth::AuthUser;
use crate::models::auth::ApiResponse;
use crate::models::user_activity::ActivityQuery;

/// 获取用户活动统计
pub async fn get_my_activity(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Query(query): Query<ActivityQuery>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的用户 ID")),
            );
        }
    };

    // 基础活动统计
    let stats = match user_activity::get_user_activity_stats(&pool, &user_id).await {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("DB_ERROR", format!("获取活动统计失败: {}", e))),
            );
        }
    };

    // 活动模式（按时段）
    let days = query.days.unwrap_or(30);
    let pattern = match user_activity::get_user_activity_pattern(&pool, &user_id, days).await {
        Ok(p) => p
            .into_iter()
            .map(|r| serde_json::json!({"hour": r.hour, "count": r.count}))
            .collect::<Vec<_>>(),
        Err(_) => vec![],
    };

    // 最近活动
    let recent = match user_activity::get_recent_activities(&pool, &user_id, 10).await {
        Ok(activities) => activities
            .into_iter()
            .map(|(typ, desc, created)| {
                serde_json::json!({
                    "activity_type": typ,
                    "description": desc,
                    "created_at": created.to_rfc3339(),
                })
            })
            .collect::<Vec<_>>(),
        Err(_) => vec![],
    };

    // 文件上传统计
    let total_files = match user_activity::get_user_file_count(&pool, &user_id).await {
        Ok(c) => c,
        Err(_) => 0,
    };

    // 更新最后活跃时间
    let _ = user_activity::update_last_active(&pool, &user_id).await;

    let avg_per_day = if stats.messages_this_month > 0 {
        stats.messages_this_month as f64 / 30.0
    } else {
        0.0
    };

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "user_id": auth.user_id,
            "total_messages": stats.total_messages,
            "messages_today": stats.messages_today,
            "messages_this_week": stats.messages_this_week,
            "messages_this_month": stats.messages_this_month,
            "total_files_uploaded": total_files,
            "active_conversations": stats.active_conversations,
            "avg_messages_per_day": avg_per_day,
            "activity_pattern": pattern,
            "recent_activity": recent,
        }))),
    )
}

/// 获取会话统计摘要（增强版）
pub async fn get_conversation_stats(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(conversation_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let conv_id = match Uuid::parse_str(&conversation_id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的会话 ID")),
            );
        }
    };

    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "无效的用户 ID")),
            );
        }
    };

    // 验证用户是会话成员
    let is_member = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM conversation_participants WHERE conversation_id = $1 AND user_id = $2)"
    )
    .bind(conv_id)
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .unwrap_or(false);

    if !is_member {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "您不是该会话的成员")),
        );
    }

    // 获取统计摘要
    match user_activity::get_conversation_stats_summary(&pool, &conv_id).await {
        Ok(summary) => {
            let peak_hours: Vec<serde_json::Value> = summary
                .peak_hours
                .into_iter()
                .map(|(h, c)| serde_json::json!({"hour": h, "message_count": c}))
                .collect();

            let type_dist: Vec<serde_json::Value> = summary
                .type_distribution
                .into_iter()
                .map(|(t, c)| serde_json::json!({"type": t, "count": c}))
                .collect();

            let daily: Vec<serde_json::Value> = summary
                .daily_stats
                .into_iter()
                .map(|(d, c)| serde_json::json!({"date": d, "count": c}))
                .collect();

            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "conversation_id": conversation_id,
                    "total_messages": summary.total_messages,
                    "active_members": summary.active_members,
                    "first_message_at": summary.first_message_at,
                    "last_message_at": summary.last_message_at,
                    "peak_hours": peak_hours,
                    "type_distribution": type_dist,
                    "daily_stats": daily,
                }))),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("DB_ERROR", format!("获取会话统计失败: {}", e))),
        ),
    }
}
