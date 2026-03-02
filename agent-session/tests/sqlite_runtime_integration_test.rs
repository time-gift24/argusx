use std::sync::Arc;

use agent_core::{
    new_id, AgentError, InputEnvelope, ModelOutputEvent, ModelRequest, RunStreamEvent, Runtime,
    SessionMeta, TurnRequest,
};
use agent_session::{SessionRuntime, SqliteSessionStore};
use async_trait::async_trait;
use futures::{stream, StreamExt};

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
    ) -> Result<agent_core::ModelEventStream, AgentError> {
        Ok(Box::pin(stream::once(async {
            Ok(ModelOutputEvent::Completed { usage: None })
        })))
    }
}

#[async_trait]
impl agent_core::tools::ToolExecutor for MockTools {
    async fn execute_tool(
        &self,
        call: agent_core::ToolCall,
        _ctx: agent_core::tools::ToolExecutionContext,
    ) -> Result<agent_core::ToolResult, agent_core::tools::ToolExecutionError> {
        Ok(agent_core::ToolResult::ok(
            call.call_id,
            serde_json::json!({ "ok": true }),
        ))
    }
}

#[async_trait]
impl agent_core::tools::ToolCatalog for MockTools {
    async fn list_tools(&self) -> Vec<agent_core::tools::ToolSpec> {
        Vec::new()
    }

    async fn tool_spec(&self, _name: &str) -> Option<agent_core::tools::ToolSpec> {
        None
    }
}

#[tokio::test]
async fn sqlite_runtime_restores_session_after_restart() {
    let temp = tempfile::tempdir().expect("create tempdir");
    let db_path = temp.path().join("sessions.db");

    let store_a = Arc::new(SqliteSessionStore::new(db_path.clone()).expect("init sqlite store"));
    let runtime_a = SessionRuntime::with_store(store_a, Arc::new(MockModel), Arc::new(MockTools));

    let session_id = runtime_a
        .create_session(None, Some("Persisted session".to_string()))
        .await
        .expect("create session");

    let first_streams = runtime_a
        .run_turn(TurnRequest {
            meta: SessionMeta::new(session_id.clone(), new_id()),
            provider: "bigmodel".to_string(),
            model: "glm-5".to_string(),
            initial_input: InputEnvelope::user_text("hello"),
            transcript: Vec::new(),
        })
        .await
        .expect("run turn");
    let events: Vec<_> = first_streams.run.collect().await;
    assert!(events
        .iter()
        .any(|event| matches!(event, RunStreamEvent::TurnDone { .. })));

    drop(runtime_a);

    let store_b = Arc::new(SqliteSessionStore::new(db_path).expect("re-open sqlite store"));
    let runtime_b = SessionRuntime::with_store(store_b, Arc::new(MockModel), Arc::new(MockTools));

    let restored = runtime_b
        .get_session(&session_id)
        .await
        .expect("load persisted session");
    assert!(restored.is_some());
    assert_eq!(restored.expect("session exists").title, "Persisted session");

    let second_streams = runtime_b
        .run_turn(TurnRequest {
            meta: SessionMeta::new(session_id, new_id()),
            provider: "bigmodel".to_string(),
            model: "glm-5".to_string(),
            initial_input: InputEnvelope::user_text("again"),
            transcript: Vec::new(),
        })
        .await
        .expect("run second turn");
    let events: Vec<_> = second_streams.run.collect().await;
    assert!(events
        .iter()
        .any(|event| matches!(event, RunStreamEvent::TurnDone { .. })));
}
