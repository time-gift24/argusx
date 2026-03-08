use std::{path::PathBuf, sync::Arc};

use futures::StreamExt;
use turn::{LlmStepRequest, ModelRunner, TurnMessage};

#[tokio::test(flavor = "current_thread")]
async fn provider_model_runner_streams_replay_fixture() {
    let runner = desktop_lib::chat::ProviderModelRunner::from_replay(
        "gpt-test",
        fixture_path("2026-03-07-sample-replay.sse"),
    )
    .unwrap();

    let request = LlmStepRequest {
        session_id: "session-1".into(),
        turn_id: "turn-1".into(),
        step_index: 0,
        messages: Arc::from([Arc::new(TurnMessage::User {
            content: "hello".into(),
        })]),
        allow_tools: false,
    };

    let mut stream = runner.start(request).await.unwrap();

    assert!(stream.next().await.is_some());
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../provider/tests/fixtures")
        .join(name)
}
