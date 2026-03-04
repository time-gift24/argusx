use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use agent_tool::{Tool, ToolContext, ToolError, ToolSpec, ToolResult};

use crate::api::center::{WaitRequest, WaitMode};

/// Tool for waiting on agent threads
pub struct WaitTool {
    center: Arc<crate::AgentCenter>,
}

impl WaitTool {
    pub fn new(center: Arc<crate::AgentCenter>) -> Self {
        Self { center }
    }
}

#[derive(Serialize, Deserialize)]
pub struct WaitInput {
    pub thread_ids: Vec<String>,
    pub mode: String,
    pub timeout_ms: u64,
}

#[async_trait]
impl Tool for WaitTool {
    fn name(&self) -> &str {
        "wait"
    }

    fn description(&self) -> &str {
        "Wait for agent threads to reach terminal state. Returns status map of all threads."
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "thread_ids": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "List of thread IDs to wait for"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["any", "all"],
                        "description": "Wait mode: 'any' (return when any thread terminal) or 'all' (wait for all)"
                    },
                    "timeout_ms": {
                        "type": "integer",
                        "description": "Timeout in milliseconds (clamped to [1000, 300000])"
                    }
                },
                "required": ["thread_ids", "mode", "timeout_ms"]
            }),
        }
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        args: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let input: WaitInput = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(format!("Invalid input: {}", e)))?;

        let mode = match input.mode.to_lowercase().as_str() {
            "any" => WaitMode::Any,
            "all" => WaitMode::All,
            _ => return Err(ToolError::InvalidArgs("mode must be 'any' or 'all'".to_string())),
        };

        let request = WaitRequest {
            thread_ids: input.thread_ids,
            mode,
            timeout_ms: input.timeout_ms,
        };

        let response = self.center.wait(request).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Wait failed: {}", e)))?;

        Ok(ToolResult::ok(json!(response)))
    }
}
