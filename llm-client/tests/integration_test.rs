use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn facade_can_chat_via_bigmodel_adapter() {
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

    let client = llm_client::LlmClient::builder()
        .with_default_bigmodel(mock_server.uri(), "test-key")
        .unwrap()
        .build()
        .unwrap();

    let req = llm_client::LlmRequest {
        model: "glm-5".to_string(),
        messages: vec![llm_client::LlmMessage {
            role: llm_client::LlmRole::User,
            content: "hello".to_string(),
        }],
        stream: false,
        max_tokens: None,
        temperature: None,
        top_p: None,
    };

    let res = client.chat(req).await.unwrap();
    assert_eq!(res.model, "glm-5");
    assert_eq!(res.output_text, "OK");
}
