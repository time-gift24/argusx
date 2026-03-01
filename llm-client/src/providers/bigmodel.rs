use crate::config::{RetryPolicy, TimeoutConfig};
use crate::error::LlmError;
use crate::retry::run_with_retry;
use crate::sse::{parse_sse_stream_result, SseEvent};
use bigmodel_api::{ChatRequest, ChatResponse, ChatResponseChunk};
use futures::{Stream, StreamExt};
use std::collections::HashMap;
use std::pin::Pin;
use tokio::time::timeout;

/// Configuration for BigModel API.
#[derive(Debug, Clone)]
pub struct BigModelConfig {
    /// Base URL for the API (e.g., "https://open.bigmodel.cn/api/paas/v4").
    pub base_url: String,
    /// API key for authentication.
    pub api_key: String,
    /// Additional custom headers.
    pub headers: HashMap<String, String>,
}

impl Default for BigModelConfig {
    fn default() -> Self {
        Self {
            base_url: "https://open.bigmodel.cn/api/paas/v4".to_string(),
            api_key: String::new(),
            headers: HashMap::new(),
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
        let headers = self.config.headers.clone();
        let retry = self.retry.clone();

        run_with_retry(retry, || {
            let http = http.clone();
            let url = url.clone();
            let api_key = api_key.clone();
            let headers = headers.clone();
            let request = request.clone();

            Box::pin(async move {
                let response = http
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("Content-Type", "application/json")
                    .headers(to_header_map(&headers))
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
        let headers = self.config.headers.clone();
        let http = self.http_stream.clone(); // Use streaming client without request timeout
        let retry = self.retry.clone();
        let idle_timeout = self.timeout.stream_idle_timeout;

        Box::pin(async_stream::try_stream! {
            // Retry only the request bootstrap phase (before stream consumption starts).
            let response = run_with_retry(retry, || {
                let http = http.clone();
                let url = url.clone();
                let api_key = api_key.clone();
                let headers = headers.clone();
                let request = request.clone();

                Box::pin(async move {
                    let response = http
                        .post(&url)
                        .header("Authorization", format!("Bearer {}", api_key))
                        .header("Content-Type", "application/json")
                        .header("Accept", "text/event-stream")
                        .headers(to_header_map(&headers))
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

fn to_header_map(headers: &HashMap<String, String>) -> reqwest::header::HeaderMap {
    let mut map = reqwest::header::HeaderMap::new();
    for (key, value) in headers {
        if key.trim().is_empty() {
            continue;
        }
        let Ok(name) = reqwest::header::HeaderName::from_bytes(key.trim().as_bytes()) else {
            continue;
        };
        let Ok(val) = reqwest::header::HeaderValue::from_str(value) else {
            continue;
        };
        map.insert(name, val);
    }
    map
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

/// Internal BigModel adapter that implements the ProviderAdapter trait.
pub(crate) struct BigModelAdapter {
    client: BigModelHttpClient,
}

impl BigModelAdapter {
    pub(crate) fn new(config: BigModelConfig) -> Self {
        Self {
            client: BigModelHttpClient::new(config),
        }
    }

    pub(crate) fn with_options(
        config: BigModelConfig,
        retry: RetryPolicy,
        timeout: TimeoutConfig,
    ) -> Self {
        Self {
            client: BigModelHttpClient::with_options(config, retry, timeout),
        }
    }
}

#[async_trait::async_trait]
impl crate::ProviderAdapter for BigModelAdapter {
    fn id(&self) -> &str {
        "bigmodel"
    }

    async fn chat(&self, req: crate::LlmRequest) -> Result<crate::LlmResponse, LlmError> {
        let bigmodel_req = crate::mapping::bigmodel::to_bigmodel_request(&req);
        let response = self.client.chat(bigmodel_req).await?;
        Ok(crate::mapping::bigmodel::to_llm_response(&response))
    }

    fn chat_stream(&self, req: crate::LlmRequest) -> crate::LlmChunkStream {
        let bigmodel_req = crate::mapping::bigmodel::to_bigmodel_request(&req);
        // Convert to streaming request
        let stream_req = bigmodel_req.stream();

        let stream = self.client.chat_stream(stream_req);
        Box::pin(stream.map(|result| {
            result.map(crate::mapping::bigmodel::to_llm_chunk)
        }))
    }
}
