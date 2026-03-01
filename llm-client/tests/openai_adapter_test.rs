use std::collections::HashMap;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn openai_adapter_can_chat() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chatcmpl-1",
            "created": 1700000000,
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "hello from openai"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        })))
        .mount(&mock_server)
        .await;

    let client = llm_client::LlmClient::builder()
        .with_openai_adapter(mock_server.uri(), "test-key", HashMap::new())
        .unwrap()
        .default_adapter("openai")
        .build()
        .unwrap();

    let req = llm_client::LlmRequest {
        model: "gpt-4o".to_string(),
        messages: vec![llm_client::LlmMessage {
            role: llm_client::LlmRole::User,
            content: "hello".to_string(),
        }],
        stream: false,
        max_tokens: Some(128),
        temperature: Some(0.1),
        top_p: Some(1.0),
        tools: None,
    };

    let resp = client.chat(req).await.unwrap();
    assert_eq!(resp.model, "gpt-4o");
    assert_eq!(resp.output_text, "hello from openai");
    let usage = resp.usage.expect("usage");
    assert_eq!(usage.total_tokens, 15);
}
