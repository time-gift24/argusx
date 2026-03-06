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
        let command = args
            .get("command")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("command is required".to_string()))?;

        let cwd = match args.get("cwd") {
            Some(value) => Some(
                value
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("cwd must be a string".to_string()))?,
            ),
            None => None,
        };

        let mut command_builder = tokio::process::Command::new("sh");
        command_builder.kill_on_drop(true);
        command_builder.args(["-c", command]);
        if let Some(cwd) = cwd {
            command_builder.current_dir(cwd);
        }

        let output = command_builder.output().await?;
        let is_error = !output.status.success();

        let result = json!({
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "exit_code": output.status.code(),
        });

        Ok(ToolResult {
            output: result,
            is_error,
        })
    }
}
