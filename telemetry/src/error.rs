use thiserror::Error;

#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("telemetry validation error: {0}")]
    Validation(String),

    #[error("telemetry write error: {0}")]
    Write(String),

    #[error("telemetry serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("telemetry HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}
