use std::collections::HashMap;
use std::sync::Arc;

use llm_client::{LlmClient, LlmMessage, LlmRequest, LlmRole};
use llm_provider::bigmodel::{BigModelAdapter, BigModelConfig};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn bigmodel_adapter_retries_and_chats() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal error"))
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "test",
            "created": 0,
            "model": "glm-test",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "OK"},
                "finish_reason": "stop"
            }]
        })))
        .mount(&mock_server)
        .await;

    let cfg =
        BigModelConfig::new(mock_server.uri(), "test-key", HashMap::new()).expect("valid config");
    let adapter = Arc::new(BigModelAdapter::new(cfg));

    let client = LlmClient::builder()
        .register_adapter(adapter)
        .default_adapter("bigmodel")
        .build()
        .unwrap();

    let req = LlmRequest {
        model: "glm-test".to_string(),
        messages: vec![LlmMessage {
            role: LlmRole::User,
            content: "hello".to_string(),
        }],
        stream: false,
        max_tokens: None,
        temperature: None,
        top_p: None,
        tools: None,
    };

    let res = client.chat(req).await.unwrap();
    assert_eq!(res.model, "glm-test");
    assert_eq!(res.output_text, "OK");
}

#[test]
fn config_requires_base_url() {
    let err = BigModelConfig::new("", "k", HashMap::new()).unwrap_err();
    assert!(err.to_string().contains("base_url"));
}
