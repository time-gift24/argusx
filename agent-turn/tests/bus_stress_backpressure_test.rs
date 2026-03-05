use std::sync::Arc;

use agent_core::tools::{ToolCatalog, ToolExecutionContext, ToolExecutionError, ToolExecutor};
use agent_core::{
    AgentError, InputEnvelope, LanguageModel, ModelEventStream, ModelOutputEvent, ModelRequest,
    RunStreamEvent, Runtime, SessionMeta, ToolCall, ToolResult, TurnRequest,
};
use agent_turn::{TurnEngineConfig, TurnRuntime};
use async_trait::async_trait;
use futures::{stream, StreamExt};

const CHUNK_COUNT: usize = 2000;

struct BurstTextModel;

#[async_trait]
impl LanguageModel for BurstTextModel {
    fn model_name(&self) -> &str {
        "burst-text-model"
    }

    async fn stream(&self, _request: ModelRequest) -> Result<ModelEventStream, AgentError> {
        let mut events = Vec::with_capacity(CHUNK_COUNT + 1);
        for _ in 0..CHUNK_COUNT {
            events.push(Ok(ModelOutputEvent::TextDelta {
                delta: "x".to_string(),
            }));
        }
        events.push(Ok(ModelOutputEvent::Completed { usage: None }));
        Ok(Box::pin(stream::iter(events)))
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

#[tokio::test]
async fn bus_handles_high_volume_tool_deltas_without_unbounded_growth() {
    let runtime = TurnRuntime::new(
        Arc::new(BurstTextModel),
        Arc::new(NoopTools),
        TurnEngineConfig::default(),
    );
    let request = TurnRequest::new(
        SessionMeta::new("s1", "t-stress"),
        "provider",
        "model",
        InputEnvelope::user_text("hello"),
    );
    let streams = runtime.run_turn(request).await.expect("run turn");
    let mut run = streams.run;
    let mut final_message = None;

    while let Some(event) = run.next().await {
        if let RunStreamEvent::TurnDone {
            final_message: message,
            ..
        } = event
        {
            final_message = message;
        }
    }

    assert_eq!(
        final_message.expect("final message").len(),
        CHUNK_COUNT,
        "runtime should consume every burst chunk"
    );
}

