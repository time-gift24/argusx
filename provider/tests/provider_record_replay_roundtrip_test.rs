use std::path::PathBuf;

use argus_core::ResponseEvent;
use futures::{FutureExt, StreamExt};
use provider::{
    Dialect, ProviderClient, ProviderConfig, ProviderDevOptions, ReplayReader, ReplayTiming,
    Request, SseRecorder,
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

async fn wait_for_recording(path: &std::path::Path) {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(1);
    loop {
        if tokio::fs::try_exists(path).await.unwrap() {
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("recording was not finalized in time: {}", path.display());
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
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
    wait_for_recording(&capture).await;

    let replay = ProviderClient::new(
        ProviderConfig::new(Dialect::Openai, "http://unused", "test-key")
            .with_dev_options(ProviderDevOptions::replay(capture, ReplayTiming::Fast)),
    )
    .unwrap();

    let mut replay_stream = replay.stream(request()).unwrap();
    let mut replay_events = Vec::new();
    while let Some(event) = replay_stream.next().await {
        replay_events.push(event);
    }

    assert!(
        replay_events
            .iter()
            .any(|e| matches!(e, ResponseEvent::Created(_)))
    );
    assert!(
        replay_events
            .iter()
            .any(|e| matches!(e, ResponseEvent::Done { .. }))
    );
}

#[tokio::test]
async fn live_recording_preserves_raw_sse_fields() {
    let server = MockServer::start().await;
    let body = concat!(
        "id: evt-1\n",
        "event: message\n",
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",",
        "\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hi\"}}]}\n\n",
        "event: done\n",
        "data: [DONE]\n\n"
    );

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let capture = dir.path().join("capture.sse");

    let client = ProviderClient::new(
        ProviderConfig::new(Dialect::Openai, server.uri(), "test-key")
            .with_dev_options(ProviderDevOptions::record_only(capture.clone())),
    )
    .unwrap();

    let mut stream = client.stream(request()).unwrap();
    while stream.next().await.is_some() {}
    wait_for_recording(&capture).await;

    let recorded = tokio::fs::read_to_string(capture).await.unwrap();
    assert_eq!(recorded, body);
}

#[tokio::test]
async fn live_stream_recording_is_finalized_before_stream_returns() {
    let server = MockServer::start().await;
    let body = include_str!("fixtures/2026-03-06-openai-chat-completions-sse.txt");

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let capture = dir.path().join("capture.sse");

    let client = ProviderClient::new(
        ProviderConfig::new(Dialect::Openai, server.uri(), "test-key")
            .with_dev_options(ProviderDevOptions::record_only(capture.clone())),
    )
    .unwrap();

    let mut stream = client.stream(request()).unwrap();
    while stream.next().await.is_some() {}

    assert!(tokio::fs::try_exists(&capture).await.unwrap());
    assert!(
        tokio::fs::try_exists(capture.with_extension("sse.meta.json"))
            .await
            .unwrap()
    );
    ReplayReader::open(capture, ReplayTiming::Fast)
        .await
        .unwrap();
}

#[tokio::test(start_paused = true)]
async fn recorder_sidecar_can_drive_recorded_replay_timing() {
    let dir = tempdir().unwrap();
    let capture = dir.path().join("capture.sse");
    let mut recorder = SseRecorder::create(capture.clone(), true).await.unwrap();

    recorder
        .write_frame("data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n")
        .await
        .unwrap();
    tokio::task::yield_now().await;
    tokio::time::advance(std::time::Duration::from_millis(15)).await;
    recorder.write_frame("data: [DONE]\n\n").await.unwrap();
    recorder.finish().await.unwrap();

    let mut replay = ReplayReader::open(capture, ReplayTiming::Recorded)
        .await
        .unwrap();
    assert!(replay.next().await.unwrap().unwrap().contains("\"hi\""));

    let second = replay.next();
    futures::pin_mut!(second);
    assert!(second.as_mut().now_or_never().is_none());

    tokio::time::advance(std::time::Duration::from_millis(15)).await;
    assert_eq!(second.await.unwrap().unwrap(), "data: [DONE]\n\n");
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

    assert!(
        events
            .iter()
            .any(|e| matches!(e, ResponseEvent::Done { .. }))
    );
}
