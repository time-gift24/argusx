use std::{path::PathBuf, sync::Arc};

use argus_core::Builtin;
use async_trait::async_trait;
use tool::{
    builtin::browser::{
        BrowserTool,
        config::{BrowserConfig, BrowserConfigManager},
    },
    scheduler::{BuiltinRegistration, EffectiveToolPolicy, ToolScheduler},
    GlobTool, GrepTool, ReadTool, ToolContext, ToolError, ToolResult, UpdatePlanTool,
};
use turn::{ToolRunner, TurnError};

pub struct ScheduledToolRunner {
    scheduler: ToolScheduler,
}

impl ScheduledToolRunner {
    pub fn new(
        allowed_roots: Vec<PathBuf>,
        browser_config_db_path: PathBuf,
    ) -> Result<Self, TurnError> {
        let read_tool = Arc::new(ReadTool::new(allowed_roots.clone()).map_err(map_init_error)?);
        let glob_tool = Arc::new(GlobTool::new(allowed_roots.clone()).map_err(map_init_error)?);
        let grep_tool = Arc::new(GrepTool::new(allowed_roots).map_err(map_init_error)?);
        let browser_tool = Arc::new(BrowserTool::new(
            BrowserConfig::default(),
            BrowserConfigManager::new(browser_config_db_path).map_err(map_init_error)?,
        ));
        let policy = EffectiveToolPolicy {
            allow_parallel: true,
            max_concurrency: 4,
        };

        let scheduler = ToolScheduler::new([
            BuiltinRegistration::new(Builtin::Read, read_tool, policy),
            BuiltinRegistration::new(Builtin::Glob, glob_tool, policy),
            BuiltinRegistration::new(Builtin::Grep, grep_tool, policy),
            BuiltinRegistration::new(Builtin::Browser, browser_tool, policy),
            BuiltinRegistration::new(Builtin::UpdatePlan, Arc::new(UpdatePlanTool), policy),
        ])
        .map_err(map_tool_error)?;

        Ok(Self { scheduler })
    }

    pub fn from_current_dir() -> Result<Self, TurnError> {
        Self::new(
            vec![std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))],
            default_browser_config_db_path(),
        )
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

fn default_browser_config_db_path() -> PathBuf {
    std::env::temp_dir().join("argusx-browser.sqlite3")
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

    #[tokio::test]
    async fn scheduled_tool_runner_executes_browser_get_config() {
        let runner = ScheduledToolRunner::from_current_dir().unwrap();
        let result = runner
            .execute(
                ToolCall::Builtin(BuiltinToolCall {
                    sequence: 0,
                    call_id: "call-browser".into(),
                    builtin: Builtin::Browser,
                    arguments_json: serde_json::json!({
                        "action": "get_config",
                    })
                    .to_string(),
                }),
                ToolContext::new("session-1", "turn-1", CancellationToken::new()),
            )
            .await
            .unwrap();

        assert_eq!(result.output["config"]["headless"], serde_json::json!(false));
        assert_eq!(result.output["config"]["is_enabled"], serde_json::json!(false));
    }
}
