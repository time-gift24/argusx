use crate::config::{RetryPolicy, TimeoutConfig};
use crate::error::LlmError;
use crate::retry::run_with_retry;
use crate::sse::SseEvent;
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
    http: reqwest::Client,
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
        let http = reqwest::Client::builder()
            .connect_timeout(timeout.connect_timeout)
            .timeout(timeout.request_timeout)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http,
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
        let http = self.http.clone();
        let idle_timeout = self.timeout.stream_idle_timeout;

        Box::pin(async_stream::try_stream! {
            let response = http
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .header("Accept", "text/event-stream")
                .json(&request)
                .send()
                .await?;

            let status = response.status();
            if !status.is_success() {
                let headers = response.headers().clone();
                let body = response.text().await.unwrap_or_default();
                Err(LlmError::from_http_status(status.as_u16(), body, &headers))?;
                return;
            }

            // Process the byte stream with idle timeout
            let mut byte_stream = std::pin::pin!(response.bytes_stream());
            let mut buffer = String::new();

            loop {
                let next_bytes = timeout(idle_timeout, byte_stream.next()).await;

                match next_bytes {
                    Ok(Some(Ok(bytes))) => {
                        if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                            buffer.push_str(&text);

                            // Process complete lines
                            while let Some(pos) = buffer.find('\n') {
                                let line = buffer[..pos].to_string();
                                buffer = buffer[pos + 1..].to_string();

                                if let Some(event) = crate::sse::parse_sse_line(&line) {
                                    match event {
                                        SseEvent::Data(json) => {
                                            match serde_json::from_str::<ChatResponseChunk>(&json) {
                                                Ok(chunk) => yield chunk,
                                                Err(e) => {
                                                    // Return parse error instead of silently ignoring
                                                    Err(LlmError::ParseError {
                                                        message: format!("Failed to parse SSE chunk: {}", e),
                                                    })?;
                                                }
                                            }
                                        }
                                        SseEvent::Done => return,
                                        SseEvent::Error(msg) => {
                                            Err(LlmError::StreamError { message: msg })?;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(Some(Err(e))) => {
                        // Return network error instead of swallowing it
                        Err(LlmError::NetworkError {
                            message: format!("Stream read error: {}", e),
                        })?;
                    }
                    Ok(None) => break, // Stream ended
                    Err(_) => {
                        Err(LlmError::StreamIdleTimeout)?;
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bigmodel_api::{ChatRequest, Message};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{header, method, path};

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

        assert!(matches!(result, Err(crate::error::LlmError::AuthError { .. })));
    }
}
