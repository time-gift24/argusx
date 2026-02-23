use async_trait::async_trait;
use futures::stream;
use std::sync::Arc;
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

    let session_id = agent
        .create_session(None, Some("demo".into()))
        .await
        .unwrap();
    let first = agent.chat(&session_id, "first").await.unwrap();
    let second = agent.chat(&session_id, "second").await.unwrap();

    // Parse "history=N" format to get numeric transcript counts
    let first_msg = first.final_message.as_deref().unwrap();
    let second_msg = second.final_message.as_deref().unwrap();

    let first_count = first_msg
        .strip_prefix("history=")
        .and_then(|s| s.parse::<usize>().ok())
        .expect("first response should be in 'history=N' format");

    let second_count = second_msg
        .strip_prefix("history=")
        .and_then(|s| s.parse::<usize>().ok())
        .expect("second response should be in 'history=N' format");

    // Verify second turn has MORE history than first (proves session continuity)
    assert!(
        second_count > first_count,
        "second turn should have more history than first, got first={}, second={}",
        first_count,
        second_count
    );
}
