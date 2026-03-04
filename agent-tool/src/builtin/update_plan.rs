use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::context::{ToolContext, ToolResult};
use crate::error::ToolError;
use crate::spec::ToolSpec;
use crate::trait_def::Tool;

pub struct UpdatePlanTool;

#[derive(Deserialize)]
struct UpdatePlanArgs {
    #[serde(default)]
    explanation: Option<String>,
    plan: Vec<PlanItem>,
}

#[derive(Deserialize)]
struct PlanItem {
    step: String,
    status: String,
}

#[async_trait]
impl Tool for UpdatePlanTool {
    fn name(&self) -> &str {
        "update_plan"
    }

    fn description(&self) -> &str {
        "Update and validate a step-by-step execution plan."
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "explanation": { "type": "string" },
                    "plan": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "step": { "type": "string" },
                                "status": { "type": "string", "enum": ["pending", "in_progress", "completed"] }
                            },
                            "required": ["step", "status"]
                        }
                    }
                },
                "required": ["plan"]
            }),
        }
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        args: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let payload: UpdatePlanArgs = serde_json::from_value(args)
            .map_err(|err| ToolError::InvalidArgs(format!("invalid update_plan args: {err}")))?;

        if payload.plan.is_empty() {
            return Err(ToolError::InvalidArgs("plan must contain at least one step".to_string()));
        }

        let mut in_progress_count = 0usize;
        let mut tasks = Vec::with_capacity(payload.plan.len());
        for (idx, item) in payload.plan.iter().enumerate() {
            let trimmed_step = item.step.trim();
            if trimmed_step.is_empty() {
                return Err(ToolError::InvalidArgs("step cannot be empty".to_string()));
            }
            let status = item.status.as_str();
            if !matches!(status, "pending" | "in_progress" | "completed") {
                return Err(ToolError::InvalidArgs(format!("invalid status: {}", item.status)));
            }
            if status == "in_progress" {
                in_progress_count += 1;
            }
            tasks.push(json!({
                "id": format!("task-{}", idx + 1),
                "title": trimmed_step,
                "status": status,
            }));
        }

        if in_progress_count > 1 {
            return Err(ToolError::InvalidArgs(
                "at most one step can be in_progress".to_string(),
            ));
        }

        let is_streaming = tasks.iter().any(|task| task["status"] != "completed");
        Ok(ToolResult::ok(json!({
            "plan": {
                "title": "Execution Plan",
                "description": payload.explanation,
                "tasks": tasks,
                "is_streaming": is_streaming
            }
        })))
    }
}
