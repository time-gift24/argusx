use futures::StreamExt;
use provider::{Dialect, ProviderClient, ProviderConfig, Request};
use telemetry::{RecordingSink, TelemetryConfig, TelemetryLayer};
use tracing_subscriber::{Registry, layer::SubscriberExt};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn stream_body() -> String {
    concat!(
        "data: {\"id\":\"resp-1\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",",
        "\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hi\"}}]}\n\n",
        "data: {\"id\":\"resp-1\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",",
        "\"choices\":[{\"index\":0,\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":5,\"completion_tokens\":7,\"total_tokens\":12}}\n\n",
        "data: [DONE]\n\n"
    )
    .to_string()
}

#[tokio::test(flavor = "current_thread")]
async fn provider_emits_request_and_completion_events() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(stream_body(), "text/event-stream"))
        .mount(&server)
        .await;

    let sink = RecordingSink::default();
    let subscriber = Registry::default().with(TelemetryLayer::new(
        sink.clone(),
        TelemetryConfig::default(),
    ));

    let _guard = tracing::subscriber::set_default(subscriber);
    let client = ProviderClient::new(ProviderConfig::new(
        Dialect::Openai,
        server.uri(),
        "test-key",
    ))
    .unwrap();
    let stream = client.stream(Request::default()).unwrap();
    let _: Vec<_> = stream.collect().await;

    let records = sink.take();
    assert!(
        records
            .iter()
            .any(|record| record.event_name == "llm_request")
    );
    assert!(records.iter().any(|record| {
        record.event_name == "llm_response_completed"
            && record.total_tokens == Some(12)
            && record.model_name.as_deref() == Some("gpt-test")
            && record.provider.as_deref() == Some("Openai")
    }));
}

#[tokio::test(flavor = "current_thread")]
async fn provider_completion_dedupe_key_is_stable_for_replayed_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(stream_body(), "text/event-stream"))
        .expect(2)
        .mount(&server)
        .await;

    let sink = RecordingSink::default();
    let subscriber = Registry::default().with(TelemetryLayer::new(
        sink.clone(),
        TelemetryConfig::default(),
    ));

    let _guard = tracing::subscriber::set_default(subscriber);
    let client = ProviderClient::new(ProviderConfig::new(
        Dialect::Openai,
        server.uri(),
        "test-key",
    ))
    .unwrap();

    for _ in 0..2 {
        let stream = client.stream(Request::default()).unwrap();
        let _: Vec<_> = stream.collect().await;
    }

    let completions: Vec<_> = sink
        .take()
        .into_iter()
        .filter(|record| record.event_name == "llm_response_completed")
        .collect();

    assert_eq!(completions.len(), 2);
    assert_eq!(
        completions[0].billing_dedupe_key,
        completions[1].billing_dedupe_key
    );
}
