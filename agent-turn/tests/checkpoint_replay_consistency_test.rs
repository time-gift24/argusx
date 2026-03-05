use std::sync::Arc;

use agent_core::tools::{ToolCatalog, ToolExecutionContext, ToolExecutionError, ToolExecutor};
use agent_core::{
    AgentError, CheckpointStore, InputEnvelope, LanguageModel, ModelEventStream, ModelOutputEvent,
    ModelRequest, Runtime, SessionMeta, ToolCall, ToolResult, TranscriptItem, TurnRequest,
};
use agent_turn::{TurnEngineConfig, TurnRuntime};
use async_trait::async_trait;
use futures::{stream, StreamExt};
use tokio::sync::Mutex;

struct TextThenDoneModel;

#[async_trait]
impl LanguageModel for TextThenDoneModel {
    fn model_name(&self) -> &str {
        "text-then-done"
    }

    async fn stream(&self, _request: ModelRequest) -> Result<ModelEventStream, AgentError> {
        Ok(Box::pin(stream::iter(vec![
            Ok(ModelOutputEvent::TextDelta {
                delta: "hello".to_string(),
            }),
            Ok(ModelOutputEvent::Completed { usage: None }),
        ])))
    }
}

struct NoopTools;

#[async_trait]
impl ToolExecutor for NoopTools {
    async fn execute_tool(
        &self,
        call: ToolCall,
        _ctx: ToolExecutionContext,
    ) -> Result<ToolResult, ToolExecutionError> {
        Ok(ToolResult::ok(call.call_id, serde_json::json!({})))
    }
}

#[async_trait]
impl ToolCatalog for NoopTools {
    async fn list_tools(&self) -> Vec<agent_core::tools::ToolSpec> {
        Vec::new()
    }

    async fn tool_spec(&self, _name: &str) -> Option<agent_core::tools::ToolSpec> {
        None
    }
}

#[derive(Default)]
struct RecordingCheckpointStore {
    appended: Mutex<Vec<TranscriptItem>>,
    snapshots: Mutex<Vec<Vec<TranscriptItem>>>,
}

#[async_trait]
impl CheckpointStore for RecordingCheckpointStore {
    async fn append_items(
        &self,
        _turn_id: &str,
        items: &[TranscriptItem],
    ) -> Result<(), AgentError> {
        self.appended.lock().await.extend_from_slice(items);
        Ok(())
    }

    async fn load_items(&self, _turn_id: &str) -> Result<Vec<TranscriptItem>, AgentError> {
        Ok(self.appended.lock().await.clone())
    }

    async fn snapshot(&self, _turn_id: &str, items: &[TranscriptItem]) -> Result<(), AgentError> {
        self.snapshots.lock().await.push(items.to_vec());
        Ok(())
    }
}

#[tokio::test]
async fn checkpoint_replay_reconstructs_state_consistently() {
    let checkpoint = Arc::new(RecordingCheckpointStore::default());
    let runtime = TurnRuntime::new(
        Arc::new(TextThenDoneModel),
        Arc::new(NoopTools),
        TurnEngineConfig::default(),
    )
    .with_checkpoint_store(checkpoint.clone());

    let request = TurnRequest::new(
        SessionMeta::new("s1", "t-checkpoint"),
        "provider",
        "model",
        InputEnvelope::user_text("hello"),
    );
    let streams = runtime.run_turn(request).await.expect("run turn");

    let mut run = streams.run;
    while run.next().await.is_some() {}

    let appended = checkpoint.appended.lock().await.clone();
    let snapshots = checkpoint.snapshots.lock().await.clone();
    let latest_snapshot = snapshots.last().expect("snapshot exists");

    assert_eq!(
        appended, *latest_snapshot,
        "incremental checkpoint replay should match final snapshot transcript"
    );
}

