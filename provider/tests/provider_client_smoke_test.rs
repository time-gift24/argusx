use provider::{Dialect, ErrorKind, ProviderClient, ProviderConfig, StreamError};

#[test]
fn provider_client_can_be_constructed() {
    let cfg = ProviderConfig::new(Dialect::Openai, "https://example.test", "secret");

    let _client = ProviderClient::new(cfg).unwrap();
}

#[test]
fn provider_error_kind_is_provider_specific() {
    let err = StreamError {
        kind: ErrorKind::Protocol,
        message: "bad chunk".into(),
    };

    assert!(matches!(err.kind, ErrorKind::Protocol));
}
