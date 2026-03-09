use agent::AgentToolSurface;
use async_trait::async_trait;
use tool::{
    ToolContext, ToolError, ToolResult,
    scheduler::ToolScheduler,
};
use turn::{ToolRunner, TurnError};

pub struct ScheduledToolRunner {
    scheduler: ToolScheduler,
}

impl ScheduledToolRunner {
    pub fn from_tool_surface(surface: &AgentToolSurface) -> Result<Self, TurnError> {
        let scheduler = ToolScheduler::new(
            surface
                .builtin_registrations_from_current_dir()
                .map_err(map_init_error)?,
        )
        .map_err(map_tool_error)?;

        Ok(Self { scheduler })
    }
}

pub fn build_agent_tool_surface(tool_policy_json: serde_json::Value) -> Result<AgentToolSurface, TurnError> {
    agent::build_agent_tool_surface(tool_policy_json).map_err(map_init_error)
}

#[async_trait]
impl ToolRunner for ScheduledToolRunner {
    async fn execute(
        &self,
        call: argus_core::ToolCall,
        ctx: ToolContext,
    ) -> Result<ToolResult, TurnError> {
        self.scheduler.execute(call, ctx).await.map_err(map_tool_error)
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

        assert_eq!(result.output["plan"]["tasks"][0]["title"], "Write failing test");
        assert_eq!(result.output["plan"]["tasks"][0]["status"], "in_progress");
    }
}
