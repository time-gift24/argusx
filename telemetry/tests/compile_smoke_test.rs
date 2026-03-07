use telemetry::{TelemetryConfig, TelemetryError};

#[test]
fn telemetry_crate_exports_config_and_error_types() {
    let config = TelemetryConfig::default();
    assert_eq!(config.high_priority_batch_size, 5);

    let err = TelemetryError::Validation("boom".into());
    assert!(err.to_string().contains("boom"));
}
