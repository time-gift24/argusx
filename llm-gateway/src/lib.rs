use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use bigmodel_api::ChatRequest;
use futures::StreamExt;
use llm_client::LlmError;
use llm_client::providers::BigModelHttpClient;

#[derive(Clone)]
pub struct GatewayState {
    pub client: Arc<BigModelHttpClient>,
}

impl GatewayState {
    pub fn new(client: BigModelHttpClient) -> Self {
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

async fn chat_completions(
    State(state): State<GatewayState>,
    Json(request): Json<ChatRequest>,
) -> Response {
    if request.stream {
        let upstream = state.client.chat_stream(request);

        let stream = async_stream::stream! {
            let mut upstream = std::pin::pin!(upstream);
            while let Some(item) = upstream.next().await {
                match item {
                    Ok(chunk) => {
                        match serde_json::to_string(&chunk) {
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

    match state.client.chat(request).await {
        Ok(response) => Json(response).into_response(),
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
    use llm_client::providers::BigModelConfig;
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

        let state = GatewayState::new(BigModelHttpClient::new(BigModelConfig {
            base_url: mock_server.uri(),
            api_key: "test-key".to_string(),
        }));

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

        let state = GatewayState::new(BigModelHttpClient::new(BigModelConfig {
            base_url: mock_server.uri(),
            api_key: "test-key".to_string(),
        }));

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
