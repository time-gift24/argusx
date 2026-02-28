// llm-client/tests/client_builder_bigmodel_test.rs
#[test]
fn with_default_bigmodel_from_env_requires_api_key() {
    std::env::remove_var("BIGMODEL_API_KEY");
    let result = llm_client::LlmClient::builder().with_default_bigmodel_from_env();
    assert!(result.is_err());
}
