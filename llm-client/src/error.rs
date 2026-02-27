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
