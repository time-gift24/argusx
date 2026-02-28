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
