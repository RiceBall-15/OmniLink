use axum::{extract::State, http::StatusCode, Json};
use crate::middleware::auth::AuthUser;
use crate::db::api_key as api_key_db;
use crate::models::api_key::{
    ApiKeyInfo, CreateApiKeyRequest, CreateApiKeyResponse,
};
use crate::models::auth::ApiResponse;
use sqlx::PgPool;
use uuid::Uuid;

/// 创建新的 API Key
#[utoipa::path(
    post,
    path = "/api/admin/api-keys",
    request_body = CreateApiKeyRequest,
    responses(
        (status = 201, description = "API Key 创建成功", body = ApiResponse<CreateApiKeyResponse>),
        (status = 400, description = "请求参数错误"),
    ),
    tag = "admin"
)]
pub async fn create_api_key(
    State(pool): State<PgPool>,
    auth_user: AuthUser,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<ApiResponse<CreateApiKeyResponse>>), (StatusCode, Json<ApiResponse<()>>)> {
    let owner_id = Uuid::parse_str(&auth_user.user_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID")),
        )
    })?;

    // 验证名称非空
    if req.name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", "Name cannot be empty")),
        ));
    }

    // 验证权限值
    if let Some(ref perm) = req.permissions {
        match perm.as_str() {
            "read" | "read_write" | "admin" => {}
            _ => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error("INVALID_INPUT", "Invalid permission: must be 'read', 'read_write', or 'admin'")),
                ));
            }
        }
    }

    let (entity, raw_key) = api_key_db::create_api_key(&pool, owner_id, &req)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("DATABASE_ERROR", format!("Failed to create API key: {}", e))),
            )
        })?;

    let response = CreateApiKeyResponse {
        id: entity.id,
        key: raw_key,
        key_prefix: entity.key_prefix,
        name: entity.name,
        permissions: entity.permissions,
        rate_limit: entity.rate_limit,
        expires_at: entity.expires_at,
        created_at: entity.created_at,
    };

    Ok((StatusCode::CREATED, Json(ApiResponse::success(response))))
}

/// 获取当前用户的 API Key 列表
#[utoipa::path(
    get,
    path = "/api/admin/api-keys",
    responses(
        (status = 200, description = "API Key 列表", body = ApiResponse<Vec<ApiKeyInfo>>),
    ),
    tag = "admin"
)]
pub async fn get_api_keys(
    State(pool): State<PgPool>,
    auth_user: AuthUser,
) -> Result<Json<ApiResponse<Vec<ApiKeyInfo>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let owner_id = Uuid::parse_str(&auth_user.user_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID")),
        )
    })?;

    let keys = api_key_db::get_api_keys_by_owner(&pool, owner_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("DATABASE_ERROR", format!("Failed to fetch API keys: {}", e))),
            )
        })?;

    let infos: Vec<ApiKeyInfo> = keys
        .into_iter()
        .map(|k| ApiKeyInfo {
            id: k.id,
            key_prefix: k.key_prefix,
            name: k.name,
            permissions: k.permissions,
            rate_limit: k.rate_limit,
            is_active: k.is_active,
            last_used_at: k.last_used_at,
            expires_at: k.expires_at,
            created_at: k.created_at,
        })
        .collect();

    Ok(Json(ApiResponse::success(infos)))
}

/// 停用 API Key
#[utoipa::path(
    delete,
    path = "/api/admin/api-keys/{key_id}",
    params(
        ("key_id" = Uuid, Path, description = "API Key ID"),
    ),
    responses(
        (status = 200, description = "API Key 已停用"),
        (status = 404, description = "API Key 不存在"),
    ),
    tag = "admin"
)]
pub async fn deactivate_api_key(
    State(pool): State<PgPool>,
    auth_user: AuthUser,
    axum::extract::Path(key_id): axum::extract::Path<uuid::Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let owner_id = Uuid::parse_str(&auth_user.user_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_USER_ID", "无效的用户ID")),
        )
    })?;

    let success = api_key_db::deactivate_api_key(&pool, key_id, owner_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("DATABASE_ERROR", format!("Failed to deactivate API key: {}", e))),
            )
        })?;

    if success {
        Ok(Json(ApiResponse::success(serde_json::json!({
            "message": "API key deactivated",
            "id": key_id
        }))))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "API key not found")),
        ))
    }
}
