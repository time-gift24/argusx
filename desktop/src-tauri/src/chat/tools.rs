use std::sync::Arc;

use async_trait::async_trait;
use tool::{
    GlobTool, GrepTool, ReadTool, ToolContext, ToolResult,
    Tool,
    catalog::{BuiltinRegistration, EffectiveToolPolicy},
    scheduler::ToolScheduler,
};
use turn::{ToolRunner, TurnError};

pub struct DesktopTooling {
    pub runner: Arc<dyn ToolRunner>,
    pub specs: Vec<tool::ToolSpec>,
}

pub fn default_tooling() -> Result<DesktopTooling, TurnError> {
    let read = Arc::new(
        ReadTool::from_current_dir()
            .map_err(|error| TurnError::Runtime(format!("init read tool: {error}")))?,
    );
    let glob = Arc::new(
        GlobTool::from_current_dir()
            .map_err(|error| TurnError::Runtime(format!("init glob tool: {error}")))?,
    );
    let grep = Arc::new(
        GrepTool::from_current_dir()
            .map_err(|error| TurnError::Runtime(format!("init grep tool: {error}")))?,
    );

    let policy = EffectiveToolPolicy {
        allow_parallel: true,
        max_concurrency: 4,
    };
    let scheduler = ToolScheduler::new([
        BuiltinRegistration::new(argus_core::Builtin::Read, read.clone(), policy),
        BuiltinRegistration::new(argus_core::Builtin::Glob, glob.clone(), policy),
        BuiltinRegistration::new(argus_core::Builtin::Grep, grep.clone(), policy),
    ])
    .map_err(|error| TurnError::Runtime(format!("init tool scheduler: {error}")))?;

    Ok(DesktopTooling {
        runner: Arc::new(ScheduledToolRunner {
            scheduler: Arc::new(scheduler),
        }),
        specs: vec![read.spec(), glob.spec(), grep.spec()],
    })
}

struct ScheduledToolRunner {
    scheduler: Arc<ToolScheduler>,
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
            .map_err(|error| TurnError::Runtime(error.to_string()))
    }
}
