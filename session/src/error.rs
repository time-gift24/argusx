use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Database error: {0}")]
    Database(#[from] anyhow::Error),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Thread not found: {0}")]
    ThreadNotFound(Uuid),

    #[error("Turn already active in thread")]
    TurnAlreadyActive,

    #[error("No active turn in thread")]
    NoActiveTurn,

    #[error("Turn error: {0}")]
    Turn(#[from] turn::TurnError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type SessionResult<T> = Result<T, SessionError>;
