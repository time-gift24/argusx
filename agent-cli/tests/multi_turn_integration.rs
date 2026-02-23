use std::sync::Arc;
use async_trait::async_trait;
use futures::stream;
use tempfile::TempDir;

struct EchoModel;

#[async_trait]
impl agent_core::LanguageModel for EchoModel {
    fn model_name(&self) -> &str {
        "echo"
    }

    async fn stream(
        &self,
        request: agent_core::ModelRequest,
    ) -> Result<agent_core::ModelEventStream, agent_core::AgentError> {
        let delta = format!("history={}", request.transcript.len());
        Ok(Box::pin(stream::iter(vec![
            Ok(agent_core::ModelOutputEvent::TextDelta { delta }),
            Ok(agent_core::ModelOutputEvent::Completed { usage: None }),
        ])))
    }
}

#[tokio::test]
async fn same_session_supports_two_turns() {
    let temp = TempDir::new().unwrap();
    let agent = agent::AgentBuilder::new()
        .model(Arc::new(EchoModel))
        .store_dir(temp.path().to_path_buf())
        .build()
        .await
        .unwrap();

    let session_id = agent.create_session(None, Some("demo".into())).await.unwrap();
    let first = agent.chat(&session_id, "first").await.unwrap();
    let second = agent.chat(&session_id, "second").await.unwrap();

    // First turn: transcript has system + user message = 2 entries (count=3 includes role)
    // Second turn: transcript grows, verifying session continuity
    let first_count = first.final_message.as_deref().unwrap();
    let second_count = second.final_message.as_deref().unwrap();
    // Verify second turn has MORE history than first (proves session continuity)
    assert!(
        second_count > first_count,
        "second turn should have more history than first, got first={}, second={}",
        first_count, second_count
    );
}
