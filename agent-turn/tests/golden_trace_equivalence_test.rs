use std::sync::Arc;

use agent_core::tools::{ToolCatalog, ToolExecutionContext, ToolExecutionError, ToolExecutor};
use agent_core::{
    AgentError, InputEnvelope, LanguageModel, ModelEventStream, ModelOutputEvent, ModelRequest,
    RunStreamEvent, Runtime, SessionMeta, ToolCall, ToolResult, TurnRequest, UiThreadEvent,
};
use agent_turn::{TurnEngineConfig, TurnRuntime};
use async_trait::async_trait;
use futures::{stream, StreamExt};

struct GoldenModel;

#[async_trait]
impl LanguageModel for GoldenModel {
    fn model_name(&self) -> &str {
        "golden-model"
    }

    async fn stream(&self, _request: ModelRequest) -> Result<ModelEventStream, AgentError> {
        Ok(Box::pin(stream::iter(vec![
            Ok(ModelOutputEvent::TextDelta {
                delta: "a".to_string(),
            }),
            Ok(ModelOutputEvent::TextDelta {
                delta: "b".to_string(),
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

#[tokio::test]
async fn golden_trace_core_flows_match_expected_contract() {
    let runtime = TurnRuntime::new(
        Arc::new(GoldenModel),
        Arc::new(NoopTools),
        TurnEngineConfig::default(),
    );
    let request = TurnRequest::new(
        SessionMeta::new("s1", "t-golden"),
        "provider",
        "model",
        InputEnvelope::user_text("hello"),
    );
    let streams = runtime.run_turn(request).await.expect("run turn");
    let mut run = streams.run;
    let mut ui = streams.ui;

    let mut run_tags = Vec::new();
    while let Some(event) = run.next().await {
        match event {
            RunStreamEvent::TurnStart { .. } => run_tags.push("turn_start"),
            RunStreamEvent::ModelCompleted { .. } => run_tags.push("model_completed"),
            RunStreamEvent::TurnDone { .. } => run_tags.push("turn_done"),
            _ => {}
        }
    }

    let mut ui_text = String::new();
    while let Some(event) = ui.next().await {
        if let UiThreadEvent::MessageDelta { delta, .. } = event {
            ui_text.push_str(&delta);
        }
    }

    assert_eq!(run_tags, vec!["turn_start", "model_completed", "turn_done"]);
    assert_eq!(ui_text, "ab");
}
