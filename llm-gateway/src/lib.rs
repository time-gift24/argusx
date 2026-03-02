use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::StreamExt;
use llm_client::{LlmClient, LlmError, LlmMessage, LlmRequest, LlmRole, LlmTool};

#[derive(Clone)]
pub struct GatewayState {
    pub client: Arc<LlmClient>,
}

impl GatewayState {
    pub fn new(client: LlmClient) -> Self {
        Self {
            client: Arc::new(client),
        }
    }
}

pub fn app(state: GatewayState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

/// Convert bigmodel_api::ChatRequest to llm_client::LlmRequest
/// Returns error for unsupported content types or non-function tools
fn to_llm_request(req: bigmodel_api::ChatRequest) -> Result<LlmRequest, (StatusCode, String)> {
    // Validate message content types
    for m in &req.messages {
        if !matches!(m.content, bigmodel_api::Content::Text(_)) {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Unsupported message content type: only text content is supported, got {:?}", std::mem::discriminant(&m.content)),
            ));
        }
    }

    // Validate and convert tools
    let tools = if let Some(tools) = req.tools {
        let mut converted = Vec::new();
        for t in tools {
            match t {
                bigmodel_api::Tool::Function(ft) => converted.push(LlmTool {
                    name: ft.function.name,
                    description: ft.function.description,
                    parameters: ft.function.parameters,
                }),
                other => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!("Unsupported tool type: only function tools are supported, got {:?}", std::mem::discriminant(&other)),
                    ));
                }
            }
        }
        Some(converted)
    } else {
        None
    };

    let messages: Vec<LlmMessage> = req.messages.into_iter().map(|m| {
        let role = match m.role {
            bigmodel_api::Role::System => LlmRole::System,
            bigmodel_api::Role::User => LlmRole::User,
            bigmodel_api::Role::Assistant => LlmRole::Assistant,
            bigmodel_api::Role::Tool => LlmRole::Tool,
        };
        let content = match m.content {
            bigmodel_api::Content::Text(s) => s,
            _ => String::new(), // This should never happen due to validation above
        };
        LlmMessage { role, content }
    }).collect();

    Ok(LlmRequest {
        model: req.model,
        messages,
        stream: req.stream,
        max_tokens: req.max_tokens,
        temperature: req.temperature,
        top_p: req.top_p,
        tools,
    })
}

/// Convert llm_client::LlmResponse to bigmodel_api::ChatResponse for backwards compatibility
fn to_chat_response(resp: llm_client::LlmResponse) -> bigmodel_api::ChatResponse {
    // Extract extensions
    let extensions = resp.extensions;
    let web_search: Vec<bigmodel_api::WebSearchResult> = extensions.get("web_search")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let video_result: Vec<bigmodel_api::VideoResult> = extensions.get("video_result")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let content_filter: Vec<serde_json::Value> = extensions.get("content_filter")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    bigmodel_api::ChatResponse {
        id: resp.id,
        request_id: resp.request_id,
        created: resp.created,
        model: resp.model,
        choices: vec![bigmodel_api::Choice {
            index: 0,
            message: bigmodel_api::Message {
                role: bigmodel_api::Role::Assistant,
                content: resp.output_text.into(),
                reasoning_content: None,
            },
            finish_reason: resp.finish_reason.unwrap_or_else(|| "stop".to_string()),
        }],
        usage: resp.usage.map(|u| bigmodel_api::Usage {
            prompt_tokens: to_i32_saturated(u.input_tokens),
            completion_tokens: to_i32_saturated(u.output_tokens),
            total_tokens: to_i32_saturated(u.total_tokens),
        }),
        web_search,
        video_result,
        content_filter,
    }
}

/// Convert llm_client::LlmChunk to bigmodel_api::ChatResponseChunk for backwards compatibility
fn to_chunk_response(chunk: llm_client::LlmChunk) -> bigmodel_api::ChatResponseChunk {
    // Convert generic tool calls to BigModel's DeltaToolCall
    let tool_calls = chunk.delta_tool_calls.map(|calls| {
        calls.into_iter().map(|tc| {
            bigmodel_api::DeltaToolCall {
                id: tc.call_id,
                type_field: Some("function".to_string()),
                function: Some(bigmodel_api::DeltaToolFunction {
                    name: tc.tool_name,
                    arguments: tc.arguments,
                }),
                index: None,
            }
        }).collect()
    });

    bigmodel_api::ChatResponseChunk {
        id: chunk.id,
        created: chunk.created,
        model: chunk.model,
        choices: vec![bigmodel_api::ChoiceChunk {
            index: 0,
            delta: bigmodel_api::Delta {
                role: None,
                content: chunk.delta_text,
                reasoning_content: chunk.delta_reasoning,
                tool_calls,
            },
            finish_reason: chunk.finish_reason,
        }],
        usage: chunk.usage.map(|u| bigmodel_api::Usage {
            prompt_tokens: to_i32_saturated(u.input_tokens),
            completion_tokens: to_i32_saturated(u.output_tokens),
            total_tokens: to_i32_saturated(u.total_tokens),
        }),
    }
}

fn to_i32_saturated(value: u64) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

async fn chat_completions(
    State(state): State<GatewayState>,
    Json(request): Json<bigmodel_api::ChatRequest>,
) -> Response {
    let llm_request = match to_llm_request(request) {
        Ok(req) => req,
        Err((status, message)) => {
            return (status, Json(serde_json::json!({ "error": message }))).into_response();
        }
    };

    if llm_request.stream {
        let upstream = match state.client.chat_stream(llm_request) {
            Ok(stream) => stream,
            Err(err) => return map_error(err).into_response(),
        };

        let stream = async_stream::stream! {
            let mut upstream = std::pin::pin!(upstream);
            while let Some(item) = upstream.next().await {
                match item {
                    Ok(chunk) => {
                        let chunk_resp = to_chunk_response(chunk);
                        match serde_json::to_string(&chunk_resp) {
                            Ok(json) => yield Ok::<Event, std::convert::Infallible>(Event::default().data(json)),
                            Err(err) => {
                                let payload = serde_json::json!({"error": format!("serialize chunk failed: {err}")}).to_string();
                                yield Ok(Event::default().event("error").data(payload));
                                break;
                            }
                        }
                    }
                    Err(err) => {
                        let payload = serde_json::json!({"error": err.to_string()}).to_string();
                        yield Ok(Event::default().event("error").data(payload));
                        break;
                    }
                }
            }

            yield Ok(Event::default().data("[DONE]"));
        };

        return Sse::new(stream)
            .keep_alive(
                KeepAlive::new()
                    .interval(Duration::from_secs(15))
                    .text("keep-alive"),
            )
            .into_response();
    }

    match state.client.chat(llm_request).await {
        Ok(response) => {
            let chat_response = to_chat_response(response);
            Json(chat_response).into_response()
        }
        Err(err) => map_error(err).into_response(),
    }
}

fn map_error(err: LlmError) -> (StatusCode, Json<serde_json::Value>) {
    let status = match &err {
        LlmError::RateLimit { .. } => StatusCode::TOO_MANY_REQUESTS,
        LlmError::ServerError { status, .. } => {
            StatusCode::from_u16(*status).unwrap_or(StatusCode::BAD_GATEWAY)
        }
        LlmError::NetworkError { .. } => StatusCode::BAD_GATEWAY,
        LlmError::Timeout | LlmError::StreamIdleTimeout => StatusCode::GATEWAY_TIMEOUT,
        LlmError::AuthError { .. } => StatusCode::UNAUTHORIZED,
        LlmError::InvalidRequest { .. } | LlmError::ContextOverflow { .. } => {
            StatusCode::BAD_REQUEST
        }
        LlmError::QuotaExceeded { .. } => StatusCode::PAYMENT_REQUIRED,
        LlmError::StreamError { .. } => StatusCode::BAD_GATEWAY,
        LlmError::ParseError { .. } => StatusCode::BAD_GATEWAY,
    };

    (
        status,
        Json(serde_json::json!({
            "error": err.to_string(),
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Method, Request, header};
    use bigmodel_api::models::{ChatRequest, Message};
    use http_body_util::BodyExt;
    use llm_provider::bigmodel::{BigModelAdapter, BigModelConfig};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tower::ServiceExt;
    use wiremock::matchers::{header as wm_header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn build_test_request(stream: bool) -> ChatRequest {
        let mut req = ChatRequest::new("glm-5", vec![Message::user("hello")]);
        req.stream = stream;
        req
    }

    fn build_test_client(base_url: String) -> llm_client::LlmClient {
        let cfg = BigModelConfig::new(base_url, "test-key", HashMap::new()).expect("valid cfg");
        llm_client::LlmClient::builder()
            .register_adapter(Arc::new(BigModelAdapter::new(cfg)))
            .default_adapter("bigmodel")
            .build()
            .expect("build client")
    }

    #[tokio::test]
    async fn non_stream_request_returns_json() {
        let mock_server = MockServer::start().await;
        let upstream_response = serde_json::json!({
            "id": "resp-1",
            "created": 1,
            "model": "glm-5",
            "choices": [
                {
                    "index": 0,
                    "message": {"role": "assistant", "content": "hi"},
                    "finish_reason": "stop"
                }
            ]
        });

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(wm_header("authorization", "Bearer test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(upstream_response))
            .mount(&mock_server)
            .await;

        let client = build_test_client(mock_server.uri());
        let state = GatewayState::new(client);

        let request_payload = serde_json::to_vec(&build_test_request(false)).unwrap();

        let response = app(state)
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/v1/chat/completions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(request_payload))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["id"], "resp-1");
    }

    #[tokio::test]
    async fn stream_request_returns_sse() {
        let mock_server = MockServer::start().await;
        let sse_body = concat!(
            "data: {\"id\":\"chunk-1\",\"created\":1,\"model\":\"glm-5\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hel\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"chunk-1\",\"created\":1,\"model\":\"glm-5\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"lo\"},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n"
        );

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_raw(sse_body, "text/event-stream"),
            )
            .mount(&mock_server)
            .await;

        let client = build_test_client(mock_server.uri());
        let state = GatewayState::new(client);

        let request_payload = serde_json::to_vec(&build_test_request(true)).unwrap();

        let response = app(state)
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/v1/chat/completions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(request_payload))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(content_type.starts_with("text/event-stream"));

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_text = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_text.contains("data: {\"id\":\"chunk-1\""));
        assert!(body_text.contains("data: [DONE]"));
    }

    /// Test that created, request_id, and finish_reason are preserved in non-streaming responses
    #[tokio::test]
    async fn non_stream_response_preserves_metadata() {
        let mock_server = MockServer::start().await;
        let upstream_response = serde_json::json!({
            "id": "resp-123",
            "request_id": "req-abc-456",
            "created": 1700000000,
            "model": "glm-5",
            "choices": [
                {
                    "index": 0,
                    "message": {"role": "assistant", "content": "Hello!"},
                    "finish_reason": "stop"
                }
            ],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        });

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(wm_header("authorization", "Bearer test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(upstream_response))
            .mount(&mock_server)
            .await;

        let client = build_test_client(mock_server.uri());
        let state = GatewayState::new(client);

        let request_payload = serde_json::to_vec(&build_test_request(false)).unwrap();

        let response = app(state)
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/v1/chat/completions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(request_payload))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        // Verify all metadata fields are preserved
        assert_eq!(json["id"], "resp-123");
        assert_eq!(json["request_id"], "req-abc-456");
        assert_eq!(json["created"], 1700000000);
        assert_eq!(json["choices"][0]["finish_reason"], "stop");
    }

    /// Test that non-text content types return 400 error
    #[tokio::test]
    async fn non_text_content_returns_400() {
        let mock_server = MockServer::start().await;
        let client = build_test_client(mock_server.uri());
        let state = GatewayState::new(client);

        // Create request with multimodal content (non-text)
        let mut req = ChatRequest::new("glm-5", vec![]);
        req.messages.push(bigmodel_api::Message {
            role: bigmodel_api::Role::User,
            content: bigmodel_api::Content::Multimodal(vec![
                bigmodel_api::ContentPart::ImageUrl {
                    image_url: bigmodel_api::ImageUrl {
                        url: "https://example.com/image.jpg".to_string(),
                    },
                },
            ]),
            reasoning_content: None,
        });

        let request_payload = serde_json::to_vec(&req).unwrap();

        let response = app(state)
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/v1/chat/completions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(request_payload))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        // Verify error message mentions unsupported content type
        let error_msg = json["error"].as_str().unwrap();
        assert!(error_msg.contains("Unsupported message content type"));
        assert!(error_msg.contains("text content"));
    }

    /// Test that non-function tools return 400 error
    #[tokio::test]
    async fn non_function_tool_returns_400() {
        let mock_server = MockServer::start().await;
        let client = build_test_client(mock_server.uri());
        let state = GatewayState::new(client);

        // Create request with a retrieval tool (non-function)
        let mut req = ChatRequest::new("glm-5", vec![Message::user("hello")]);
        req.tools = Some(vec![bigmodel_api::Tool::Retrieval(
            bigmodel_api::RetrievalTool {
                retrieval: bigmodel_api::RetrievalObject {
                    knowledge_id: "kb-123".to_string(),
                    prompt_template: None,
                },
            },
        )]);

        let request_payload = serde_json::to_vec(&req).unwrap();

        let response = app(state)
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/v1/chat/completions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(request_payload))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        // Verify error message mentions unsupported tool type
        let error_msg = json["error"].as_str().unwrap();
        assert!(error_msg.contains("Unsupported tool type"));
        assert!(error_msg.contains("function tools"));
    }

    #[test]
    fn non_stream_usage_tokens_clamp_to_i32_max() {
        let resp = llm_client::LlmResponse {
            id: "resp-overflow".to_string(),
            created: 1,
            model: "glm-5".to_string(),
            output_text: "ok".to_string(),
            finish_reason: Some("stop".to_string()),
            request_id: Some("req-1".to_string()),
            usage: Some(llm_client::LlmUsage {
                input_tokens: i32::MAX as u64 + 42,
                output_tokens: i32::MAX as u64 + 7,
                total_tokens: i32::MAX as u64 + 999,
            }),
            extensions: serde_json::json!({}),
        };

        let chat = to_chat_response(resp);
        let usage = chat.usage.expect("usage");
        assert_eq!(usage.prompt_tokens, i32::MAX);
        assert_eq!(usage.completion_tokens, i32::MAX);
        assert_eq!(usage.total_tokens, i32::MAX);
    }

    #[test]
    fn stream_usage_tokens_clamp_to_i32_max() {
        let chunk = llm_client::LlmChunk {
            id: "chunk-overflow".to_string(),
            created: 1,
            model: "glm-5".to_string(),
            delta_text: Some("x".to_string()),
            delta_reasoning: None,
            delta_tool_calls: None,
            finish_reason: None,
            usage: Some(llm_client::LlmUsage {
                input_tokens: i32::MAX as u64 + 42,
                output_tokens: i32::MAX as u64 + 7,
                total_tokens: i32::MAX as u64 + 999,
            }),
        };

        let out = to_chunk_response(chunk);
        let usage = out.usage.expect("usage");
        assert_eq!(usage.prompt_tokens, i32::MAX);
        assert_eq!(usage.completion_tokens, i32::MAX);
        assert_eq!(usage.total_tokens, i32::MAX);
    }
}
