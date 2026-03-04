use async_trait::async_trait;
use serde::{Deserialize, Serialize};
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
    #[serde(default)]
    lifecycle_status: Option<String>,
    #[serde(default)]
    progress: Option<PlanProgress>,
    #[serde(default)]
    view: Option<PlanView>,
    #[serde(default)]
    queue: Option<PlanQueue>,
}

#[derive(Deserialize)]
struct PlanItem {
    step: String,
    status: String,
}

#[derive(Serialize, Deserialize)]
struct PlanProgress {
    #[serde(default)]
    completed: Option<usize>,
    #[serde(default)]
    total: Option<usize>,
    #[serde(default)]
    percentage: Option<f64>,
}

#[derive(Serialize, Deserialize)]
struct PlanView {
    #[serde(default)]
    mode: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct PlanQueue {
    #[serde(default)]
    todos: Vec<QueueTodo>,
}

#[derive(Serialize, Deserialize)]
struct QueueTodo {
    id: String,
    title: String,
    #[serde(default)]
    description: Option<String>,
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
                                "status": { "type": "string", "enum": ["pending", "in_progress", "blocked", "completed", "failed"] }
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

        // Validate queue todos if present
        if let Some(ref queue) = payload.queue {
            for todo in &queue.todos {
                let status = todo.status.as_str();
                if !matches!(status, "pending" | "in_progress" | "blocked" | "completed" | "failed") {
                    return Err(ToolError::InvalidArgs(format!("invalid queue todo status: {}", todo.status)));
                }
            }
        }

        let mut in_progress_count = 0usize;
        let mut tasks = Vec::with_capacity(payload.plan.len());
        for (idx, item) in payload.plan.iter().enumerate() {
            let trimmed_step = item.step.trim();
            if trimmed_step.is_empty() {
                return Err(ToolError::InvalidArgs("step cannot be empty".to_string()));
            }
            let status = item.status.as_str();
            if !matches!(status, "pending" | "in_progress" | "blocked" | "completed" | "failed") {
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

        // Infer lifecycle_status if not provided
        let lifecycle_status = payload.lifecycle_status.or_else(|| {
            let has_in_progress = tasks.iter().any(|t| t["status"] == "in_progress");
            let all_completed = tasks.iter().all(|t| t["status"] == "completed");
            let has_failed = tasks.iter().any(|t| t["status"] == "failed");

            if has_failed {
                Some("failed".to_string())
            } else if all_completed {
                Some("completed".to_string())
            } else if has_in_progress {
                Some("in_progress".to_string())
            } else {
                Some("pending".to_string())
            }
        });

        // Infer progress if not provided
        let progress = payload.progress.or_else(|| {
            let total = tasks.len();
            let completed = tasks.iter().filter(|t| t["status"] == "completed").count();
            let percentage = if total > 0 {
                (completed as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            Some(PlanProgress {
                completed: Some(completed),
                total: Some(total),
                percentage: Some(percentage),
            })
        });

        Ok(ToolResult::ok(json!({
            "plan": {
                "title": "Execution Plan",
                "description": payload.explanation,
                "tasks": tasks,
                "is_streaming": is_streaming,
                "lifecycle_status": lifecycle_status,
                "progress": progress,
                "view": payload.view,
                "queue": payload.queue
            }
        })))
    }
}
