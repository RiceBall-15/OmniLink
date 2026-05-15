//! Webhook 管理 API Handler

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::webhook;
use crate::middleware::auth::AuthUser;
use crate::models::auth::ApiResponse;
use crate::models::webhook::{
    CreateWebhookRequest, UpdateWebhookRequest, Webhook, WebhookDelivery, WebhookDeliveryQuery,
    WebhookQuery,
};

/// 注册新 Webhook
#[utoipa::path(
    post,
    path = "/api/webhooks",
    tag = "webhooks",
    request_body = CreateWebhookRequest,
    responses(
        (status = 201, description = "注册成功", body = ApiResponse<Webhook>),
    )
)]
pub async fn create_webhook(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<CreateWebhookRequest>,
) -> Result<(StatusCode, Json<ApiResponse<Webhook>>), (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID".to_string())),
            ))
        }
    };

    // 验证 URL
    if req.url.is_empty() || (!req.url.starts_with("https://") && !req.url.starts_with("http://localhost")) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_URL",
                "URL 必须使用 HTTPS（localhost 除外）".to_string(),
            )),
        ));
    }

    if req.events.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_EVENTS", "事件列表不能为空".to_string())),
        ));
    }

    // 验证事件类型
    for event in &req.events {
        if event.parse::<crate::models::webhook::WebhookEventType>().is_err() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "INVALID_EVENT_TYPE",
                    format!("未知事件类型: {}", event),
                )),
            ));
        }
    }

    let entity = webhook::create_webhook(
        &pool,
        user_id,
        &req.url,
        req.secret.as_deref(),
        &req.events,
        req.description.as_deref(),
    )
    .await
    .map_err(|e| {
        tracing::error!("创建 Webhook 失败: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("DATABASE_ERROR", "创建 Webhook 失败".to_string())),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(entity.to_api())),
    ))
}

/// 获取用户的所有 Webhook
#[utoipa::path(
    get,
    path = "/api/webhooks",
    tag = "webhooks",
    params(
        ("is_active" = Option<bool>, Query, description = "按状态筛选")
    ),
    responses(
        (status = 200, description = "获取成功", body = ApiResponse<Vec<Webhook>>),
    )
)]
pub async fn get_webhooks(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Query(query): Query<WebhookQuery>,
) -> Result<(StatusCode, Json<ApiResponse<Vec<Webhook>>>), (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID".to_string())),
            ))
        }
    };

    let webhooks = webhook::get_user_webhooks(&pool, user_id, query.is_active)
        .await
        .map_err(|e| {
            tracing::error!("获取 Webhook 列表失败: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("DATABASE_ERROR", "获取 Webhook 列表失败".to_string())),
            )
        })?;

    Ok((
        StatusCode::OK,
        Json(ApiResponse::success(webhooks.iter().map(|w| w.to_api()).collect())),
    ))
}

/// 获取单个 Webhook 详情
#[utoipa::path(
    get,
    path = "/api/webhooks/{id}",
    tag = "webhooks",
    responses(
        (status = 200, description = "获取成功", body = ApiResponse<Webhook>),
        (status = 404, description = "Webhook 不存在"),
    )
)]
pub async fn get_webhook(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(webhook_id): Path<String>,
) -> Result<(StatusCode, Json<ApiResponse<Webhook>>), (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID".to_string())),
            ))
        }
    };

    let wh_id = Uuid::parse_str(&webhook_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_ID", "无效的 Webhook ID".to_string())),
        )
    })?;

    let entity = webhook::get_webhook(&pool, wh_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("获取 Webhook 失败: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("DATABASE_ERROR", "获取 Webhook 失败".to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Webhook 不存在".to_string())),
            )
        })?;

    Ok((
        StatusCode::OK,
        Json(ApiResponse::success(entity.to_api())),
    ))
}

/// 更新 Webhook
#[utoipa::path(
    put,
    path = "/api/webhooks/{id}",
    tag = "webhooks",
    request_body = UpdateWebhookRequest,
    responses(
        (status = 200, description = "更新成功", body = ApiResponse<Webhook>),
        (status = 404, description = "Webhook 不存在"),
    )
)]
pub async fn update_webhook(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(webhook_id): Path<String>,
    Json(req): Json<UpdateWebhookRequest>,
) -> Result<(StatusCode, Json<ApiResponse<Webhook>>), (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID".to_string())),
            ))
        }
    };

    let wh_id = Uuid::parse_str(&webhook_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_ID", "无效的 Webhook ID".to_string())),
        )
    })?;

    let entity = webhook::update_webhook(
        &pool,
        wh_id,
        user_id,
        req.url.as_deref(),
        req.secret.as_deref(),
        req.events.as_deref(),
        req.description.as_deref(),
        req.is_active,
    )
    .await
    .map_err(|e| {
        tracing::error!("更新 Webhook 失败: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("DATABASE_ERROR", "更新 Webhook 失败".to_string())),
        )
    })?
    .ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Webhook 不存在".to_string())),
        )
    })?;

    Ok((
        StatusCode::OK,
        Json(ApiResponse::success(entity.to_api())),
    ))
}

/// 删除 Webhook
#[utoipa::path(
    delete,
    path = "/api/webhooks/{id}",
    tag = "webhooks",
    responses(
        (status = 200, description = "删除成功"),
        (status = 404, description = "Webhook 不存在"),
    )
)]
pub async fn delete_webhook(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(webhook_id): Path<String>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID".to_string())),
            ))
        }
    };

    let wh_id = Uuid::parse_str(&webhook_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_ID", "无效的 Webhook ID".to_string())),
        )
    })?;

    let deleted = webhook::delete_webhook(&pool, wh_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("删除 Webhook 失败: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("DATABASE_ERROR", "删除 Webhook 失败".to_string())),
            )
        })?;

    if deleted {
        Ok((
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({"deleted": true}))),
        ))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Webhook 不存在".to_string())),
        ))
    }
}

/// 获取 Webhook 投递日志
#[utoipa::path(
    get,
    path = "/api/webhooks/{id}/deliveries",
    tag = "webhooks",
    params(
        ("event_type" = Option<String>, Query, description = "按事件类型筛选"),
        ("success" = Option<bool>, Query, description = "按投递状态筛选"),
        ("page" = Option<i64>, Query, description = "页码"),
        ("page_size" = Option<i64>, Query, description = "每页数量"),
    ),
    responses(
        (status = 200, description = "获取成功", body = ApiResponse<Vec<WebhookDelivery>>),
    )
)]
pub async fn get_deliveries(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(webhook_id): Path<String>,
    Query(query): Query<WebhookDeliveryQuery>,
) -> Result<(StatusCode, Json<ApiResponse<Vec<WebhookDelivery>>>), (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID".to_string())),
            ))
        }
    };

    let wh_id = Uuid::parse_str(&webhook_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_ID", "无效的 Webhook ID".to_string())),
        )
    })?;

    // 验证 webhook 存在且属于用户
    let exists = webhook::get_webhook(&pool, wh_id, user_id).await.map_err(|e| {
        tracing::error!("验证 Webhook 失败: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("DATABASE_ERROR", "查询失败".to_string())),
        )
    })?;

    if exists.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Webhook 不存在".to_string())),
        ));
    }

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).min(100).max(1);

    let logs = webhook::get_delivery_logs(
        &pool,
        wh_id,
        query.event_type.as_deref(),
        query.success,
        page,
        page_size,
    )
    .await
    .map_err(|e| {
        tracing::error!("获取投递日志失败: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("DATABASE_ERROR", "获取投递日志失败".to_string())),
        )
    })?;

    Ok((
        StatusCode::OK,
        Json(ApiResponse::success(logs.iter().map(|l| l.to_api()).collect())),
    ))
}
