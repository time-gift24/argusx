#[test]
fn sensitive_filter_redacts_auth_headers() {
    let preview = telemetry::redact_preview(r#"{"authorization":"Bearer secret","prompt":"hello"}"#, 256);
    assert!(preview.contains("[REDACTED]"));
    assert!(!preview.contains("secret"));
}
