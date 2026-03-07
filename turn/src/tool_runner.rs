use argus_core::ToolCall;
use async_trait::async_trait;
use tool::{ToolContext, ToolResult};

use crate::TurnError;

#[async_trait]
pub trait ToolRunner: Send + Sync {
    async fn execute(&self, call: ToolCall, ctx: ToolContext) -> Result<ToolResult, TurnError>;
}
