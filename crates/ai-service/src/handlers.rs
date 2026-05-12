use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Json, Sse, sse::Event},
    Json as JsonResponse,
};
use std::sync::Arc;
use std::convert::Infallible;
use std::time::Duration;
use uuid::Uuid;
use serde::Deserialize;

use common::ApiResponse;
use crate::models::*;
use crate::services::AIService;
use crate::middleware::Auth;

/// Query parameters
#[derive(Debug, Deserialize)]
pub struct DateRangeQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// Pagination query parameters
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// Chat with AI assistant
pub async fn chat(
    State(service): State<Arc<AIService>>,
    Auth(claims): Auth,
    Json(request): Json<ChatRequest>,
) -> Result<JsonResponse<ApiResponse<ChatResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match service.chat(request, claims.sub).await {
        Ok(response) => Ok(JsonResponse(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("Chat error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(ApiResponse::error(500, e.to_string())),
            ))
        }
    }
}

/// Stream chat with AI assistant
pub async fn chat_stream(
    State(service): State<Arc<AIService>>,
    Auth(claims): Auth,
    Json(request): Json<ChatRequest>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let assistant_id = request.assistant_id;
    let conversation_id = request.conversation_id;
    let message_id = Uuid::new_v4();

    // Call the AI service to get a real provider stream
    let provider_result = service.chat_stream(request, claims.sub).await;

    // Map provider StreamChunk items to SSE events
    let stream = async_stream::stream! {
        match provider_result {
            Ok(mut provider_stream) => {
                use futures::StreamExt;
                let mut accumulated_content = String::new();

                while let Some(chunk_result) = provider_stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            accumulated_content.push_str(&chunk.content);
                            let data = serde_json::json!({
                                "conversation_id": conversation_id,
                                "assistant_id": assistant_id,
                                "message_id": message_id,
                                "delta": chunk.content,
                                "content": accumulated_content,
                                "model": chunk.model,
                                "done": chunk.done,
                            });
                            let event = Event::default().data(data.to_string());
                            yield Ok(event);

                            if chunk.done {
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::error!("Stream chunk error: {:?}", e);
                            let data = serde_json::json!({
                                "conversation_id": conversation_id,
                                "assistant_id": assistant_id,
                                "message_id": message_id,
                                "error": format!("{}", e),
                                "done": true,
                            });
                            let event = Event::default().data(data.to_string());
                            yield Ok(event);
                            break;
                        }
                    }
                }

                // Send final done event if we had content but no explicit done
                let data = serde_json::json!({
                    "conversation_id": conversation_id,
                    "assistant_id": assistant_id,
                    "message_id": message_id,
                    "delta": "",
                    "content": accumulated_content,
                    "done": true,
                });
                let event = Event::default().data(data.to_string());
                yield Ok(event);
            }
            Err(e) => {
                tracing::error!("Chat stream error: {:?}", e);
                let data = serde_json::json!({
                    "conversation_id": conversation_id,
                    "assistant_id": assistant_id,
                    "message_id": message_id,
                    "error": e.to_string(),
                    "done": true,
                });
                let event = Event::default().data(data.to_string());
                yield Ok(event);
            }
        }
    };

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15)))
}

/// Create AI assistant
pub async fn create_assistant(
    State(service): State<Arc<AIService>>,
    Auth(claims): Auth,
    Json(request): Json<CreateAssistantRequest>,
) -> Result<JsonResponse<ApiResponse<CreateAssistantResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match service.create_assistant(request, claims.sub).await {
        Ok(response) => Ok(JsonResponse(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("Create assistant error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(ApiResponse::error(500, e.to_string())),
            ))
        }
    }
}

/// List AI assistants
pub async fn list_assistants(
    State(service): State<Arc<AIService>>,
    Auth(claims): Auth,
) -> Result<JsonResponse<ApiResponse<AssistantsListResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match service.list_assistants(claims.sub).await {
        Ok(response) => Ok(JsonResponse(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("List assistants error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(ApiResponse::error(500, e.to_string())),
            ))
        }
    }
}

/// Get AI assistant detail
pub async fn get_assistant(
    State(service): State<Arc<AIService>>,
    Auth(_claims): Auth,
    Path(assistant_id): Path<Uuid>,
) -> Result<JsonResponse<ApiResponse<AssistantInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    match service.get_assistant(assistant_id).await {
        Ok(Some(info)) => Ok(JsonResponse(ApiResponse::success(info))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            JsonResponse(ApiResponse::error(404, "Assistant not found")),
        )),
        Err(e) => {
            tracing::error!("Get assistant error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(ApiResponse::error(500, e.to_string())),
            ))
        }
    }
}

/// Update AI assistant
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
                JsonResponse(ApiResponse::error(500, e.to_string())),
            ))
        }
    }
}

/// Delete AI assistant
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
                JsonResponse(ApiResponse::error(500, e.to_string())),
            ))
        }
    }
}

/// Get token usage statistics
pub async fn get_token_usage(
    State(service): State<Arc<AIService>>,
    Auth(claims): Auth,
    Query(query): Query<DateRangeQuery>,
) -> Result<JsonResponse<ApiResponse<TokenUsageResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match service.get_token_usage(claims.sub, query.start_date, query.end_date).await {
        Ok(response) => Ok(JsonResponse(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("Get token usage error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(ApiResponse::error(500, e.to_string())),
            ))
        }
    }
}

/// List supported models
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
                JsonResponse(ApiResponse::error(500, e.to_string())),
            ))
        }
    }
}

/// Get conversation history
pub async fn get_conversation_history(
    State(service): State<Arc<AIService>>,
    Auth(_claims): Auth,
    Path(conversation_id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<JsonResponse<ApiResponse<ConversationHistoryResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(50).min(100);

    match service.get_conversation_history(conversation_id, page, page_size).await {
        Ok(response) => Ok(JsonResponse(ApiResponse::success(response))),
        Err(e) => {
            tracing::error!("Get conversation history error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(ApiResponse::error(500, e.to_string())),
            ))
        }
    }
}

/// Clear conversation history
pub async fn clear_conversation(
    State(service): State<Arc<AIService>>,
    Auth(_claims): Auth,
    Path(conversation_id): Path<Uuid>,
) -> Result<JsonResponse<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match service.clear_conversation(conversation_id).await {
        Ok(_) => Ok(JsonResponse(ApiResponse::success(()))),
        Err(e) => {
            tracing::error!("Clear conversation error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(ApiResponse::error(500, e.to_string())),
            ))
        }
    }
}
