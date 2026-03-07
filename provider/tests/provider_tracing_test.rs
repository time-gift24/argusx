use std::{
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use argus_core::ResponseEvent;
use futures::StreamExt;
use provider::{
    Dialect, ProviderClient, ProviderConfig, ProviderDevOptions, ReplayTiming, Request,
};
use tempfile::tempdir;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
struct SharedBuf(Arc<Mutex<Vec<u8>>>);

impl<'a> MakeWriter<'a> for SharedBuf {
    type Writer = SharedBufGuard;

    fn make_writer(&'a self) -> Self::Writer {
        SharedBufGuard(self.0.clone())
    }
}

struct SharedBufGuard(Arc<Mutex<Vec<u8>>>);

impl Write for SharedBufGuard {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[tokio::test]
async fn provider_tracing_emits_mode_and_completion_fields() {
    let logs = SharedBuf::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(logs.clone())
        .with_ansi(false)
        .without_time()
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let request: Request = provider::dialect::openai::schema::request::ChatCompletionsOptions {
        model: "gpt-test".into(),
        stream: Some(true),
        ..Default::default()
    };
    let client = ProviderClient::new(
        ProviderConfig::new(Dialect::Openai, "http://unused", "test-key").with_dev_options(
            ProviderDevOptions::replay(
                fixture_path("2026-03-06-openai-chat-completions-sse.txt"),
                ReplayTiming::Fast,
            ),
        ),
    )
    .unwrap();

    let mut stream = client.stream(request).unwrap();
    while stream.next().await.is_some() {}

    let output = String::from_utf8(logs.0.lock().unwrap().clone()).unwrap();
    assert!(output.contains("provider.stream"));
    assert!(output.contains("mode=\"replay\"") || output.contains("mode=replay"));
    assert!(output.contains("stream completed"));
}

#[tokio::test]
async fn provider_tracing_does_not_report_completion_for_terminal_mapper_errors() {
    let logs = SharedBuf::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(logs.clone())
        .with_ansi(false)
        .without_time()
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let dir = tempdir().unwrap();
    let capture = dir.path().join("bad-tool-call.sse");
    tokio::fs::write(
        &capture,
        concat!(
            "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",",
            "\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call-1\"}]}}]}\n\n",
            "data: [DONE]\n\n"
        ),
    )
    .await
    .unwrap();

    let request: Request = provider::dialect::openai::schema::request::ChatCompletionsOptions {
        model: "gpt-test".into(),
        stream: Some(true),
        ..Default::default()
    };
    let client = ProviderClient::new(
        ProviderConfig::new(Dialect::Openai, "http://unused", "test-key")
            .with_dev_options(ProviderDevOptions::replay(capture, ReplayTiming::Fast)),
    )
    .unwrap();

    let events: Vec<_> = client.stream(request).unwrap().collect().await;
    assert!(matches!(events.last(), Some(ResponseEvent::Error(_))));

    let output = String::from_utf8(logs.0.lock().unwrap().clone()).unwrap();
    assert!(output.contains("provider.stream"));
    assert!(!output.contains("stream completed"));
}
