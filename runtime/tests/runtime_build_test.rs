use std::sync::{Mutex, OnceLock};

use runtime::{AppConfig, PathsConfig, TelemetrySection, build_runtime_from_config};

fn tracing_test_guard() -> std::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("runtime test tracing guard poisoned")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn build_runtime_with_disabled_telemetry_creates_sqlite_and_initializes_session() {
    let _guard = tracing_test_guard();
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
    assert!(
        runtime
            .session_manager
            .list_threads()
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn build_runtime_initializes_agent_services() {
    let _guard = tracing_test_guard();
    let temp = tempfile::tempdir().unwrap();
    let config = AppConfig {
        paths: PathsConfig {
            sqlite: temp.path().join("sqlite.db"),
            log_file: temp.path().join("argusx.log"),
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
    let builtin = runtime.agent_profiles.get_profile("builtin-main").await.unwrap();

    assert!(builtin.is_some());
}
