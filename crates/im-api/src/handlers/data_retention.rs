//! 数据保留策略 API Handler

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::data_retention;
use crate::middleware::auth::AuthUser;
use crate::models::auth::ApiResponse;
use crate::models::data_retention::{
    CleanupResult, CreateRetentionPolicyRequest, UpdateRetentionPolicyRequest,
};

/// 创建数据保留策略
pub async fn create_policy(
    State(pool): State<PgPool>,
    _auth: AuthUser,
    Json(req): Json<CreateRetentionPolicyRequest>,
) -> Result<Json<ApiResponse<Uuid>>, StatusCode> {
    if req.retention_days < 1 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let allowed_tables = vec!["messages", "message_reactions", "delivery_receipts", "chat_exports"];
    if !allowed_tables.contains(&req.target_table.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let policy_id = data_retention::create_policy(
        &pool,
        &req.name,
        req.description.as_deref(),
        req.retention_days,
        &req.target_table,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::success(policy_id)))
}

/// 获取所有保留策略
pub async fn get_policies(
    State(pool): State<PgPool>,
    _auth: AuthUser,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, StatusCode> {
    let policies = data_retention::get_all_policies(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result: Vec<serde_json::Value> = policies
        .into_iter()
        .map(|(id, name, desc, days, table, enabled, last_run, created, updated)| {
            serde_json::json!({
                "id": id,
                "name": name,
                "description": desc,
                "retention_days": days,
                "target_table": table,
                "is_enabled": enabled,
                "last_run_at": last_run,
                "created_at": created,
                "updated_at": updated,
            })
        })
        .collect();

    Ok(Json(ApiResponse::success(result)))
}

/// 获取单个策略
pub async fn get_policy(
    State(pool): State<PgPool>,
    _auth: AuthUser,
    Path(policy_id): Path<Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    let policy = data_retention::get_policy(&pool, policy_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match policy {
        Some((id, name, desc, days, table, enabled, last_run, created, updated)) => {
            Ok(Json(ApiResponse::success(serde_json::json!({
                "id": id,
                "name": name,
                "description": desc,
                "retention_days": days,
                "target_table": table,
                "is_enabled": enabled,
                "last_run_at": last_run,
                "created_at": created,
                "updated_at": updated,
            }))))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// 更新策略
pub async fn update_policy(
    State(pool): State<PgPool>,
    _auth: AuthUser,
    Path(policy_id): Path<Uuid>,
    Json(req): Json<UpdateRetentionPolicyRequest>,
) -> Result<Json<ApiResponse<bool>>, StatusCode> {
    let updated = data_retention::update_policy(
        &pool,
        policy_id,
        req.name.as_deref(),
        req.description.as_deref(),
        req.retention_days,
        req.is_enabled,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if updated {
        Ok(Json(ApiResponse::success(true)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// 删除策略
pub async fn delete_policy(
    State(pool): State<PgPool>,
    _auth: AuthUser,
    Path(policy_id): Path<Uuid>,
) -> Result<Json<ApiResponse<bool>>, StatusCode> {
    let deleted = data_retention::delete_policy(&pool, policy_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if deleted {
        Ok(Json(ApiResponse::success(true)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// 手动触发清理
pub async fn run_cleanup(
    State(pool): State<PgPool>,
    _auth: AuthUser,
) -> Result<Json<ApiResponse<Vec<CleanupResult>>>, StatusCode> {
    let policies = data_retention::get_enabled_policies(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut results = Vec::new();

    for (id, name, retention_days, target_table) in policies {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days as i64);

        let delete_query = match target_table.as_str() {
            "messages" => format!("DELETE FROM messages WHERE created_at < '{}'", cutoff),
            "message_reactions" => format!("DELETE FROM message_reactions WHERE created_at < '{}'", cutoff),
            "delivery_receipts" => format!("DELETE FROM delivery_receipts WHERE created_at < '{}'", cutoff),
            "chat_exports" => format!("DELETE FROM export_jobs WHERE created_at < '{}'", cutoff),
            _ => continue,
        };

        let result: Result<_, sqlx::Error> = sqlx::query(&delete_query)
            .execute(&pool)
            .await;

        let (rows_deleted, success, error_msg) = match result {
            Ok(r) => (r.rows_affected(), true, None),
            Err(e) => (0, false, Some(e.to_string())),
        };

        if success {
            let _ = data_retention::update_last_run(&pool, id).await;
        }

        results.push(CleanupResult {
            policy_name: name,
            target_table,
            rows_deleted,
            executed_at: chrono::Utc::now(),
            success,
            error_message: error_msg,
        });
    }

    Ok(Json(ApiResponse::success(results)))
}
