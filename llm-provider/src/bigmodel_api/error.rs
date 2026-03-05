use thiserror::Error;

#[derive(Error, Debug)]
pub enum BigModelError {
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitError(String),

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Parse error: {0}")]
    ParseError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, BigModelError>;
