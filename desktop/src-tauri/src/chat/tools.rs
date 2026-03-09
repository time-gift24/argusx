use std::sync::Arc;

use argus_core::Builtin;
use async_trait::async_trait;
use tool::{
    scheduler::{BuiltinRegistration, EffectiveToolPolicy, ToolScheduler},
    GlobTool, GrepTool, ReadTool, ToolContext, ToolError, ToolResult, UpdatePlanTool,
};
use turn::{ToolRunner, TurnError};

pub struct ScheduledToolRunner {
    scheduler: ToolScheduler,
}

impl ScheduledToolRunner {
    pub fn from_current_dir() -> Result<Self, TurnError> {
        let policy = EffectiveToolPolicy {
            allow_parallel: true,
            max_concurrency: 4,
        };

        let scheduler = ToolScheduler::new([
            BuiltinRegistration::new(
                Builtin::Read,
                Arc::new(ReadTool::from_current_dir().map_err(map_init_error)?),
                policy,
            ),
            BuiltinRegistration::new(
                Builtin::Glob,
                Arc::new(GlobTool::from_current_dir().map_err(map_init_error)?),
                policy,
            ),
            BuiltinRegistration::new(
                Builtin::Grep,
                Arc::new(GrepTool::from_current_dir().map_err(map_init_error)?),
                policy,
            ),
            BuiltinRegistration::new(Builtin::UpdatePlan, Arc::new(UpdatePlanTool), policy),
        ])
        .map_err(map_tool_error)?;

        Ok(Self { scheduler })
    }
}

#[async_trait]
impl ToolRunner for ScheduledToolRunner {
    async fn execute(
        &self,
        call: argus_core::ToolCall,
        ctx: ToolContext,
    ) -> Result<ToolResult, TurnError> {
        self.scheduler
            .execute(call, ctx)
            .await
            .map_err(map_tool_error)
    }
}

fn map_init_error(err: impl std::fmt::Display) -> TurnError {
    TurnError::Runtime(err.to_string())
}

fn map_tool_error(err: ToolError) -> TurnError {
    TurnError::Runtime(err.to_string())
}

#[cfg(test)]
mod tests {
    use tokio_util::sync::CancellationToken;

    use argus_core::{Builtin, BuiltinToolCall, ToolCall};

    use super::*;

    #[tokio::test]
    async fn scheduled_tool_runner_executes_update_plan() {
        let runner = ScheduledToolRunner::from_current_dir().unwrap();
        let result = runner
            .execute(
                ToolCall::Builtin(BuiltinToolCall {
                    sequence: 0,
                    call_id: "call-1".into(),
                    builtin: Builtin::UpdatePlan,
                    arguments_json: serde_json::json!({
                        "explanation": "Starting execution",
                        "plan": [
                            {
                                "step": "Write failing test",
                                "status": "in_progress"
                            },
                            {
                                "step": "Implement minimal fix",
                                "status": "pending"
                            }
                        ]
                    })
                    .to_string(),
                }),
                ToolContext::new("session-1", "turn-1", CancellationToken::new()),
            )
            .await
            .unwrap();

        assert_eq!(
            result.output["plan"]["tasks"][0]["title"],
            "Write failing test"
        );
        assert_eq!(result.output["plan"]["tasks"][0]["status"], "in_progress");
    }
}
