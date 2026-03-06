use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct ToolContext {
    pub session_id: String,
    pub turn_id: String,
    pub cancel_token: CancellationToken,
}

impl ToolContext {
    pub fn new(
        session_id: impl Into<String>,
        turn_id: impl Into<String>,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            turn_id: turn_id.into(),
            cancel_token,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub output: serde_json::Value,
    pub is_error: bool,
}

impl ToolResult {
    pub fn ok(output: serde_json::Value) -> Self {
        Self {
            output,
            is_error: false,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            output: serde_json::json!({ "error": message.into() }),
            is_error: true,
        }
    }
}
