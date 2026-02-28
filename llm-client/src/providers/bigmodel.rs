use crate::config::{RetryPolicy, TimeoutConfig};
use crate::error::LlmError;
use crate::retry::run_with_retry;
use crate::sse::{parse_sse_stream_result, SseEvent};
use bigmodel_api::{ChatRequest, ChatResponse, ChatResponseChunk};
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tokio::time::timeout;

/// Configuration for BigModel API.
#[derive(Debug, Clone)]
pub struct BigModelConfig {
    /// Base URL for the API (e.g., "https://open.bigmodel.cn/api/paas/v4").
    pub base_url: String,
    /// API key for authentication.
    pub api_key: String,
}

impl Default for BigModelConfig {
    fn default() -> Self {
        Self {
            base_url: "https://open.bigmodel.cn/api/paas/v4".to_string(),
            api_key: String::new(),
        }
    }
}

/// HTTP client for BigModel API with retry and timeout support.
pub struct BigModelHttpClient {
    /// Client for non-streaming requests (with request timeout)
    http: reqwest::Client,
    /// Client for streaming requests (without request timeout, relies on idle timeout)
    http_stream: reqwest::Client,
    config: BigModelConfig,
    retry: RetryPolicy,
    timeout: TimeoutConfig,
}

impl BigModelHttpClient {
    /// Create a new BigModel client with default settings.
    pub fn new(config: BigModelConfig) -> Self {
        Self::with_options(config, RetryPolicy::default(), TimeoutConfig::default())
    }

    /// Create a new BigModel client with custom retry and timeout settings.
    pub fn with_options(
        config: BigModelConfig,
        retry: RetryPolicy,
        timeout: TimeoutConfig,
    ) -> Self {
        // Client for non-streaming requests - has request timeout
        let http = reqwest::Client::builder()
            .connect_timeout(timeout.connect_timeout)
            .timeout(timeout.request_timeout)
            .build()
            .expect("Failed to build HTTP client");

        // Client for streaming requests - no request timeout, uses idle timeout instead
        let http_stream = reqwest::Client::builder()
            .connect_timeout(timeout.connect_timeout)
            .build()
            .expect("Failed to build streaming HTTP client");

        Self {
            http,
            http_stream,
            config,
            retry,
            timeout,
        }
    }

    /// Send a non-streaming chat request.
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LlmError> {
        let http = self.http.clone();
        let url = format!("{}/chat/completions", self.config.base_url);
        let api_key = self.config.api_key.clone();
        let retry = self.retry.clone();

        run_with_retry(retry, || {
            let http = http.clone();
            let url = url.clone();
            let api_key = api_key.clone();
            let request = request.clone();

            Box::pin(async move {
                let response = http
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("Content-Type", "application/json")
                    .json(&request)
                    .send()
                    .await?;

                let status = response.status();
                if status.is_success() {
                    response.json().await.map_err(|e| LlmError::ParseError {
                        message: e.to_string(),
                    })
                } else {
                    let headers = response.headers().clone();
                    let body = response.text().await.unwrap_or_default();
                    Err(LlmError::from_http_status(status.as_u16(), body, &headers))
                }
            })
        })
        .await
    }

    /// Send a streaming chat request.
    pub fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatResponseChunk, LlmError>> + Send>> {
        let url = format!("{}/chat/completions", self.config.base_url);
        let api_key = self.config.api_key.clone();
        let http = self.http_stream.clone(); // Use streaming client without request timeout
        let retry = self.retry.clone();
        let idle_timeout = self.timeout.stream_idle_timeout;

        Box::pin(async_stream::try_stream! {
            // Retry only the request bootstrap phase (before stream consumption starts).
            let response = run_with_retry(retry, || {
                let http = http.clone();
                let url = url.clone();
                let api_key = api_key.clone();
                let request = request.clone();

                Box::pin(async move {
                    let response = http
                        .post(&url)
                        .header("Authorization", format!("Bearer {}", api_key))
                        .header("Content-Type", "application/json")
                        .header("Accept", "text/event-stream")
                        .json(&request)
                        .send()
                        .await?;

                    let status = response.status();
                    if status.is_success() {
                        Ok(response)
                    } else {
                        let headers = response.headers().clone();
                        let body = response.text().await.unwrap_or_default();
                        Err(LlmError::from_http_status(status.as_u16(), body, &headers))
                    }
                })
            })
            .await?;

            let byte_stream = response
                .bytes_stream()
                .map(|item| item.map_err(LlmError::from));
            let mut sse_events = std::pin::pin!(parse_sse_stream_result(byte_stream));

            loop {
                match timeout(idle_timeout, sse_events.next()).await {
                    Ok(Some(Ok(SseEvent::Data(json)))) => {
                        let chunk = parse_chunk(&json)?;
                        yield chunk;
                    }
                    Ok(Some(Ok(SseEvent::Done))) => return,
                    Ok(Some(Ok(SseEvent::Error(message)))) => {
                        Err(LlmError::StreamError { message })?;
                    }
                    Ok(Some(Err(err))) => {
                        Err(err)?;
                    }
                    Ok(None) => break,
                    Err(_) => Err(LlmError::StreamIdleTimeout)?,
                }
            }
        })
    }
}

fn parse_chunk(payload: &str) -> Result<ChatResponseChunk, LlmError> {
    serde_json::from_str(payload).map_err(|err| LlmError::ParseError {
        message: format!(
            "failed to parse SSE chunk: {}; payload={}",
            err,
            truncate_payload(payload, 200),
        ),
    })
}

fn truncate_payload(payload: &str, max_len: usize) -> String {
    let mut chars = payload.chars();
    let truncated: String = chars.by_ref().take(max_len).collect();
    if chars.next().is_some() {
        format!("{}...", truncated)
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bigmodel_api::{ChatRequest, Message};
    use futures::StreamExt;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn chat_sends_request_correctly() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("Authorization", "Bearer test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "test-id",
                "created": 1234567890,
                "model": "glm-5",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello!"
                    },
                    "finish_reason": "stop"
                }]
            })))
            .mount(&mock_server)
            .await;

        let config = BigModelConfig {
            base_url: mock_server.uri(),
            api_key: "test-key".to_string(),
        };
        let client = BigModelHttpClient::new(config);

        let request = ChatRequest::new("glm-5", vec![Message::user("Hi")]);
        let response = client.chat(request).await.unwrap();

        assert_eq!(response.id, "test-id");
    }

    #[tokio::test]
    async fn chat_retries_on_500() {
        let mock_server = MockServer::start().await;

        // First request: 500 error
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal error"))
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;

        // Second request: success
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "test-id",
                "created": 1234567890,
                "model": "glm-5",
                "choices": [{
                    "index": 0,
                    "message": { "role": "assistant", "content": "OK" },
                    "finish_reason": "stop"
                }]
            })))
            .mount(&mock_server)
            .await;

        let config = BigModelConfig {
            base_url: mock_server.uri(),
            api_key: "test-key".to_string(),
        };
        // Use fast retry for testing
        let retry = crate::config::RetryPolicy::default()
            .max_attempts(3)
            .base_delay(std::time::Duration::from_millis(10));
        let client = BigModelHttpClient::with_options(
            config,
            retry,
            crate::config::TimeoutConfig::default(),
        );

        let request = ChatRequest::new("glm-5", vec![Message::user("Hi")]);
        let response = client.chat(request).await.unwrap();

        assert_eq!(response.id, "test-id");
    }

    #[tokio::test]
    async fn chat_fails_on_auth_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&mock_server)
            .await;

        let config = BigModelConfig {
            base_url: mock_server.uri(),
            api_key: "bad-key".to_string(),
        };
        let client = BigModelHttpClient::new(config);

        let request = ChatRequest::new("glm-5", vec![Message::user("Hi")]);
        let result = client.chat(request).await;

        assert!(matches!(
            result,
            Err(crate::error::LlmError::AuthError { .. })
        ));
    }

    #[tokio::test]
    async fn chat_stream_returns_parse_error_for_invalid_chunk() {
        let mock_server = MockServer::start().await;
        let sse_body = "data: not-json\n\ndata: [DONE]\n\n";

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .append_header("content-type", "text/event-stream")
                    .set_body_string(sse_body),
            )
            .mount(&mock_server)
            .await;

        let client = BigModelHttpClient::new(BigModelConfig {
            base_url: mock_server.uri(),
            api_key: "test-key".to_string(),
        });

        let request = ChatRequest::new("glm-5", vec![Message::user("Hi")]).stream();
        let mut stream = client.chat_stream(request);
        let first = stream.next().await.expect("first stream item");

        assert!(matches!(first, Err(LlmError::ParseError { .. })));
    }

    #[tokio::test]
    async fn chat_stream_accepts_done_without_space() {
        let mock_server = MockServer::start().await;
        let sse_body = "data:[DONE]\n\n";

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .append_header("content-type", "text/event-stream")
                    .set_body_string(sse_body),
            )
            .mount(&mock_server)
            .await;

        let client = BigModelHttpClient::new(BigModelConfig {
            base_url: mock_server.uri(),
            api_key: "test-key".to_string(),
        });

        let request = ChatRequest::new("glm-5", vec![Message::user("Hi")]).stream();
        let mut stream = client.chat_stream(request);
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn chat_stream_retries_on_bootstrap_500() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(500).set_body_string("temporary error"))
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .append_header("content-type", "text/event-stream")
                    .set_body_string("data: [DONE]\n\n"),
            )
            .mount(&mock_server)
            .await;

        let retry = crate::config::RetryPolicy::default()
            .max_attempts(3)
            .base_delay(std::time::Duration::from_millis(10));
        let client = BigModelHttpClient::with_options(
            BigModelConfig {
                base_url: mock_server.uri(),
                api_key: "test-key".to_string(),
            },
            retry,
            crate::config::TimeoutConfig::default(),
        );

        let request = ChatRequest::new("glm-5", vec![Message::user("Hi")]).stream();
        let mut stream = client.chat_stream(request);
        assert!(stream.next().await.is_none());
    }
}
