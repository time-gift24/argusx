use async_trait::async_trait;
use serde_json::json;

use crate::context::{ToolContext, ToolResult};
use crate::error::ToolError;
use crate::spec::ToolSpec;
use crate::trait_def::Tool;

pub struct ShellTool;

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute a shell command"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Command to execute"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        args: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let command = args["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("command is required".to_string()))?;

        let output = tokio::process::Command::new("sh")
            .args(["-c", command])
            .output()
            .await?;

        let result = json!({
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "exit_code": output.status.code(),
        });

        Ok(ToolResult::ok(result))
    }
}
