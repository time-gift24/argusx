use std::sync::Arc;

use agent_core::tools::{ToolExecutionContext, ToolExecutionError, ToolExecutionErrorKind, ToolExecutor};
use agent_core::{AgentError, LanguageModel, ModelEventStream, ModelRequest, ToolCall, ToolResult};
use agent_turn::{TurnEngineConfig, TurnRuntime};
use async_trait::async_trait;
use futures::stream;

struct DummyModel;

#[async_trait]
impl LanguageModel for DummyModel {
    fn model_name(&self) -> &str {
        "dummy"
    }

    async fn stream(&self, _request: ModelRequest) -> Result<ModelEventStream, AgentError> {
        Ok(Box::pin(stream::empty()))
    }
}

struct DummyTools;

#[async_trait]
impl ToolExecutor for DummyTools {
    async fn execute_tool(
        &self,
        call: ToolCall,
        _ctx: ToolExecutionContext,
    ) -> Result<ToolResult, ToolExecutionError> {
        Ok(ToolResult::ok(call.call_id, serde_json::json!({"ok": true})))
    }
}

#[test]
fn turn_runtime_accepts_core_tool_executor() {
    let _runtime = TurnRuntime::new(
        Arc::new(DummyModel),
        Arc::new(DummyTools),
        TurnEngineConfig::default(),
    );

    let _ = ToolExecutionError {
        kind: ToolExecutionErrorKind::Internal,
        message: "unused compile guard".to_string(),
        retry_after_ms: None,
    };
}
