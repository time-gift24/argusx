use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use agent_tool::{Tool, ToolContext, ToolError, ToolSpec, ToolResult};

use crate::api::center::SpawnRequest;

/// Tool for spawning child agents
pub struct SpawnAgentTool {
    center: Arc<crate::AgentCenter>,
}

impl SpawnAgentTool {
    pub fn new(center: Arc<crate::AgentCenter>) -> Self {
        Self { center }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SpawnAgentInput {
    pub parent_thread_id: String,
    pub key: String,
    pub agent_name: String,
    pub initial_input: String,
}

#[async_trait]
impl Tool for SpawnAgentTool {
    fn name(&self) -> &str {
        "spawn_agent"
    }

    fn description(&self) -> &str {
        "Spawn a child agent with a given agent type and initial input. Returns the thread ID of the spawned agent."
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "parent_thread_id": {
                        "type": "string",
                        "description": "ID of the parent thread (use 'root' for top-level threads)"
                    },
                    "key": {
                        "type": "string",
                        "description": "Unique key for deduplication within parent scope"
                    },
                    "agent_name": {
                        "type": "string",
                        "description": "Name/type of agent to spawn"
                    },
                    "initial_input": {
                        "type": "string",
                        "description": "Initial input for the agent"
                    }
                },
                "required": ["parent_thread_id", "key", "agent_name", "initial_input"]
            }),
        }
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        args: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let input: SpawnAgentInput = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(format!("Invalid input: {}", e)))?;

        let request = SpawnRequest {
            parent_thread_id: input.parent_thread_id,
            key: input.key,
            agent_name: input.agent_name,
            initial_input: input.initial_input,
        };

        let response = self.center.spawn(request).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Spawn failed: {}", e)))?;

        Ok(ToolResult::ok(json!(response)))
    }
}
