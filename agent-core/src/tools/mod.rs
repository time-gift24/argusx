use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{ToolCall, ToolResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolParallelMode {
    ParallelSafe,
    Exclusive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolRetryPolicy {
    pub max_retries: u32,
    pub backoff_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolExecutionPolicy {
    pub parallel_mode: ToolParallelMode,
    pub timeout_ms: Option<u64>,
    pub retry: Option<ToolRetryPolicy>,
}

impl Default for ToolExecutionPolicy {
    fn default() -> Self {
        Self {
            parallel_mode: ToolParallelMode::ParallelSafe,
            timeout_ms: None,
            retry: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub execution_policy: ToolExecutionPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolExecutionContext {
    pub session_id: String,
    pub turn_id: String,
    pub epoch: u64,
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolExecutionErrorKind {
    User,
    Runtime,
    Transient,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolExecutionError {
    pub kind: ToolExecutionErrorKind,
    pub message: String,
    pub retry_after_ms: Option<u64>,
}

#[async_trait]
pub trait ToolCatalog: Send + Sync {
    async fn list_tools(&self) -> Vec<ToolSpec>;
    async fn tool_spec(&self, name: &str) -> Option<ToolSpec>;
}

#[async_trait]
pub trait ToolExecutor: Send + Sync {
    async fn execute_tool(
        &self,
        call: ToolCall,
        ctx: ToolExecutionContext,
    ) -> Result<ToolResult, ToolExecutionError>;
}
