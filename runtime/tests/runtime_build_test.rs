use runtime::{build_runtime_from_config, AppConfig, PathsConfig, TelemetrySection};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn build_runtime_with_disabled_telemetry_creates_sqlite_and_initializes_session() {
    let temp = tempfile::tempdir().unwrap();
    let sqlite = temp.path().join("sqlite.db");
    let log_file = temp.path().join("argusx.log");

    let config = AppConfig {
        paths: PathsConfig {
            sqlite: sqlite.clone(),
            log_file: log_file.clone(),
        },
        telemetry: TelemetrySection {
            enabled: false,
            clickhouse_url: "http://localhost:8123".into(),
            database: "argusx".into(),
            table: "telemetry_logs".into(),
            high_priority_batch_size: 5,
            low_priority_batch_size: 500,
            high_priority_flush_interval_ms: 1000,
            low_priority_flush_interval_ms: 30000,
            max_in_memory_events: 10000,
            max_retry_backoff_ms: 30000,
            full_logging: false,
            delta_events: false,
        },
    };

    let runtime = build_runtime_from_config(config).await.unwrap();

    assert!(sqlite.exists());
    assert!(log_file.exists());
    assert!(runtime.telemetry.is_none());
    assert!(runtime.session_manager.list_threads().await.unwrap().is_empty());
}
