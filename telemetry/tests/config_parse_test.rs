use telemetry::TelemetryConfig;

#[test]
fn example_config_parses_into_runtime_config() {
    let raw = include_str!("../../config/telemetry.toml");
    let config: TelemetryConfig = toml::from_str(raw).unwrap();
    assert_eq!(config.database, "argusx");
    assert_eq!(config.table, "telemetry_logs");
}
