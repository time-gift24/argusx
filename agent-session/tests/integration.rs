use agent_session::{SessionRuntime, SessionFilter};
use agent_core::{InputEnvelope, ModelRequest, SessionMeta, TurnRequest};
use async_trait::async_trait;
use std::sync::Arc;

// Dummy implementations for testing
struct MockModel;
struct MockTools;

#[async_trait]
impl agent_core::LanguageModel for MockModel {
    fn model_name(&self) -> &str {
        "mock"
    }

    async fn stream(
        &self,
        _request: ModelRequest,
    ) -> Result<agent_core::ModelEventStream, agent_core::AgentError> {
        Ok(Box::pin(futures::stream::empty()))
    }
}

#[async_trait]
impl agent_turn::effect::ToolExecutor for MockTools {
    async fn execute_tool(
        &self,
        _call: agent_core::ToolCall,
        _epoch: u64,
    ) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({"result": "ok"}))
    }
}

#[tokio::test]
async fn test_full_session_lifecycle() {
    let temp_dir = tempfile::tempdir().unwrap();
    let runtime = SessionRuntime::new(
        temp_dir.path().to_path_buf(),
        Arc::new(MockModel),
        Arc::new(MockTools),
    );

    // Create session
    let session_id = runtime.create_session(None, None).await.unwrap();
    assert!(!session_id.is_empty());

    // List sessions
    let sessions = runtime.list_sessions(SessionFilter::default()).await.unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id, session_id);

    // Get session
    let session = runtime.get_session(&session_id).await.unwrap();
    assert!(session.is_some());
    assert_eq!(session.unwrap().session_id, session_id);

    // Delete session
    runtime.delete_session(&session_id).await.unwrap();

    let sessions = runtime.list_sessions(SessionFilter::default()).await.unwrap();
    assert_eq!(sessions.len(), 0);
}

#[tokio::test]
async fn test_session_persistence() {
    let temp_dir = tempfile::tempdir().unwrap();
    let runtime = SessionRuntime::new(
        temp_dir.path().to_path_buf(),
        Arc::new(MockModel),
        Arc::new(MockTools),
    );

    // Create session with title
    let session_id = runtime
        .create_session(Some("user1".into()), Some("My Session".into()))
        .await
        .unwrap();

    // Drop runtime (simulating restart)
    drop(runtime);

    // Create new runtime with same storage
    let runtime2 = SessionRuntime::new(
        temp_dir.path().to_path_buf(),
        Arc::new(MockModel),
        Arc::new(MockTools),
    );

    // Session should persist
    let session = runtime2.get_session(&session_id).await.unwrap();
    assert!(session.is_some());
    assert_eq!(session.unwrap().title, "My Session");
}
