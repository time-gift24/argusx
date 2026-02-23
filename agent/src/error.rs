use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentFacadeError {
    #[error("invalid input: {message}")]
    InvalidInput { message: String },

    #[error("busy: {message}")]
    Busy { message: String },

    #[error("transient: {message}")]
    Transient {
        message: String,
        retry_after_ms: Option<u64>,
    },

    #[error("execution failed: {message}")]
    Execution { message: String },

    #[error("internal error: {message}")]
    Internal { message: String },
}

impl AgentFacadeError {
    pub(crate) fn from_agent_error(error: agent_core::AgentError) -> Self {
        use agent_core::{AgentError, RuntimeError};

        match error {
            AgentError::Transient(e) => Self::Transient {
                message: e.to_string(),
                retry_after_ms: e.retry_after_ms(),
            },
            AgentError::Runtime(RuntimeError::TurnNotFound { turn_id }) => Self::InvalidInput {
                message: format!("turn not found: {turn_id}"),
            },
            AgentError::Runtime(RuntimeError::TurnAlreadyExists { turn_id }) => Self::Busy {
                message: format!("turn already exists: {turn_id}"),
            },
            AgentError::Runtime(e) => Self::Execution {
                message: e.to_string(),
            },
            AgentError::Tool { message }
            | AgentError::Model { message }
            | AgentError::Checkpoint { message } => Self::Execution { message },
            AgentError::Internal { message } => map_by_message(message),
        }
    }

    pub(crate) fn from_anyhow(error: anyhow::Error) -> Self {
        map_by_message(error.to_string())
    }
}

fn map_by_message(message: String) -> AgentFacadeError {
    let lower = message.to_lowercase();
    if lower.contains("busy") {
        return AgentFacadeError::Busy { message };
    }
    if lower.contains("not found") || lower.contains("invalid session") {
        return AgentFacadeError::InvalidInput { message };
    }
    AgentFacadeError::Internal { message }
}
