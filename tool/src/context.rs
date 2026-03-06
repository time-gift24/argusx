use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContext {
    pub session_id: String,
    pub turn_id: String,
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
