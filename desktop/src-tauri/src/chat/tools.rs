use std::sync::Arc;

use argus_core::Builtin;
use async_trait::async_trait;
use tool::{
    GlobTool, GrepTool, ReadTool, ToolContext, ToolError, ToolResult,
    scheduler::{BuiltinRegistration, EffectiveToolPolicy, ToolScheduler},
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
        self.scheduler.execute(call, ctx).await.map_err(map_tool_error)
    }
}

fn map_init_error(err: impl std::fmt::Display) -> TurnError {
    TurnError::Runtime(err.to_string())
}

fn map_tool_error(err: ToolError) -> TurnError {
    TurnError::Runtime(err.to_string())
}
