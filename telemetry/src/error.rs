use thiserror::Error;

#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("telemetry validation error: {0}")]
    Validation(String),
}
