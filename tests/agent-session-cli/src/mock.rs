use std::path::PathBuf;
use std::sync::Arc;

use agent::AgentBuilder;
use agent_core::{AgentError, LanguageModel, ModelEventStream, ModelOutputEvent, ModelRequest};
use async_trait::async_trait;
use futures::stream;

#[derive(Default)]
pub struct MockModel;

pub async fn build_agent(store_dir: PathBuf) -> anyhow::Result<agent::Agent<MockModel>> {
    AgentBuilder::new()
        .model(Arc::new(MockModel))
        .store_dir(store_dir)
        .build()
        .await
        .map_err(anyhow::Error::from)
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

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn mock_model_emits_completed_event() {
        let model = MockModel;
        let req = ModelRequest {
            epoch: 0,
            provider: "bigmodel".to_string(),
            model: "glm-5".to_string(),
            transcript: vec![],
            inputs: vec![],
            tools: vec![],
        };
        let mut stream = model.stream(req).await.expect("stream");
        let first = stream.next().await.expect("event").expect("ok");
        assert!(matches!(first, ModelOutputEvent::TextDelta { .. }));
    }
}
