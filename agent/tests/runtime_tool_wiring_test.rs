use std::sync::Arc;
use agent::AgentBuilder;
use agent_center::AgentCenter;
use tempfile::tempdir;
use futures::Stream;
use std::pin::Pin;

// Mock LanguageModel for testing
struct MockModel;

fn mock_stream() -> Pin<Box<dyn Stream<Item = Result<agent_core::ModelOutputEvent, agent_core::AgentError>> + Send>> {
    use futures::stream;
    use futures::StreamExt;
    stream::empty().boxed()
}

#[async_trait::async_trait]
impl agent_core::LanguageModel for MockModel {
    fn model_name(&self) -> &str {
        "mock-model"
    }

    async fn stream(
        &self,
        _request: agent_core::ModelRequest,
    ) -> Result<agent_core::ModelEventStream, agent_core::AgentError> {
        Ok(mock_stream())
    }
}

#[tokio::test]
async fn agent_builder_accepts_agent_center() -> anyhow::Result<()> {
    // Create agent center
    let temp = tempdir()?;
    let db_path = temp.path().join("test.db");
    let center = Arc::new(
        AgentCenter::builder()
            .db_path(db_path)
            .build()?
    );

    // Build agent with agent-center - should not fail
    let model = Arc::new(MockModel);
    let agent = AgentBuilder::new()
        .model(model)
        .agent_center(center.clone())
        .build()
        .await?;

    // Verify agent was created successfully
    // The tool registration happens internally during build()
    // We can verify center.list_tools() works as expected
    let tools = center.list_tools();
    assert!(tools.contains(&"spawn_agent".to_string()));
    assert!(tools.contains(&"wait".to_string()));
    assert!(tools.contains(&"close_agent".to_string()));

    // Agent should be usable
    let session_id = agent.create_session(None, Some("Test Session".to_string())).await?;
    assert!(!session_id.is_empty());

    Ok(())
}

#[tokio::test]
async fn agent_builder_works_without_agent_center() -> anyhow::Result<()> {
    // Build agent without agent-center - should work normally
    let model = Arc::new(MockModel);
    let agent = AgentBuilder::new()
        .model(model)
        .build()
        .await?;

    // Agent should still work
    let session_id = agent.create_session(None, Some("Test Session".to_string())).await?;
    assert!(!session_id.is_empty());

    Ok(())
}
