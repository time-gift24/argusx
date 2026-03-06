use std::path::PathBuf;

use argus_core::ResponseEvent;
use futures::StreamExt;
use provider::{
    Dialect, ProviderClient, ProviderConfig, ProviderDevOptions, ReplayTiming, Request,
};
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn request() -> Request {
    provider::dialect::openai::schema::request::ChatCompletionsOptions {
        model: "gpt-test".into(),
        stream: Some(true),
        ..Default::default()
    }
}

#[tokio::test]
async fn recorded_live_stream_can_be_replayed_through_same_mapper_path() {
    let server = MockServer::start().await;
    let body = include_str!("fixtures/2026-03-06-openai-chat-completions-sse.txt");

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let capture = dir.path().join("capture.sse");

    let live = ProviderClient::new(
        ProviderConfig::new(Dialect::Openai, server.uri(), "test-key")
            .with_dev_options(ProviderDevOptions::record_only(capture.clone())),
    )
    .unwrap();

    let mut live_stream = live.stream(request()).unwrap();
    while live_stream.next().await.is_some() {}

    let replay = ProviderClient::new(
        ProviderConfig::new(Dialect::Openai, "http://unused", "test-key").with_dev_options(
            ProviderDevOptions::replay(capture, ReplayTiming::Fast),
        ),
    )
    .unwrap();

    let mut replay_stream = replay.stream(request()).unwrap();
    let mut replay_events = Vec::new();
    while let Some(event) = replay_stream.next().await {
        replay_events.push(event);
    }

    assert!(replay_events.iter().any(|e| matches!(e, ResponseEvent::Created(_))));
    assert!(replay_events.iter().any(|e| matches!(e, ResponseEvent::Done { .. })));
}

#[tokio::test]
async fn recorder_creation_failure_does_not_break_live_stream() {
    let server = MockServer::start().await;
    let body = include_str!("fixtures/2026-03-06-openai-chat-completions-sse.txt");

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let capture_path: PathBuf = dir.path().to_path_buf();

    let client = ProviderClient::new(
        ProviderConfig::new(Dialect::Openai, server.uri(), "test-key")
            .with_dev_options(ProviderDevOptions::record_only(capture_path)),
    )
    .unwrap();

    let mut stream = client.stream(request()).unwrap();
    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(event);
    }

    assert!(events.iter().any(|e| matches!(e, ResponseEvent::Done { .. })));
}
