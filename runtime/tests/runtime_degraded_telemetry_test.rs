use runtime::{build_runtime_from_config, AppConfig, PathsConfig, TelemetrySection};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn build_runtime_degrades_when_clickhouse_probe_fails() {
    let temp = tempfile::tempdir().unwrap();
    let config = AppConfig {
        paths: PathsConfig {
            sqlite: temp.path().join("sqlite.db"),
            log_file: temp.path().join("argusx.log"),
        },
        telemetry: TelemetrySection {
            enabled: true,
            clickhouse_url: "http://127.0.0.1:9".into(),
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

    assert!(runtime.telemetry.is_none());
}
