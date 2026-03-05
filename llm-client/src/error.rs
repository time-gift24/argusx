use std::time::{Duration, SystemTime};
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
    ServerError { status: u16, message: String },

    #[error("network error: {message}")]
    NetworkError { message: String },

    #[error("request timeout")]
    Timeout,

    #[error("stream idle timeout")]
    StreamIdleTimeout,

    // Non-retryable errors
    #[error("authentication error: {message}")]
    AuthError { message: String },

    #[error("invalid request: {message}")]
    InvalidRequest { message: String },

    #[error("context window exceeded: {message}")]
    ContextOverflow { message: String },

    #[error("quota exceeded: {message}")]
    QuotaExceeded { message: String },

    // Stream errors
    #[error("stream error: {message}")]
    StreamError { message: String },

    #[error("parse error: {message}")]
    ParseError { message: String },
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
            408 => Self::Timeout,
            429 => Self::RateLimit {
                message: body,
                retry_after,
            },
            status if (400..=499).contains(&status) => Self::InvalidRequest {
                message: format!("HTTP {}: {}", status, body),
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
        if err.is_timeout() {
            Self::Timeout
        } else {
            Self::NetworkError {
                message: err.to_string(),
            }
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

    #[test]
    fn from_http_status_maps_unknown_4xx_to_invalid_request() {
        let headers = HeaderMap::new();
        let err = LlmError::from_http_status(404, "not found".to_string(), &headers);
        assert!(matches!(err, LlmError::InvalidRequest { .. }));
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
