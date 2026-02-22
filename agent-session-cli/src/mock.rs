use std::path::PathBuf;
use std::sync::Arc;

use agent_core::{AgentError, LanguageModel, ModelEventStream, ModelOutputEvent, ModelRequest};
use agent_session::SessionRuntime;
use agent_turn::effect::ToolExecutor;
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
        _call: agent_core::ToolCall,
        _epoch: u64,
    ) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({"result": "ok"}))
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
        };
        let mut stream = model.stream(req).await.expect("stream");
        let first = stream.next().await.expect("event").expect("ok");
        assert!(matches!(first, ModelOutputEvent::TextDelta { .. }));
    }
}
