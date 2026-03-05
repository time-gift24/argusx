use llm_provider::bigmodel_api;

#[test]
fn test_config_new() {
    let config = bigmodel_api::Config::new("test-key", "https://provider.test/v1");
    assert_eq!(config.api_key, "test-key");
    assert_eq!(config.base_url, "https://provider.test/v1");
}

#[test]
fn test_config_with_base_url() {
    let config = bigmodel_api::Config::new("test-key", "https://provider.test/v1")
        .with_base_url("https://custom.example.com");
    assert_eq!(config.base_url, "https://custom.example.com");
}
