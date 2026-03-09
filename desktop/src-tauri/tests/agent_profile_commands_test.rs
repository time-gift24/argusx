use agent::AgentProfileRecord;
use desktop_lib::agent_profiles::commands::list_agent_profiles_from_store;
use runtime::{AppConfig, PathsConfig, TelemetrySection, build_runtime_from_config};

fn disabled_telemetry() -> TelemetrySection {
    TelemetrySection {
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
    }
}

#[tokio::test(flavor = "current_thread")]
async fn list_agent_profiles_returns_builtin_and_custom_profiles() {
    let temp = tempfile::tempdir().unwrap();
    let runtime = build_runtime_from_config(AppConfig {
        paths: PathsConfig {
            sqlite: temp.path().join("sqlite.db"),
            log_file: temp.path().join("argusx.log"),
        },
        telemetry: disabled_telemetry(),
    })
    .await
    .unwrap();

    runtime
        .agent_profiles
        .upsert_profile(&AgentProfileRecord::custom(
            "reviewer",
            "Code Reviewer",
            "Review a change set with an engineering lens",
            "You are a strict reviewer.",
            serde_json::json!({"builtins": ["read", "grep"]}),
        ))
        .await
        .unwrap();

    let profiles = list_agent_profiles_from_store(runtime.agent_profiles.as_ref())
        .await
        .unwrap();

    assert_eq!(profiles.len(), 2);
    assert_eq!(profiles[0].id, "builtin-main");
    assert_eq!(profiles[0].label, "Planner");
    assert_eq!(profiles[1].id, "reviewer");
    assert_eq!(profiles[1].label, "Code Reviewer");
}
