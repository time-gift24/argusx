use std::collections::HashMap;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn anthropic_adapter_can_chat() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "msg_1",
            "model": "claude-sonnet-4-20250514",
            "content": [
                {"type": "text", "text": "hello from anthropic"}
            ],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 12,
                "output_tokens": 8
            }
        })))
        .mount(&mock_server)
        .await;

    let client = llm_client::LlmClient::builder()
        .with_anthropic_adapter(mock_server.uri(), "test-key", HashMap::new())
        .unwrap()
        .default_adapter("anthropic")
        .build()
        .unwrap();

    let req = llm_client::LlmRequest {
        model: "claude-sonnet-4-20250514".to_string(),
        messages: vec![llm_client::LlmMessage {
            role: llm_client::LlmRole::User,
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
