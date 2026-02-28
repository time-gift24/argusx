use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::StreamExt;
use llm_client::{LlmClient, LlmError, LlmMessage, LlmRequest, LlmRole};

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
fn to_llm_request(req: bigmodel_api::ChatRequest) -> LlmRequest {
    let messages: Vec<LlmMessage> = req.messages.into_iter().map(|m| {
        let role = match m.role {
            bigmodel_api::Role::System => LlmRole::System,
            bigmodel_api::Role::User => LlmRole::User,
            bigmodel_api::Role::Assistant => LlmRole::Assistant,
            bigmodel_api::Role::Tool => LlmRole::Tool,
        };
        let content = match m.content {
            bigmodel_api::Content::Text(s) => s,
            _ => String::new(), // Handle other content types as empty for now
        };
        LlmMessage { role, content }
    }).collect();

    LlmRequest {
        model: req.model,
        messages,
        stream: req.stream,
        max_tokens: req.max_tokens,
        temperature: req.temperature,
        top_p: req.top_p,
    }
}

/// Convert llm_client::LlmResponse to bigmodel_api::ChatResponse for backwards compatibility
fn to_chat_response(resp: llm_client::LlmResponse) -> bigmodel_api::ChatResponse {
    bigmodel_api::ChatResponse {
        id: resp.id,
        request_id: None,
        created: 0,
        model: resp.model,
        choices: vec![bigmodel_api::Choice {
            index: 0,
            message: bigmodel_api::Message {
                role: bigmodel_api::Role::Assistant,
                content: resp.output_text.into(),
                reasoning_content: None,
            },
            finish_reason: "stop".to_string(),
        }],
        usage: resp.usage.map(|u| bigmodel_api::Usage {
            prompt_tokens: u.input_tokens as i32,
            completion_tokens: u.output_tokens as i32,
            total_tokens: u.total_tokens as i32,
        }),
        web_search: vec![],
        video_result: vec![],
        content_filter: vec![],
    }
}

/// Convert llm_client::LlmChunk to bigmodel_api::ChatResponseChunk for backwards compatibility
fn to_chunk_response(chunk: llm_client::LlmChunk) -> bigmodel_api::ChatResponseChunk {
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
                tool_calls: None,
            },
            finish_reason: chunk.finish_reason,
        }],
        usage: chunk.usage.map(|u| bigmodel_api::Usage {
            prompt_tokens: u.input_tokens as i32,
            completion_tokens: u.output_tokens as i32,
            total_tokens: u.total_tokens as i32,
        }),
    }
}

async fn chat_completions(
    State(state): State<GatewayState>,
    Json(request): Json<bigmodel_api::ChatRequest>,
) -> Response {
    let llm_request = to_llm_request(request);

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
    use tower::ServiceExt;
    use wiremock::matchers::{header as wm_header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn build_test_request(stream: bool) -> ChatRequest {
        let mut req = ChatRequest::new("glm-5", vec![Message::user("hello")]);
        req.stream = stream;
        req
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

        let client = llm_client::LlmClient::builder()
            .with_default_bigmodel(mock_server.uri(), "test-key")
            .unwrap()
            .build()
            .unwrap();
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

        let client = llm_client::LlmClient::builder()
            .with_default_bigmodel(mock_server.uri(), "test-key")
            .unwrap()
            .build()
            .unwrap();
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
}
