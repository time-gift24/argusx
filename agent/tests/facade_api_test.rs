use std::sync::Arc;

use agent::{AgentBuilder, AgentFacadeError, ChatTurnStatus};
use agent_core::{AgentError, LanguageModel, ModelEventStream, ModelOutputEvent, ModelRequest};
use async_trait::async_trait;
use futures::stream;
use tempfile::TempDir;

struct MockModel;

#[async_trait]
impl LanguageModel for MockModel {
    fn model_name(&self) -> &str {
        "mock"
    }

    async fn stream(&self, _request: ModelRequest) -> Result<ModelEventStream, AgentError> {
        Ok(Box::pin(stream::iter(vec![
            Ok(ModelOutputEvent::TextDelta {
                delta: "assistant says hi".to_string(),
            }),
            Ok(ModelOutputEvent::Completed { usage: None }),
        ])))
    }
}

#[tokio::test]
async fn builder_without_model_is_rejected() {
    let result = AgentBuilder::<MockModel>::new().build().await;
    assert!(matches!(result, Err(AgentFacadeError::InvalidInput { .. })));
}

#[tokio::test]
async fn builder_with_model_and_default_tools_builds() {
    let temp_dir = TempDir::new().expect("create tempdir");

    let result = AgentBuilder::new()
        .model(Arc::new(MockModel))
        .store_dir(temp_dir.path().to_path_buf())
        .build()
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn chat_roundtrip_after_creating_session() {
    let temp_dir = TempDir::new().expect("create tempdir");

    let agent = AgentBuilder::new()
        .model(Arc::new(MockModel))
        .store_dir(temp_dir.path().to_path_buf())
        .build()
        .await
        .expect("build agent");

    let session_id = agent
        .create_session(None, Some("Facade Test".to_string()))
        .await
        .expect("create session");

    let response = agent
        .chat(&session_id, "hello")
        .await
        .expect("chat should succeed");

    assert_eq!(response.status, ChatTurnStatus::Done);
    assert_eq!(response.final_message.as_deref(), Some("assistant says hi"));
}

#[tokio::test]
async fn inject_and_cancel_unknown_turn_are_invalid_input() {
    let temp_dir = TempDir::new().expect("create tempdir");

    let agent = AgentBuilder::new()
        .model(Arc::new(MockModel))
        .store_dir(temp_dir.path().to_path_buf())
        .build()
        .await
        .expect("build agent");

    let inject_result = agent
        .inject_input("missing-turn", agent_core::InputEnvelope::user_text("hi"))
        .await;
    assert!(matches!(
        inject_result,
        Err(AgentFacadeError::InvalidInput { .. })
    ));

    let cancel_result = agent.cancel_turn("missing-turn", None).await;
    assert!(matches!(
        cancel_result,
        Err(AgentFacadeError::InvalidInput { .. })
    ));
}
