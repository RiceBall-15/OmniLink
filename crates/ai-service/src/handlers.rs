use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Json, Sse, sse::Event},
    Json as JsonResponse,
};
use axum_extra::extract::TypedHeader;
use headers::authorization::Bearer;
use std::sync::Arc;
use std::convert::Infallible;
use std::time::Duration;
use uuid::Uuid;
use serde::Deserialize;

use common::{ApiResponse, Claims};
use common::auth::TokenManager;
use crate::models::*;
use crate::services::AIService;
use crate::middleware::Auth;

/// 查询参数
#[derive(Debug, Deserialize)]
pub struct DateRangeQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// 发送AI对话请求
pub async fn chat(
    State(service): State<Arc<AIService>>,
    Auth(claims): Auth,
    Json(request): Json<ChatRequest>,
) -> Result<JsonResponse<ApiResponse<ChatResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match service.chat(request, claims.user_id).await {
        Ok(response) => Ok(JsonResponse(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("Chat error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(ApiResponse::error(e.to_string())),
            ))
        }
    }
}

/// 发送流式AI对话请求
pub async fn chat_stream(
    State(service): State<Arc<AIService>>,
    Auth(claims): Auth,
    Json(request): Json<ChatRequest>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    // 获取助手信息
    let assistant_id = request.assistant_id;
    let conversation_id = request.conversation_id;

    // 模拟流式响应
    let stream = async_stream::stream! {
        let mut content = String::new();
        let chunks = vec![
            "Hello", "!", " I", "'m", " your", " AI", " assistant",
            ".", " How", " can", " I", " help", " you", " today", "?"
        ];

        for chunk in chunks {
            content.push_str(chunk);
            let event = Event::default()
                .data(serde_json::json!({
                    "conversation_id": conversation_id,
                    "assistant_id": assistant_id,
                    "delta": chunk,
                    "content": content,
                    "done": false,
                }));
            yield Ok(event);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let event = Event::default()
            .data(serde_json::json!({
                "conversation_id": conversation_id,
                "assistant_id": assistant_id,
                "delta": "",
                "content": content,
                "done": true,
            }));
        yield Ok(event);
    };

    Sse::new(stream).keep_alive(Duration::from_secs(15))
}

/// 创建AI助手
pub async fn create_assistant(
    State(service): State<Arc<AIService>>,
    Auth(claims): Auth,
    Json(request): Json<CreateAssistantRequest>,
) -> Result<JsonResponse<ApiResponse<CreateAssistantResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match service.create_assistant(request, claims.user_id).await {
        Ok(response) => Ok(JsonResponse(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("Create assistant error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(ApiResponse::error(e.to_string())),
            ))
        }
    }
}

/// 获取AI助手列表
pub async fn list_assistants(
    State(service): State<Arc<AIService>>,
    Auth(claims): Auth,
) -> Result<JsonResponse<ApiResponse<AssistantsListResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match service.list_assistants(claims.user_id).await {
        Ok(response) => Ok(JsonResponse(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("List assistants error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(ApiResponse::error(e.to_string())),
            ))
        }
    }
}

/// 获取AI助手详情
pub async fn get_assistant(
    State(service): State<Arc<AIService>>,
    Auth(_claims): Auth,
    Path(assistant_id): Path<Uuid>,
) -> Result<JsonResponse<ApiResponse<AssistantInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    // 简化版本，直接返回
    Ok(JsonResponse(ApiResponse::success(AssistantInfo {
        id: assistant_id,
        name: "Assistant Name".to_string(),
        description: Some("Description".to_string()),
        model_id: "gpt-3.5-turbo".to_string(),
        system_prompt: None,
        temperature: Some(0.7),
        max_tokens: Some(2048),
        created_at: chrono::Utc::now().timestamp(),
    })))
}

/// 更新AI助手
pub async fn update_assistant(
    State(service): State<Arc<AIService>>,
    Auth(_claims): Auth,
    Path(assistant_id): Path<Uuid>,
    Json(request): Json<UpdateAssistantRequest>,
) -> Result<JsonResponse<ApiResponse<AssistantInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    match service.update_assistant(assistant_id, request).await {
        Ok(response) => Ok(JsonResponse(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("Update assistant error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(ApiResponse::error(e.to_string())),
            ))
        }
    }
}

/// 删除AI助手
pub async fn delete_assistant(
    State(service): State<Arc<AIService>>,
    Auth(_claims): Auth,
    Path(assistant_id): Path<Uuid>,
) -> Result<JsonResponse<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match service.delete_assistant(assistant_id).await {
        Ok(_) => Ok(JsonResponse(ApiResponse::success(()))),
        Err(e) => {
            tracing::error!("Delete assistant error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(ApiResponse::error(e.to_string())),
            ))
        }
    }
}

/// 获取Token使用统计
pub async fn get_token_usage(
    State(service): State<Arc<AIService>>,
    Auth(claims): Auth,
    Query(query): Query<DateRangeQuery>,
) -> Result<JsonResponse<ApiResponse<TokenUsageResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match service.get_token_usage(claims.user_id, query.start_date, query.end_date).await {
        Ok(response) => Ok(JsonResponse(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("Get token usage error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(ApiResponse::error(e.to_string())),
            ))
        }
    }
}

/// 获取支持的模型列表
pub async fn list_models(
    State(service): State<Arc<AIService>>,
    Auth(_claims): Auth,
) -> Result<JsonResponse<ApiResponse<ModelsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match service.list_models().await {
        Ok(response) => Ok(JsonResponse(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("List models error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(ApiResponse::error(e.to_string())),
            ))
        }
    }
}