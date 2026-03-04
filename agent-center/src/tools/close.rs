use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use agent_tool::{Tool, ToolContext, ToolError, ToolSpec, ToolResult};

use crate::api::center::CloseRequest;

/// Tool for closing agent threads
pub struct CloseAgentTool {
    center: Arc<crate::AgentCenter>,
}

impl CloseAgentTool {
    pub fn new(center: Arc<crate::AgentCenter>) -> Self {
        Self { center }
    }
}

#[derive(Serialize, Deserialize)]
pub struct CloseAgentInput {
    pub thread_id: String,
    #[serde(default)]
    pub force: bool,
}

#[async_trait]
impl Tool for CloseAgentTool {
    fn name(&self) -> &str {
        "close_agent"
    }

    fn description(&self) -> &str {
        "Close an agent thread and mark it as terminal. Returns final thread status."
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "thread_id": {
                        "type": "string",
                        "description": "ID of thread to close"
                    },
                    "force": {
                        "type": "boolean",
                        "description": "Force close even if thread is still active",
                        "default": false
                    }
                },
                "required": ["thread_id"]
            }),
        }
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        args: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let input: CloseAgentInput = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(format!("Invalid input: {}", e)))?;

        let request = CloseRequest {
            thread_id: input.thread_id,
            force: input.force,
        };

        let response = self.center.close(request).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Close failed: {}", e)))?;

        Ok(ToolResult::ok(json!(response)))
    }
}
