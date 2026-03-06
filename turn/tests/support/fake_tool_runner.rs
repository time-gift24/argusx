use async_trait::async_trait;
use argus_core::ToolCall;
use serde_json::json;
use tool::{ToolContext, ToolResult};
use turn::{ToolRunner, TurnError};

pub struct FakeToolRunner;

#[async_trait]
impl ToolRunner for FakeToolRunner {
    async fn execute(&self, _call: ToolCall, _ctx: ToolContext) -> Result<ToolResult, TurnError> {
        Ok(ToolResult::ok(json!({"ok": true})))
    }
}
