use std::path::PathBuf;

use argus_core::ResponseEvent;
use futures::StreamExt;
use provider::{
    Dialect, ProviderClient, ProviderConfig, ProviderDevOptions, ReplayTiming, Request,
};

fn request() -> Request {
    provider::dialect::openai::schema::request::ChatCompletionsOptions {
        model: "gpt-test".into(),
        stream: Some(true),
        ..Default::default()
    }
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[tokio::test]
async fn provider_client_replay_mode_still_emits_created_and_done() {
    let client = ProviderClient::new(
        ProviderConfig::new(Dialect::Openai, "http://unused", "test-key").with_dev_options(
            ProviderDevOptions::replay(
                fixture_path("2026-03-06-openai-chat-completions-sse.txt"),
                ReplayTiming::Fast,
            ),
        ),
    )
    .unwrap();

    let mut stream = client.stream(request()).unwrap();
    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(event);
    }

    assert!(events.iter().any(|e| matches!(e, ResponseEvent::Created(_))));
    assert!(events.iter().any(|e| matches!(e, ResponseEvent::Done { .. })));
}
