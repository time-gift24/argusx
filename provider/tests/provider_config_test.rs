use argus_core::ResponseEvent;
use futures::StreamExt;
use provider::{Dialect, ProviderClient, ProviderConfig, Request};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn request(model: &str) -> Request {
    provider::dialect::openai::schema::request::ChatCompletionsOptions {
        model: model.into(),
        stream: Some(true),
        ..Default::default()
    }
}

#[tokio::test]
async fn custom_chat_completions_path_overrides_default_path() {
    let server = MockServer::start().await;
    let body = include_str!("fixtures/2026-03-06-openai-chat-completions-sse.txt");

    Mock::given(method("POST"))
        .and(path("/custom/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
        .mount(&server)
        .await;

    let client = ProviderClient::new(
        ProviderConfig::new(Dialect::Openai, server.uri(), "test-key")
            .with_chat_completions_path("/custom/chat/completions"),
    )
    .unwrap();

    let events: Vec<_> = client.stream(request("gpt-test")).unwrap().collect().await;

    assert!(
        events
            .iter()
            .any(|e| matches!(e, ResponseEvent::Done { usage: Some(_), .. }))
    );
}
