use std::collections::HashMap;
use std::sync::Arc;

use llm_client::{LlmClient, LlmMessage, LlmRequest, LlmRole};
use llm_provider::anthropic::{AnthropicAdapter, AnthropicConfig};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn anthropic_adapter_can_chat() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "msg_1",
            "model": "claude-test",
            "content": [{"type": "text", "text": "hello from anthropic"}],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 12,
                "output_tokens": 8
            }
        })))
        .mount(&mock_server)
        .await;

    let cfg =
        AnthropicConfig::new(mock_server.uri(), "test-key", HashMap::new()).expect("valid config");
    let adapter = Arc::new(AnthropicAdapter::new(cfg));

    let client = LlmClient::builder()
        .register_adapter(adapter)
        .default_adapter("anthropic")
        .build()
        .unwrap();

    let req = LlmRequest {
        model: "claude-test".to_string(),
        messages: vec![LlmMessage {
            role: LlmRole::User,
            content: "hello".to_string(),
        }],
        stream: false,
        max_tokens: Some(256),
        temperature: Some(0.2),
        top_p: Some(1.0),
        tools: None,
    };

    let resp = client.chat(req).await.unwrap();
    assert_eq!(resp.output_text, "hello from anthropic");
    let usage = resp.usage.expect("usage");
    assert_eq!(usage.input_tokens, 12);
    assert_eq!(usage.output_tokens, 8);
    assert_eq!(usage.total_tokens, 20);
}

#[test]
fn config_requires_base_url() {
    let err = AnthropicConfig::new("", "k", HashMap::new()).unwrap_err();
    assert!(err.to_string().contains("base_url"));
}
