# LLM Client Robustness Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create a robust `llm-client` crate with retry, timeout, and SSE handling for BigModel API requests.

**Architecture:** New `llm-client` crate provides low-level HTTP transport with exponential backoff retry, detailed error types, and idle timeout SSE processing. Existing `bigmodel-api` keeps type definitions only. `agent-turn` adapter uses new client.

**Tech Stack:** Rust, reqwest, tokio, futures, async-stream, thiserror, eventsource-stream

---

## Task 1: Create llm-client crate skeleton

**Files:**
- Create: `llm-client/Cargo.toml`
- Create: `llm-client/src/lib.rs`
- Modify: `Cargo.toml:2` (add to workspace members)

**Step 1: Create Cargo.toml**

```toml
[package]
name = "llm-client"
version = "0.1.0"
edition = "2021"
description = "Robust LLM HTTP client with retry and SSE support"

[dependencies]
reqwest.workspace = true
bytes.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tokio.workspace = true
futures.workspace = true
async-stream.workspace = true
async-trait.workspace = true
tracing.workspace = true
bigmodel-api = { path = "../bigmodel-api" }

# For retry jitter
rand = "0.8"

# For SSE parsing
eventsource-stream = "0.2"
tokio-stream = "0.1"

[dev-dependencies]
tokio = { workspace = true, features = ["rt-multi-thread", "macros", "time", "sync"] }
wiremock = "0.6"
```

**Step 2: Create lib.rs skeleton**

```rust
//! Robust LLM HTTP client with retry and SSE support.
//!
//! This crate provides:
//! - Exponential backoff retry with jitter
//! - Detailed error types (retryable vs non-retryable)
//! - SSE streaming with idle timeout detection
//! - Configurable timeouts

pub mod config;
pub mod error;
pub mod retry;
pub mod sse;
pub mod providers;

pub use config::{RetryPolicy, RetryOn, TimeoutConfig};
pub use error::LlmError;
pub use retry::run_with_retry;
```

**Step 3: Add to workspace**

In `Cargo.toml`, add `"llm-client"` to workspace members:

```toml
members = ["llm-client", "agent-cli", "agent", ...]
```

**Step 4: Verify compilation**

Run: `cargo check -p llm-client`
Expected: errors about missing modules (expected)

**Step 5: Commit**

```bash
git add llm-client/Cargo.toml llm-client/src/lib.rs Cargo.toml
git commit -m "feat(llm-client): create crate skeleton"
```

---

## Task 2: Implement error types

**Files:**
- Create: `llm-client/src/error.rs`
- Create: `llm-client/src/error.rs` tests

**Step 1: Write the failing test**

```rust
// At bottom of llm-client/src/error.rs

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn rate_limit_is_retryable() {
        let err = LlmError::RateLimit {
            message: "too many requests".to_string(),
            retry_after: Some(Duration::from_secs(5)),
        };
        assert!(err.is_retryable());
        assert_eq!(err.retry_after(), Some(Duration::from_secs(5)));
    }

    #[test]
    fn auth_error_is_not_retryable() {
        let err = LlmError::AuthError {
            message: "invalid key".to_string(),
        };
        assert!(!err.is_retryable());
        assert_eq!(err.retry_after(), None);
    }

    #[test]
    fn server_error_is_retryable() {
        let err = LlmError::ServerError {
            status: 503,
            message: "unavailable".to_string(),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn context_overflow_is_not_retryable() {
        let err = LlmError::ContextOverflow {
            message: "prompt too long".to_string(),
        };
        assert!(!err.is_retryable());
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p llm-client --lib error::tests`
Expected: FAIL with "cannot find type `LlmError`"

**Step 3: Write implementation**

```rust
// llm-client/src/error.rs
use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during LLM API calls.
#[derive(Debug, Error)]
pub enum LlmError {
    // Retryable errors
    #[error("rate limit exceeded: {message}")]
    RateLimit {
        message: String,
        retry_after: Option<Duration>,
    },

    #[error("server error ({status}): {message}")]
    ServerError {
        status: u16,
        message: String,
    },

    #[error("network error: {message}")]
    NetworkError {
        message: String,
    },

    #[error("request timeout")]
    Timeout,

    #[error("stream idle timeout")]
    StreamIdleTimeout,

    // Non-retryable errors
    #[error("authentication error: {message}")]
    AuthError {
        message: String,
    },

    #[error("invalid request: {message}")]
    InvalidRequest {
        message: String,
    },

    #[error("context window exceeded: {message}")]
    ContextOverflow {
        message: String,
    },

    #[error("quota exceeded: {message}")]
    QuotaExceeded {
        message: String,
    },

    // Stream errors
    #[error("stream error: {message}")]
    StreamError {
        message: String,
    },

    #[error("parse error: {message}")]
    ParseError {
        message: String,
    },
}

impl LlmError {
    /// Returns true if the error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimit { .. }
                | Self::ServerError { .. }
                | Self::NetworkError { .. }
                | Self::Timeout
                | Self::StreamIdleTimeout
        )
    }

    /// Returns the retry-after duration if available.
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RateLimit { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Maps HTTP status code to appropriate error type.
    pub fn from_http_status(status: u16, body: String, headers: &reqwest::header::HeaderMap) -> Self {
        let retry_after = headers
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_secs);

        match status {
            400 => {
                // Check for specific error patterns
                if body.contains("context") || body.contains("token") || body.contains("length") {
                    Self::ContextOverflow { message: body }
                } else {
                    Self::InvalidRequest { message: body }
                }
            }
            401 | 403 => Self::AuthError { message: body },
            402 => Self::QuotaExceeded { message: body },
            429 => Self::RateLimit {
                message: body,
                retry_after,
            },
            500..=599 => Self::ServerError {
                status,
                message: body,
            },
            _ => Self::ServerError {
                status,
                message: format!("Unknown HTTP error: {}", body),
            },
        }
    }
}

impl From<reqwest::Error> for LlmError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::Timeout
        } else if err.is_connect() {
            Self::NetworkError {
                message: err.to_string(),
            }
        } else {
            Self::NetworkError {
                message: err.to_string(),
            }
        }
    }
}

// Tests go here (from Step 1)
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p llm-client --lib error::tests`
Expected: 4 tests PASS

**Step 5: Commit**

```bash
git add llm-client/src/error.rs
git commit -m "feat(llm-client): add LlmError types with retryable classification"
```

---

## Task 3: Implement config types

**Files:**
- Create: `llm-client/src/config.rs`

**Step 1: Write the failing test**

```rust
// At bottom of llm-client/src/config.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_retry_policy() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.base_delay, Duration::from_secs(2));
        assert_eq!(policy.max_delay, Duration::from_secs(30));
        assert!(policy.retry_on.retry_429);
        assert!(policy.retry_on.retry_5xx);
        assert!(policy.retry_on.retry_network);
    }

    #[test]
    fn default_timeout_config() {
        let config = TimeoutConfig::default();
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.request_timeout, Duration::from_secs(120));
        assert_eq!(config.stream_idle_timeout, Duration::from_secs(30));
    }

    #[test]
    fn retry_policy_builder() {
        let policy = RetryPolicy::default()
            .max_attempts(5)
            .base_delay(Duration::from_secs(1));

        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.base_delay, Duration::from_secs(1));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p llm-client --lib config::tests`
Expected: FAIL with "cannot find type `RetryPolicy`"

**Step 3: Write implementation**

```rust
// llm-client/src/config.rs
use std::time::Duration;

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts.
    pub max_attempts: u64,
    /// Base delay for exponential backoff.
    pub base_delay: Duration,
    /// Maximum delay cap.
    pub max_delay: Duration,
    /// Conditions under which to retry.
    pub retry_on: RetryOn,
}

/// Conditions for retrying requests.
#[derive(Debug, Clone, Copy)]
pub struct RetryOn {
    /// Retry on 429 (rate limit) errors.
    pub retry_429: bool,
    /// Retry on 5xx server errors.
    pub retry_5xx: bool,
    /// Retry on network errors.
    pub retry_network: bool,
}

/// Configuration for timeouts.
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Timeout for establishing connection.
    pub connect_timeout: Duration,
    /// Timeout for complete request (non-streaming).
    pub request_timeout: Duration,
    /// Timeout for no data received on stream.
    pub stream_idle_timeout: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_secs(2),
            max_delay: Duration::from_secs(30),
            retry_on: RetryOn {
                retry_429: true,
                retry_5xx: true,
                retry_network: true,
            },
        }
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(120),
            stream_idle_timeout: Duration::from_secs(30),
        }
    }
}

impl RetryPolicy {
    /// Set maximum retry attempts.
    pub fn max_attempts(mut self, n: u64) -> Self {
        self.max_attempts = n;
        self
    }

    /// Set base delay for backoff.
    pub fn base_delay(mut self, d: Duration) -> Self {
        self.base_delay = d;
        self
    }

    /// Set maximum delay cap.
    pub fn max_delay(mut self, d: Duration) -> Self {
        self.max_delay = d;
        self
    }
}

impl Default for RetryOn {
    fn default() -> Self {
        Self {
            retry_429: true,
            retry_5xx: true,
            retry_network: true,
        }
    }
}

// Tests go here
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p llm-client --lib config::tests`
Expected: 3 tests PASS

**Step 5: Commit**

```bash
git add llm-client/src/config.rs
git commit -m "feat(llm-client): add RetryPolicy and TimeoutConfig"
```

---

## Task 4: Implement retry logic with backoff

**Files:**
- Create: `llm-client/src/retry.rs`

**Step 1: Write the failing test**

```rust
// At bottom of llm-client/src/retry.rs

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn backoff_increases_exponentially() {
        let base = Duration::from_secs(1);
        let max = Duration::from_secs(60);
        let err = LlmError::ServerError {
            status: 500,
            message: "test".to_string(),
        };

        // Attempt 1: 1 * 1 * jitter ≈ 1s
        let d1 = backoff(base, 1, &err, max);
        assert!(d1 >= Duration::from_millis(900));
        assert!(d1 <= Duration::from_millis(1100));

        // Attempt 2: 1 * 2 * jitter ≈ 2s
        let d2 = backoff(base, 2, &err, max);
        assert!(d2 >= Duration::from_millis(1800));
        assert!(d2 <= Duration::from_millis(2200));

        // Attempt 3: 1 * 4 * jitter ≈ 4s
        let d3 = backoff(base, 3, &err, max);
        assert!(d3 >= Duration::from_millis(3600));
        assert!(d3 <= Duration::from_millis(4400));
    }

    #[test]
    fn backoff_respects_retry_after() {
        let base = Duration::from_secs(1);
        let max = Duration::from_secs(60);
        let err = LlmError::RateLimit {
            message: "test".to_string(),
            retry_after: Some(Duration::from_secs(10)),
        };

        let d = backoff(base, 1, &err, max);
        assert_eq!(d, Duration::from_secs(10));
    }

    #[test]
    fn backoff_respects_max_delay() {
        let base = Duration::from_secs(10);
        let max = Duration::from_secs(30);
        let err = LlmError::ServerError {
            status: 500,
            message: "test".to_string(),
        };

        // 10 * 4 = 40, but capped at 30
        let d = backoff(base, 3, &err, max);
        assert!(d <= max);
    }

    #[tokio::test]
    async fn run_with_retry_succeeds_after_failures() {
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result = run_with_retry(RetryPolicy::default(), || {
            let attempts = attempts_clone.clone();
            async move {
                let n = attempts.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    // Fail first 2 attempts
                    Err(LlmError::ServerError {
                        status: 500,
                        message: "temporary".to_string(),
                    })
                } else {
                    Ok("success")
                }
            }
        })
        .await;

        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn run_with_retry_fails_after_max_attempts() {
        let policy = RetryPolicy::default().max_attempts(2);
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result = run_with_retry(policy, || {
            let attempts = attempts_clone.clone();
            async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err(LlmError::ServerError {
                    status: 500,
                    message: "always fails".to_string(),
                })
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn run_with_retry_fails_immediately_for_non_retryable() {
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result = run_with_retry(RetryPolicy::default(), || {
            let attempts = attempts_clone.clone();
            async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err(LlmError::AuthError {
                    message: "invalid key".to_string(),
                })
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 1); // Only one attempt
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p llm-client --lib retry::tests`
Expected: FAIL with "cannot find function `backoff`"

**Step 3: Write implementation**

```rust
// llm-client/src/retry.rs
use crate::config::RetryPolicy;
use crate::error::LlmError;
use rand::Rng;
use std::time::Duration;
use tracing::warn;

/// Run an async operation with retry logic.
pub async fn run_with_retry<T, F, Fut>(
    policy: RetryPolicy,
    mut make_request: impl FnMut() -> F,
) -> Result<T, LlmError>
where
    F: std::future::Future<Output = Result<T, LlmError>>,
{
    let mut attempt = 0;
    loop {
        attempt += 1;
        match make_request().await {
            Ok(result) => return Ok(result),
            Err(err) if should_retry(&err, &policy, attempt) => {
                let delay = backoff(policy.base_delay, attempt, &err, policy.max_delay);
                warn!(
                    attempt = attempt,
                    delay_ms = delay.as_millis(),
                    error = %err,
                    "Retrying request"
                );
                tokio::time::sleep(delay).await;
            }
            Err(err) => return Err(err),
        }
    }
}

fn should_retry(err: &LlmError, policy: &RetryPolicy, attempt: u64) -> bool {
    if attempt >= policy.max_attempts {
        return false;
    }

    if !err.is_retryable() {
        return false;
    }

    // Check specific retry conditions
    match err {
        LlmError::RateLimit { .. } => policy.retry_on.retry_429,
        LlmError::ServerError { status, .. } => {
            policy.retry_on.retry_5xx && *status >= 500 && *status < 600
        }
        LlmError::NetworkError { .. } | LlmError::Timeout | LlmError::StreamIdleTimeout => {
            policy.retry_on.retry_network
        }
        _ => false,
    }
}

/// Calculate backoff duration with exponential increase and jitter.
pub fn backoff(
    base: Duration,
    attempt: u64,
    err: &LlmError,
    max_delay: Duration,
) -> Duration {
    // Prefer retry-after header if available
    if let Some(retry_after) = err.retry_after() {
        return retry_after.min(max_delay);
    }

    // Exponential backoff: base * 2^(attempt-1) * jitter
    let exp = 2u64.saturating_pow(attempt.saturating_sub(1) as u32);
    let jitter = rand::thread_rng().gen_range(0.9..1.1);
    let delay = Duration::from_millis(
        (base.as_millis() as f64 * exp as f64 * jitter) as u64
    );

    delay.min(max_delay)
}

// Tests go here
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p llm-client --lib retry::tests`
Expected: 6 tests PASS

**Step 5: Commit**

```bash
git add llm-client/src/retry.rs
git commit -m "feat(llm-client): implement retry logic with exponential backoff"
```

---

## Task 5: Implement SSE processing with idle timeout

**Files:**
- Create: `llm-client/src/sse.rs`

**Step 1: Write the failing test**

```rust
// At bottom of llm-client/src/sse.rs

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream::{self, StreamExt};

    #[tokio::test]
    async fn parse_valid_sse_events() {
        let input = "data: {\"text\":\"hello\"}\n\ndata: [DONE]\n\n";
        let bytes_stream = stream::iter(vec![Ok(bytes::Bytes::from(input))]);

        let events: Vec<_> = parse_sse_stream(bytes_stream.map(|b| b.unwrap()))
            .collect()
            .await;

        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], SseEvent::Data(ref s) if s == "{\"text\":\"hello\"}"));
        assert!(matches!(events[1], SseEvent::Done));
    }

    #[test]
    fn parse_single_line_event() {
        let line = "data: {\"content\":\"test\"}";
        let event = parse_sse_line(line);
        assert!(matches!(event, Some(SseEvent::Data(ref s)) if s == "{\"content\":\"test\"}"));
    }

    #[test]
    fn parse_done_event() {
        let line = "data: [DONE]";
        let event = parse_sse_line(line);
        assert!(matches!(event, Some(SseEvent::Done)));
    }

    #[test]
    fn ignore_non_data_lines() {
        assert!(parse_sse_line(": comment").is_none());
        assert!(parse_sse_line("").is_none());
        assert!(parse_sse_line("event: foo").is_none());
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p llm-client --lib sse::tests`
Expected: FAIL with "cannot find function `parse_sse_stream`"

**Step 3: Write implementation**

```rust
// llm-client/src/sse.rs
use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;
use std::time::Duration;
use tokio::time::timeout;
use tokio_stream::StreamExt;

/// SSE event types.
#[derive(Debug, Clone)]
pub enum SseEvent {
    /// Data event with JSON payload.
    Data(String),
    /// Stream completed.
    Done,
    /// Error event.
    Error(String),
}

/// Parse a single SSE line into an event.
pub fn parse_sse_line(line: &str) -> Option<SseEvent> {
    let line = line.trim();

    // Skip empty lines and comments
    if line.is_empty() || line.starts_with(':') {
        return None;
    }

    // Parse data lines
    if let Some(data) = line.strip_prefix("data: ") {
        if data == "[DONE]" {
            return Some(SseEvent::Done);
        }
        return Some(SseEvent::Data(data.to_string()));
    }

    // Skip other SSE fields (event:, id:, retry:)
    None
}

/// Parse a byte stream into SSE events.
pub fn parse_sse_stream<S>(
    stream: S,
) -> impl Stream<Item = SseEvent>
where
    S: Stream<Item = Bytes> + Unpin,
{
    use async_stream::stream;

    stream! {
        let mut buffer = String::new();
        let mut lines_stream = std::pin::pin!(stream);

        while let Some(bytes) = lines_stream.next().await {
            if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                buffer.push_str(&text);

                // Process complete lines
                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].to_string();
                    buffer = buffer[pos + 1..].to_string();

                    if let Some(event) = parse_sse_line(&line) {
                        yield event;
                    }
                }
            }
        }
    }
}

/// Wrap a stream with idle timeout detection.
pub fn with_idle_timeout<S>(
    stream: S,
    idle_timeout: Duration,
) -> impl Stream<Item = Result<SseEvent, crate::error::LlmError>>
where
    S: Stream<Item = SseEvent> + Unpin,
{
    use crate::error::LlmError;

    async_stream::try_stream! {
        let mut stream = std::pin::pin!(stream);

        loop {
            match timeout(idle_timeout, stream.next()).await {
                Ok(Some(event)) => yield event,
                Ok(None) => break, // Stream ended
                Err(_) => {
                    yield Err(LlmError::StreamIdleTimeout)?;
                }
            }
        }
    }
}

// Tests go here
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p llm-client --lib sse::tests`
Expected: 4 tests PASS

**Step 5: Commit**

```bash
git add llm-client/src/sse.rs
git commit -m "feat(llm-client): implement SSE parsing with idle timeout"
```

---

## Task 6: Implement BigModel HTTP client

**Files:**
- Create: `llm-client/src/providers/mod.rs`
- Create: `llm-client/src/providers/bigmodel.rs`

**Step 1: Write the failing test**

```rust
// At bottom of llm-client/src/providers/bigmodel.rs

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path, header};

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
        let client = BigModelHttpClient::new(config);

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

        assert!(matches!(result, Err(LlmError::AuthError { .. })));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p llm-client --lib providers::bigmodel::tests`
Expected: FAIL with "cannot find type `BigModelHttpClient`"

**Step 3: Create providers/mod.rs**

```rust
// llm-client/src/providers/mod.rs
pub mod bigmodel;

pub use bigmodel::{BigModelConfig, BigModelHttpClient};
```

**Step 4: Write implementation**

```rust
// llm-client/src/providers/bigmodel.rs
use crate::config::{RetryPolicy, TimeoutConfig};
use crate::error::LlmError;
use crate::retry::run_with_retry;
use crate::sse::{parse_sse_stream, SseEvent};
use bigmodel_api::{ChatRequest, ChatResponse, ChatResponseChunk, Message};
use bytes::Bytes;
use futures::Stream;
use reqwest::header::HeaderMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error};

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
        let client = Arc::clone(&Arc::new(self.http.clone()));
        let url = format!("{}/chat/completions", self.config.base_url);
        let api_key = self.config.api_key.clone();
        let retry = self.retry.clone();

        run_with_retry(retry, || {
            let client = client.clone();
            let url = url.clone();
            let api_key = api_key.clone();
            let request = request.clone();

            async move {
                let response = client
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
            }
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

            let byte_stream = response.bytes_stream();
            let sse_stream = parse_sse_stream(byte_stream);

            // Apply idle timeout
            let mut timed_stream = tokio_stream::StreamExt::timeout(sse_stream, idle_timeout);

            while let Some(result) = timed_stream.next().await {
                match result {
                    Ok(SseEvent::Data(json)) => {
                        match serde_json::from_str::<ChatResponseChunk>(&json) {
                            Ok(chunk) => yield chunk,
                            Err(e) => {
                                debug!(error = %e, json = %json, "Failed to parse SSE chunk");
                                // Continue on parse errors for resilience
                            }
                        }
                    }
                    Ok(SseEvent::Done) => break,
                    Ok(SseEvent::Error(msg)) => {
                        Err(LlmError::StreamError { message: msg })?;
                    }
                    Err(_) => {
                        Err(LlmError::StreamIdleTimeout)?;
                    }
                }
            }
        })
    }
}

// Tests go here
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p llm-client --lib providers::bigmodel::tests`
Expected: 3 tests PASS

**Step 6: Commit**

```bash
git add llm-client/src/providers/mod.rs llm-client/src/providers/bigmodel.rs
git commit -m "feat(llm-client): implement BigModelHttpClient with retry and streaming"
```

---

## Task 7: Update agent-turn adapter

**Files:**
- Modify: `agent-turn/Cargo.toml`
- Modify: `agent-turn/src/adapters/bigmodel.rs`

**Step 1: Update Cargo.toml**

Add `llm-client` dependency to `agent-turn/Cargo.toml`:

```toml
[dependencies]
# ... existing dependencies ...
llm-client = { path = "../llm-client" }
```

**Step 2: Update adapter to use llm-client**

Replace the adapter implementation in `agent-turn/src/adapters/bigmodel.rs`:

```rust
use std::sync::Arc;

use agent_core::{
    new_id, AgentError, InputEnvelope, InputPart, InputSource, LanguageModel, ModelEventStream,
    ModelOutputEvent, ModelRequest, NoteLevel, ToolCall, TranscriptItem, TransientError, Usage,
};
use async_trait::async_trait;
use bigmodel_api::{
    ChatRequest, ChatResponseChunk, Content, FunctionDefinition, FunctionTool, Message,
    Role, Tool as BigModelTool, Usage as BigModelUsage,
};
use futures::StreamExt;
use llm_client::providers::{BigModelConfig, BigModelHttpClient};
use llm_client::{LlmError, RetryPolicy, TimeoutConfig};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

// ... (keep existing config structs: BigModelAdapterConfig, etc.)

pub struct BigModelModelAdapter {
    client: Arc<BigModelHttpClient>,
    config: BigModelAdapterConfig,
}

impl BigModelModelAdapter {
    pub fn new(client: Arc<BigModelHttpClient>) -> Self {
        Self {
            client,
            config: BigModelAdapterConfig::default(),
        }
    }

    pub fn with_config(mut self, config: BigModelAdapterConfig) -> Self {
        self.config = config;
        self
    }
}

#[async_trait]
impl LanguageModel for BigModelModelAdapter {
    fn model_name(&self) -> &str {
        &self.config.model
    }

    async fn stream(&self, request: ModelRequest) -> Result<ModelEventStream, AgentError> {
        let request = convert_model_request(request, &self.config);
        let (tx, rx) = mpsc::unbounded_channel::<Result<ModelOutputEvent, AgentError>>();

        tokio::spawn(async move {
            let mut stream = self.client.chat_stream(request);
            let mut usage: Option<Usage> = None;

            while let Some(item) = stream.next().await {
                match item {
                    Ok(chunk) => {
                        usage = extract_usage_from_chunk(&chunk).or(usage);
                        emit_chunk(chunk, &tx);
                    }
                    Err(err) => {
                        let _ = tx.send(Err(map_llm_error(err)));
                        return;
                    }
                }
            }

            let _ = tx.send(Ok(ModelOutputEvent::Completed { usage }));
        });

        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }
}

fn map_llm_error(err: LlmError) -> AgentError {
    match err {
        LlmError::RateLimit { message, retry_after } => {
            AgentError::Transient(TransientError::RateLimit {
                message,
                retry_after_ms: retry_after.map(|d| d.as_millis() as u64),
            })
        }
        LlmError::NetworkError { message } => {
            AgentError::Transient(TransientError::Network {
                message,
                retry_after_ms: None,
            })
        }
        LlmError::ServerError { message, .. } => {
            AgentError::Transient(TransientError::ServiceUnavailable {
                message,
                retry_after_ms: None,
            })
        }
        LlmError::Timeout | LlmError::StreamIdleTimeout => {
            AgentError::Transient(TransientError::Network {
                message: "request timeout".to_string(),
                retry_after_ms: None,
            })
        }
        LlmError::AuthError { message } | LlmError::InvalidRequest { message } => {
            AgentError::Model { message }
        }
        LlmError::ContextOverflow { message } => AgentError::Model { message },
        LlmError::QuotaExceeded { message } => AgentError::Model { message },
        LlmError::StreamError { message } => AgentError::Model { message },
        LlmError::ParseError { message } => AgentError::Model { message },
    }
}

// ... (keep existing helper functions: emit_chunk, extract_usage_from_chunk, convert_model_request, etc.)
```

**Step 3: Run tests to verify**

Run: `cargo test -p agent-turn`
Expected: All existing tests PASS

**Step 4: Commit**

```bash
git add agent-turn/Cargo.toml agent-turn/src/adapters/bigmodel.rs
git commit -m "refactor(agent-turn): use llm-client for BigModel requests"
```

---

## Task 8: Clean up bigmodel-api

**Files:**
- Modify: `bigmodel-api/src/lib.rs`
- Delete: `bigmodel-api/src/client.rs`

**Step 1: Remove client module**

In `bigmodel-api/src/lib.rs`:

```rust
// BigModel API types crate

pub mod config;
pub mod error;
pub mod models;

pub use config::Config;
pub use error::{BigModelError, Result};
pub use models::*;
```

**Step 2: Delete client.rs**

```bash
rm bigmodel-api/src/client.rs
```

**Step 3: Update Cargo.toml if needed**

Remove any dependencies only used by client.rs (like reqwest if only used there).

**Step 4: Verify compilation**

Run: `cargo check -p bigmodel-api`
Expected: Success

**Step 5: Commit**

```bash
git add bigmodel-api/src/lib.rs
git rm bigmodel-api/src/client.rs
git commit -m "refactor(bigmodel-api): remove client, keep type definitions only"
```

---

## Task 9: Integration test

**Files:**
- Create: `llm-client/tests/integration_test.rs`

**Step 1: Write integration test**

```rust
// llm-client/tests/integration_test.rs
use llm_client::providers::{BigModelConfig, BigModelHttpClient};
use llm_client::{RetryPolicy, TimeoutConfig};
use bigmodel_api::{ChatRequest, Message};
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::method;

#[tokio::test]
async fn full_flow_with_retries() {
    let mock_server = MockServer::start().await;

    // First two requests fail with 500
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    // Third succeeds
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "test",
            "created": 0,
            "model": "glm-5",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "OK"},
                "finish_reason": "stop"
            }]
        })))
        .mount(&mock_server)
        .await;

    let config = BigModelConfig {
        base_url: mock_server.uri(),
        api_key: "test".to_string(),
    };

    let retry = RetryPolicy::default()
        .max_attempts(3)
        .base_delay(std::time::Duration::from_millis(10));

    let client = BigModelHttpClient::with_options(config, retry, TimeoutConfig::default());

    let request = ChatRequest::new("glm-5", vec![Message::user("hi")]);
    let result = client.chat(request).await;

    assert!(result.is_ok());
}
```

**Step 2: Run test**

Run: `cargo test -p llm-client --test integration_test`
Expected: PASS

**Step 3: Commit**

```bash
git add llm-client/tests/integration_test.rs
git commit -m "test(llm-client): add integration test for retry flow"
```

---

## Task 10: Final verification and documentation

**Step 1: Run all tests**

Run: `cargo test --workspace`
Expected: All tests PASS

**Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: No warnings

**Step 3: Update crate documentation**

Add examples to `llm-client/src/lib.rs`:

```rust
//! # Example
//!
//! ```no_run
//! use llm_client::providers::{BigModelConfig, BigModelHttpClient};
//! use llm_client::{RetryPolicy, TimeoutConfig};
//! use bigmodel_api::{ChatRequest, Message};
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = BigModelConfig {
//!         api_key: "your-api-key".to_string(),
//!         ..Default::default()
//!     };
//!
//!     let client = BigModelHttpClient::new(config);
//!
//!     let request = ChatRequest::new("glm-5", vec![
//!         Message::user("Hello!")
//!     ]);
//!
//!     let response = client.chat(request).await.unwrap();
//!     println!("{:?}", response);
//! }
//! ```

**Step 4: Final commit**

```bash
git add llm-client/src/lib.rs
git commit -m "docs(llm-client): add usage examples"
```

---

## Summary

| Task | Description |
|------|-------------|
| 1 | Create llm-client crate skeleton |
| 2 | Implement LlmError types |
| 3 | Implement RetryPolicy and TimeoutConfig |
| 4 | Implement retry logic with backoff |
| 5 | Implement SSE parsing with idle timeout |
| 6 | Implement BigModelHttpClient |
| 7 | Update agent-turn adapter |
| 8 | Clean up bigmodel-api |
| 9 | Integration test |
| 10 | Final verification |
