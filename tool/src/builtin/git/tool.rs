use async_trait::async_trait;
use std::path::PathBuf;

use crate::builtin::fs::error::FsError;
use crate::context::{ToolContext, ToolResult};
use crate::error::ToolError;
use crate::spec::ToolSpec;
use crate::trait_def::Tool;

use super::guard::GitGuard;
use super::ops;
use super::types::{self, GitArgs};

pub struct GitTool {
    guard: GitGuard,
}

impl GitTool {
    pub fn new(allowed_roots: Vec<PathBuf>) -> Result<Self, FsError> {
        let guard = GitGuard::new(allowed_roots)?;
        Ok(Self { guard })
    }

    pub fn from_current_dir() -> Result<Self, FsError> {
        Self::new(vec![
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        ])
    }
}

#[async_trait]
impl Tool for GitTool {
    fn name(&self) -> &str {
        "git"
    }

    fn description(&self) -> &str {
        "Git operations: status, diff, log, show, branch_list, remote_list, worktree_list, add, commit, branch_create, checkout, clone, fetch"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_schema: types::build_schema(),
        }
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        args: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let args: GitArgs =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        ops::execute(&self.guard, args)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))
    }
}
