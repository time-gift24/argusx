use argus_core::ResponseEvent;
use futures::StreamExt;
use provider::{Dialect, ProviderClient, ProviderConfig, Request};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn zai_stream_emits_mcp_and_done() {
    let server = MockServer::start().await;
    let body = include_str!("fixtures/2026-03-06-zai-chat-completions-sse.txt");

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
        .mount(&server)
        .await;

    let client =
        ProviderClient::new(ProviderConfig::new(Dialect::Zai, server.uri(), "test-key")).unwrap();

    let request: Request = provider::dialect::openai::schema::request::ChatCompletionsOptions {
        model: "glm-test".into(),
        stream: Some(true),
        ..Default::default()
    };

    let stream = client.stream(request).unwrap();
    let events: Vec<_> = stream.collect().await;

    assert!(
        events
            .iter()
            .any(|e| matches!(e, ResponseEvent::ToolDone(_)))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, ResponseEvent::Done { usage: Some(_), .. }))
    );
}
