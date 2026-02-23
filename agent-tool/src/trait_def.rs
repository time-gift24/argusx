use crate::context::{ToolContext, ToolResult};
use crate::error::ToolError;
use crate::spec::ToolSpec;
use async_trait::async_trait;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn spec(&self) -> ToolSpec;

    async fn execute(
        &self,
        ctx: ToolContext,
        args: serde_json::Value,
    ) -> Result<ToolResult, ToolError>;
}
