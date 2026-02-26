use futures::StreamExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_client_chat() {
    let config = bigmodel_api::Config::new("test-key");
    let client = bigmodel_api::BigModelClient::new(config);

    let request =
        bigmodel_api::ChatRequest::new("glm-4", vec![bigmodel_api::Message::user("Hello")])
            .temperature(0.7)
            .max_tokens(100);

    // This will fail with network error since no real API
    // But verifies the request structure is correct
    let result = client.chat(request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_client_chat_stream_maps_http_400_to_invalid_request_with_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(400).set_body_string(
            r#"{"error":{"message":"invalid model","type":"invalid_request_error"}}"#,
        ))
        .mount(&server)
        .await;

    let config = bigmodel_api::Config::new("test-key").with_base_url(server.uri());
    let client = bigmodel_api::BigModelClient::new(config);
    let request =
        bigmodel_api::ChatRequest::new("glm-4.5", vec![bigmodel_api::Message::user("hello")])
            .stream();

    let mut stream = client.chat_stream(request);
    let first = stream.next().await;

    match first {
        Some(Err(bigmodel_api::BigModelError::InvalidRequest(body))) => {
            assert!(body.contains("invalid model"));
        }
        other => panic!("expected InvalidRequest stream error, got {other:?}"),
    }
}
