use runtime::ensure_app_config_at;

#[test]
fn ensure_app_config_creates_default_file_and_expands_paths() {
    let temp = tempfile::tempdir().unwrap();
    let app_home = temp.path().join(".argusx");

    let (config_path, config) = ensure_app_config_at(&app_home).unwrap();

    assert_eq!(config_path, app_home.join("argusx.toml"));
    assert!(config_path.exists());
    assert_eq!(config.paths.sqlite, app_home.join("sqlite.db"));
    assert_eq!(config.paths.log_file, app_home.join("argusx.log"));
}

#[test]
fn ensure_app_config_reuses_existing_file() {
    let temp = tempfile::tempdir().unwrap();
    let app_home = temp.path().join(".argusx");
    std::fs::create_dir_all(&app_home).unwrap();
    std::fs::write(
        app_home.join("argusx.toml"),
        r#"
[paths]
sqlite = "./state/app.db"
log_file = "./logs/app.log"

[telemetry]
enabled = false
clickhouse_url = "http://localhost:8123"
database = "argusx"
table = "telemetry_logs"
high_priority_batch_size = 5
low_priority_batch_size = 500
high_priority_flush_interval_ms = 1000
low_priority_flush_interval_ms = 30000
max_in_memory_events = 10000
max_retry_backoff_ms = 30000
full_logging = false
delta_events = false
"#,
    )
    .unwrap();

    let (_, config) = ensure_app_config_at(&app_home).unwrap();

    assert_eq!(config.paths.sqlite, app_home.join("state/app.db"));
    assert_eq!(config.paths.log_file, app_home.join("logs/app.log"));
    assert!(!config.telemetry.enabled);
}

#[test]
fn ensure_app_config_expands_arbitrary_home_relative_paths() {
    let temp = tempfile::tempdir().unwrap();
    let app_home = temp.path().join(".argusx");
    std::fs::create_dir_all(&app_home).unwrap();
    std::fs::write(
        app_home.join("argusx.toml"),
        r#"
[paths]
sqlite = "~/.argusx/state/app.db"
log_file = "~/logs/argusx.log"

[telemetry]
enabled = false
clickhouse_url = "http://localhost:8123"
database = "argusx"
table = "telemetry_logs"
high_priority_batch_size = 5
low_priority_batch_size = 500
high_priority_flush_interval_ms = 1000
low_priority_flush_interval_ms = 30000
max_in_memory_events = 10000
max_retry_backoff_ms = 30000
full_logging = false
delta_events = false
"#,
    )
    .unwrap();

    let (_, config) = ensure_app_config_at(&app_home).unwrap();

    assert_eq!(config.paths.sqlite, app_home.join("state/app.db"));
    assert_eq!(config.paths.log_file, temp.path().join("logs/argusx.log"));
}
