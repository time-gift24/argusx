use std::time::{Duration, SystemTime};
use thiserror::Error;

/// Retry category used by the generic retry loop.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RetryClass {
    RateLimit,
    Server,
    Network,
}

/// Errors that can occur during LLM API calls.
#[derive(Debug, Error)]
pub enum LlmError {
    #[error("retryable error ({class:?}): {message}")]
    Retryable {
        class: RetryClass,
        message: String,
        retry_after: Option<Duration>,
    },

    #[error("invalid request: {message}")]
    InvalidRequest { message: String },

    #[error("provider error: {message}")]
    ProviderError { message: String },

    #[error("stream error: {message}")]
    StreamError { message: String },

    #[error("parse error: {message}")]
    ParseError { message: String },
}

impl LlmError {
    /// Returns true if the error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Retryable { .. })
    }

    /// Returns the retry-after duration if available.
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::Retryable { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Returns retry classification when the error is retryable.
    pub fn retry_class(&self) -> Option<RetryClass> {
        match self {
            Self::Retryable { class, .. } => Some(*class),
            _ => None,
        }
    }

    /// Maps HTTP status code to appropriate error type.
    pub fn from_http_status(
        status: u16,
        body: String,
        headers: &reqwest::header::HeaderMap,
    ) -> Self {
        // Parse retry-after header (supports both seconds and HTTP-date format)
        let retry_after = headers
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(parse_retry_after);

        match status {
            400 => Self::InvalidRequest { message: body },
            408 => Self::Retryable {
                class: RetryClass::Network,
                message: format!("HTTP {}: {}", status, body),
                retry_after: None,
            },
            429 => Self::Retryable {
                class: RetryClass::RateLimit,
                message: body,
                retry_after,
            },
            500..=599 => Self::Retryable {
                class: RetryClass::Server,
                message: format!("HTTP {}: {}", status, body),
                retry_after: None,
            },
            status if (400..=499).contains(&status) => Self::ProviderError {
                message: format!("HTTP {}: {}", status, body),
            },
            _ => Self::ProviderError {
                message: format!("HTTP {}: {}", status, body),
            },
        }
    }
}

fn parse_retry_after(value: &str) -> Option<Duration> {
    if let Ok(seconds) = value.trim().parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }

    let retry_at = httpdate::parse_http_date(value).ok()?;
    Some(
        retry_at
            .duration_since(SystemTime::now())
            .unwrap_or(Duration::ZERO),
    )
}

impl From<reqwest::Error> for LlmError {
    fn from(err: reqwest::Error) -> Self {
        Self::Retryable {
            class: RetryClass::Network,
            message: err.to_string(),
            retry_after: None,
        }
    }
}

impl From<crate::sse::Error> for LlmError {
    fn from(err: crate::sse::Error) -> Self {
        match err {
            crate::sse::Error::Utf8(inner) => Self::ParseError {
                message: format!("invalid UTF-8 in SSE stream: {}", inner),
            },
            crate::sse::Error::Parser(message) => Self::ParseError {
                message: format!("invalid SSE frame: {}", message),
            },
            crate::sse::Error::Transport(inner) => Self::from(inner),
            crate::sse::Error::InvalidContentType(content_type) => Self::StreamError {
                message: format!("invalid content-type for SSE: {:?}", content_type),
            },
            crate::sse::Error::InvalidStatusCode(status) if status.is_server_error() => {
                Self::Retryable {
                    class: RetryClass::Server,
                    message: format!("invalid status code for SSE: {}", status),
                    retry_after: None,
                }
            }
            crate::sse::Error::InvalidStatusCode(status) => Self::ProviderError {
                message: format!("invalid status code for SSE: {}", status),
            },
            crate::sse::Error::InvalidLastEventId(last_event_id) => Self::InvalidRequest {
                message: format!("invalid Last-Event-ID: {}", last_event_id),
            },
            crate::sse::Error::StreamEnded => Self::StreamError {
                message: "stream ended".to_string(),
            },
            crate::sse::Error::CannotCloneRequest(_) => Self::InvalidRequest {
                message: "expected a cloneable request".to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};
    use std::time::Duration;

    #[test]
    fn rate_limit_is_retryable() {
        let err = LlmError::Retryable {
            class: RetryClass::RateLimit,
            message: "too many requests".to_string(),
            retry_after: Some(Duration::from_secs(5)),
        };
        assert!(err.is_retryable());
        assert_eq!(err.retry_after(), Some(Duration::from_secs(5)));
        assert_eq!(err.retry_class(), Some(RetryClass::RateLimit));
    }

    #[test]
    fn provider_error_is_not_retryable() {
        let err = LlmError::ProviderError {
            message: "invalid key".to_string(),
        };
        assert!(!err.is_retryable());
        assert_eq!(err.retry_after(), None);
    }

    #[test]
    fn server_error_is_retryable() {
        let err = LlmError::Retryable {
            class: RetryClass::Server,
            message: "unavailable".to_string(),
            retry_after: None,
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn from_http_status_maps_unknown_4xx_to_invalid_request() {
        let headers = HeaderMap::new();
        let err = LlmError::from_http_status(404, "not found".to_string(), &headers);
        assert!(matches!(err, LlmError::ProviderError { .. }));
    }

    #[test]
    fn retry_after_delay_seconds_is_parsed() {
        let mut headers = HeaderMap::new();
        headers.insert(RETRY_AFTER, HeaderValue::from_static("7"));

        let err = LlmError::from_http_status(429, "rate limit".to_string(), &headers);
        assert_eq!(err.retry_after(), Some(Duration::from_secs(7)));
    }

    #[test]
    fn retry_after_http_date_is_parsed() {
        let mut headers = HeaderMap::new();
        let date = httpdate::fmt_http_date(SystemTime::now() + Duration::from_secs(60));
        let value = HeaderValue::from_str(&date).expect("valid retry-after value");
        headers.insert(RETRY_AFTER, value);

        let err = LlmError::from_http_status(429, "rate limit".to_string(), &headers);
        let delay = err.retry_after().expect("retry-after should be parsed");
        assert!(delay <= Duration::from_secs(60));
        assert!(delay > Duration::from_secs(45));
    }
}
