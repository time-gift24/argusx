use std::{path::PathBuf, sync::Arc};

use futures::StreamExt;
use turn::{LlmStepRequest, ModelRunner, TurnMessage};

#[tokio::test(flavor = "current_thread")]
async fn provider_model_runner_streams_replay_fixture() {
    let surface = desktop_lib::chat::build_agent_tool_surface(serde_json::json!({
        "builtins": ["read", "update_plan"]
    }))
    .unwrap();
    let runner = desktop_lib::chat::ProviderModelRunner::from_replay(
        "gpt-test",
        fixture_path("2026-03-07-sample-replay.sse"),
        &surface,
    )
    .unwrap();

    let request = LlmStepRequest {
        session_id: "session-1".into(),
        turn_id: "turn-1".into(),
        step_index: 0,
        messages: Arc::from([Arc::new(TurnMessage::User {
            content: "hello".into(),
        })]),
        system_prompt: Some("You are a planner.".into()),
        allow_tools: false,
    };

    let mut stream = runner.start(request).await.unwrap();

    assert!(stream.next().await.is_some());
}

#[test]
fn provider_model_runner_emits_only_agent_allowed_tool_definitions() {
    let surface = desktop_lib::chat::build_agent_tool_surface(serde_json::json!({
        "builtins": ["read", "update_plan"]
    }))
    .unwrap();
    let runner = desktop_lib::chat::ProviderModelRunner::from_replay(
        "gpt-test",
        fixture_path("2026-03-07-sample-replay.sse"),
        &surface,
    )
    .unwrap();
    let tool_names = runner.tool_names();

    assert!(tool_names.iter().any(|name| name == "read"));
    assert!(tool_names.iter().any(|name| name == "update_plan"));
    assert!(!tool_names.iter().any(|name| name == "dispatch_subagent"));
    assert!(!tool_names.iter().any(|name| name == "shell"));
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../provider/tests/fixtures")
        .join(name)
}
