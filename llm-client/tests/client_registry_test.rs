// llm-client/tests/client_registry_test.rs
use llm_client::LlmClient;

#[tokio::test]
async fn build_client_without_default_adapter_fails() {
    let result = LlmClient::builder().build();
    assert!(result.is_err());
}

#[tokio::test]
async fn calling_unknown_adapter_fails() {
    let client = LlmClient::builder()
        .default_adapter("missing")
        .build()
        .unwrap_err();
    assert!(client.to_string().contains("default adapter"));
}

#[tokio::test]
async fn chat_with_unknown_adapter_fails() {
    let client = LlmClient::builder()
        .with_default_bigmodel("https://open.bigmodel.cn/api/paas/v4", "test-key")
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
        tools: None,
    };
    let err = client
        .chat_with_adapter("openai", req)
        .await
        .expect_err("unknown adapter should fail");
    assert!(err.to_string().contains("adapter 'openai' not found"));
}
