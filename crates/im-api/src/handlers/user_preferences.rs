//! 用户偏好设置 API Handler

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::user_preferences;
use crate::middleware::auth::AuthUser;
use crate::models::auth::ApiResponse;
use crate::models::user_preferences::{
    AllPreferencesResponse, BatchSetPreferenceRequest, DefaultTemplatesResponse, PreferenceQuery,
    SetPreferenceRequest, UserPreference, get_default_templates,
};

/// 获取当前用户的所有偏好设置
#[utoipa::path(
    get,
    path = "/api/users/preferences",
    tag = "user-preferences",
    params(
        ("category" = Option<String>, Query, description = "按类别筛选")
    ),
    responses(
        (status = 200, description = "获取成功", body = ApiResponse<AllPreferencesResponse>),
    )
)]
pub async fn get_preferences(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Query(query): Query<PreferenceQuery>,
) -> Result<(StatusCode, Json<ApiResponse<AllPreferencesResponse>>), (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID".to_string())),
            ))
        }
    };

    let preferences = if let Some(ref category) = query.category {
        user_preferences::get_preferences_by_category(&pool, user_id, category)
            .await
            .map_err(|e| {
                tracing::error!("获取偏好设置失败: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("DATABASE_ERROR", "获取偏好设置失败".to_string())),
                )
            })?
    } else {
        user_preferences::get_all_preferences(&pool, user_id)
            .await
            .map_err(|e| {
                tracing::error!("获取偏好设置失败: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("DATABASE_ERROR", "获取偏好设置失败".to_string())),
                )
            })?
    };

    let categories = user_preferences::get_category_summary(&pool, user_id)
        .await
        .unwrap_or_default();

    let api_preferences: Vec<UserPreference> = preferences.iter().map(|p| p.to_api()).collect();
    let total_count = api_preferences.len();

    Ok((
        StatusCode::OK,
        Json(ApiResponse::success(AllPreferencesResponse {
            preferences: api_preferences,
            categories,
            total_count,
        })),
    ))
}

/// 设置单个偏好
#[utoipa::path(
    put,
    path = "/api/users/preferences",
    tag = "user-preferences",
    request_body = SetPreferenceRequest,
    responses(
        (status = 200, description = "设置成功", body = ApiResponse<UserPreference>),
    )
)]
pub async fn set_preference(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<SetPreferenceRequest>,
) -> Result<(StatusCode, Json<ApiResponse<UserPreference>>), (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID".to_string())),
            ))
        }
    };

    // 验证类别
    if req.category.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", "类别不能为空".to_string())),
        ));
    }

    // 验证键名
    if req.key.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", "键名不能为空".to_string())),
        ));
    }

    // 限制类别和键名长度
    if req.category.len() > 50 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", "类别长度不能超过50个字符".to_string())),
        ));
    }
    if req.key.len() > 100 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", "键名长度不能超过100个字符".to_string())),
        ));
    }

    let entity = user_preferences::set_preference(&pool, user_id, &req.category, &req.key, &req.value)
        .await
        .map_err(|e| {
            tracing::error!("设置偏好失败: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("DATABASE_ERROR", "设置偏好失败".to_string())),
            )
        })?;

    Ok((
        StatusCode::OK,
        Json(ApiResponse::success(entity.to_api())),
    ))
}

/// 批量设置偏好
#[utoipa::path(
    put,
    path = "/api/users/preferences/batch",
    tag = "user-preferences",
    request_body = BatchSetPreferenceRequest,
    responses(
        (status = 200, description = "批量设置成功", body = ApiResponse<Vec<UserPreference>>),
    )
)]
pub async fn batch_set_preferences(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<BatchSetPreferenceRequest>,
) -> Result<(StatusCode, Json<ApiResponse<Vec<UserPreference>>>), (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = match Uuid::parse_str(&auth.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID".to_string())),
            ))
        }
    };

    if req.preferences.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", "偏好列表不能为空".to_string())),
        ));
    }

    if req.preferences.len() > 50 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", "单次最多设置50个偏好".to_string())),
        ));
    }

    let mut results = Vec::new();
    for pref in &req.preferences {
        if pref.category.trim().is_empty() || pref.key.trim().is_empty() {
            continue;
        }
        match user_preferences::set_preference(&pool, user_id, &pref.category, &pref.key, &pref.value).await {
            Ok(entity) => results.push(entity.to_api()),
            Err(e) => {
                tracing::error!("批量设置偏好失败 ({}::{}): {}", pref.category, pref.key, e);
            }
        }
    }

    Ok((
        StatusCode::OK,
        Json(ApiResponse::success(results)),
    ))
}

/// 删除单个偏好
#[utoipa::path(
    delete,
    path = "/api/users/preferences",
    tag = "user-preferences",
    request_body = SetPreferenceRequest,
    responses(
        (status = 200, description = "删除成功"),
    )
)]
pub async fn delete_preference(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<SetPreferenceRequest>,
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

    let deleted = user_preferences::delete_preference(&pool, user_id, &req.category, &req.key)
        .await
        .map_err(|e| {
            tracing::error!("删除偏好失败: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("DATABASE_ERROR", "删除偏好失败".to_string())),
            )
        })?;

    if deleted {
        Ok((
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "deleted": true,
                "category": req.category,
                "key": req.key
            }))),
        ))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "偏好设置不存在".to_string())),
        ))
    }
}

/// 删除某类别下所有偏好
#[utoipa::path(
    delete,
    path = "/api/users/preferences/category/{category}",
    tag = "user-preferences",
    responses(
        (status = 200, description = "删除成功"),
    )
)]
pub async fn delete_category(
    State(pool): State<PgPool>,
    auth: AuthUser,
    axum::extract::Path(category): axum::extract::Path<String>,
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

    let count = user_preferences::delete_category_preferences(&pool, user_id, &category)
        .await
        .map_err(|e| {
            tracing::error!("删除类别偏好失败: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("DATABASE_ERROR", "删除偏好失败".to_string())),
            )
        })?;

    Ok((
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "deleted_count": count,
            "category": category
        }))),
    ))
}

/// 获取系统默认偏好模板列表
#[utoipa::path(
    get,
    path = "/api/preferences/templates",
    tag = "偏好设置",
    responses(
        (status = 200, description = "获取成功", body = ApiResponse<DefaultTemplatesResponse>),
    ),
    security(("bearer" = []))
)]
pub async fn get_templates() -> (StatusCode, Json<ApiResponse<DefaultTemplatesResponse>>) {
    let templates = get_default_templates();
    let total = templates.len();
    (
        StatusCode::OK,
        Json(ApiResponse::success(DefaultTemplatesResponse {
            templates,
            total_count: total,
        })),
    )
}

/// 应用默认偏好模板（仅设置用户尚未设置的偏好）
#[utoipa::path(
    post,
    path = "/api/preferences/templates/apply",
    tag = "偏好设置",
    responses(
        (status = 200, description = "应用成功", body = ApiResponse<serde_json::Value>),
    ),
    security(("bearer" = []))
)]
pub async fn apply_templates(
    auth: AuthUser,
    State(pool): State<PgPool>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let user_id = auth.user_id;
    let templates = get_default_templates();

    // 获取用户现有偏好
    let existing = user_preferences::get_user_preferences(&pool, user_id)
        .await
        .unwrap_or_default();
    let existing_keys: std::collections::HashSet<(String, String)> = existing
        .iter()
        .map(|p| (p.category.clone(), p.key.clone()))
        .collect();

    // 只设置用户尚未设置的偏好
    let mut applied = 0;
    for tmpl in &templates {
        if !existing_keys.contains(&(tmpl.category.clone(), tmpl.key.clone())) {
            let req = SetPreferenceRequest {
                category: tmpl.category.clone(),
                key: tmpl.key.clone(),
                value: tmpl.default_value.clone(),
            };
            if user_preferences::set_preference(&pool, user_id, &req)
                .await
                .is_ok()
            {
                applied += 1;
            }
        }
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "applied_count": applied,
            "skipped_count": templates.len() - applied,
            "message": format!("已应用 {} 个默认偏好设置，跳过 {} 个已有设置", applied, templates.len() - applied)
        }))),
    )
}
