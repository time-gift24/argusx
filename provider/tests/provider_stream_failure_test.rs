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
async fn http_status_failure_becomes_terminal_error_event() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    let client = ProviderClient::new(ProviderConfig {
        dialect: Dialect::Openai,
        base_url: server.uri(),
        api_key: "test-key".into(),
        headers: Default::default(),
    })
    .unwrap();

    let stream = client.stream(request("gpt-test")).unwrap();
    let events: Vec<_> = stream.collect().await;

    assert!(matches!(
        events.last(),
        Some(ResponseEvent::Error(err)) if err.message.contains("HttpStatus")
    ));
}

#[tokio::test]
async fn malformed_chunk_becomes_terminal_parse_error_event() {
    let server = MockServer::start().await;
    let body = "data: {bad json}\n\n";

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

    let stream = client.stream(request("gpt-test")).unwrap();
    let events: Vec<_> = stream.collect().await;

    assert!(matches!(
        events.last(),
        Some(ResponseEvent::Error(err)) if err.message.contains("Parse")
    ));
}

#[tokio::test]
async fn eof_without_done_becomes_terminal_protocol_error_event() {
    let server = MockServer::start().await;
    let body = concat!(
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",",
        "\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hi\"}}]}\n\n"
    );

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

    let stream = client.stream(request("gpt-test")).unwrap();
    let events: Vec<_> = stream.collect().await;

    assert!(events.iter().any(|e| matches!(e, ResponseEvent::Created(_))));
    assert!(matches!(
        events.last(),
        Some(ResponseEvent::Error(err)) if err.message.contains("Protocol")
    ));
}
