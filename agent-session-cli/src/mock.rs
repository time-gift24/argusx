use std::path::PathBuf;
use std::sync::Arc;

use agent_core::tools::{ToolCatalog, ToolExecutionContext, ToolExecutionError, ToolExecutor};
use agent_core::{AgentError, LanguageModel, ModelEventStream, ModelOutputEvent, ModelRequest};
use agent_session::SessionRuntime;
use async_trait::async_trait;
use futures::stream;

#[derive(Default)]
pub struct MockModel;

pub struct MockTools;

pub fn build_runtime(store_dir: PathBuf) -> SessionRuntime<MockModel, MockTools> {
    SessionRuntime::new(store_dir, Arc::new(MockModel), Arc::new(MockTools))
}

#[async_trait]
impl LanguageModel for MockModel {
    fn model_name(&self) -> &str {
        "mock"
    }

    async fn stream(&self, _request: ModelRequest) -> Result<ModelEventStream, AgentError> {
        Ok(Box::pin(stream::iter(vec![
            Ok(ModelOutputEvent::TextDelta {
                delta: "mock response".to_string(),
            }),
            Ok(ModelOutputEvent::Completed { usage: None }),
        ])))
    }
}

#[async_trait]
impl ToolExecutor for MockTools {
    async fn execute_tool(
        &self,
        call: agent_core::ToolCall,
        _ctx: ToolExecutionContext,
    ) -> Result<agent_core::ToolResult, ToolExecutionError> {
        Ok(agent_core::ToolResult::ok(
            call.call_id,
            serde_json::json!({"result": "ok"}),
        ))
    }
}

#[async_trait]
impl ToolCatalog for MockTools {
    async fn list_tools(&self) -> Vec<agent_core::tools::ToolSpec> {
        Vec::new()
    }

    async fn tool_spec(&self, _name: &str) -> Option<agent_core::tools::ToolSpec> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn mock_model_emits_completed_event() {
        let model = MockModel;
        let req = ModelRequest {
            epoch: 0,
            transcript: vec![],
            inputs: vec![],
            tools: vec![],
        };
        let mut stream = model.stream(req).await.expect("stream");
        let first = stream.next().await.expect("event").expect("ok");
        assert!(matches!(first, ModelOutputEvent::TextDelta { .. }));
    }
}
