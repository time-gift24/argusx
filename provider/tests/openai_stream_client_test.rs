use argus_core::ResponseEvent;
use futures::StreamExt;
use provider::{Dialect, ProviderClient, ProviderConfig, Request};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn openai_stream_returns_created_deltas_and_done() {
    let server = MockServer::start().await;
    let body = include_str!("fixtures/2026-03-06-openai-chat-completions-sse.txt");

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
        .mount(&server)
        .await;

    let client = ProviderClient::new(ProviderConfig {
        dialect: Dialect::Openai,
        base_url: server.uri(),
        api_key: "test-key".into(),
        headers: Default::default(),
    })
    .unwrap();

    let request: Request = provider::dialect::openai::schema::request::ChatCompletionsOptions {
        model: "gpt-test".into(),
        stream: Some(true),
        ..Default::default()
    };

    let mut stream = client.stream(request).unwrap();
    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(event);
    }

    assert!(events.iter().any(|e| matches!(e, ResponseEvent::Created(_))));
    assert!(events.iter().any(|e| matches!(e, ResponseEvent::Done(Some(_)))));
}
