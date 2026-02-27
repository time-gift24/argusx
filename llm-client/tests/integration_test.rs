use bigmodel_api::{ChatRequest, Message};
use llm_client::providers::{BigModelConfig, BigModelHttpClient};
use llm_client::{RetryPolicy, TimeoutConfig};
use std::time::Duration;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::method;

#[tokio::test]
async fn full_flow_with_retries() {
    let mock_server = MockServer::start().await;

    // First two requests fail with 500
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal error"))
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    // Third succeeds
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "test",
            "created": 0,
            "model": "glm-5",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "OK"},
                "finish_reason": "stop"
            }]
        })))
        .mount(&mock_server)
        .await;

    let config = BigModelConfig {
        base_url: mock_server.uri(),
        api_key: "test".to_string(),
    };

    let retry = RetryPolicy::default()
        .max_attempts(3)
        .base_delay(Duration::from_millis(10));

    let client = BigModelHttpClient::with_options(config, retry, TimeoutConfig::default());

    let request = ChatRequest::new("glm-5", vec![Message::user("hi")]);
    let result = client.chat(request).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().id, "test");
}
