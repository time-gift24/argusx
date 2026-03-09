use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use argus_core::{Builtin, BuiltinToolCall, ToolCall};
use desktop_lib::bootstrap::build_desktop_bootstrap_with_workspace_root;
use runtime::{build_runtime_from_config, AppConfig, PathsConfig, TelemetrySection};
use tokio_util::sync::CancellationToken;
use turn::ToolRunner;

#[tokio::test(flavor = "current_thread")]
async fn desktop_bootstrap_routes_provider_settings_and_tool_roots_through_runtime() {
    let root = temp_dir("desktop-bootstrap-runtime");
    let workspace_root = root.join("workspace");
    fs::create_dir_all(&workspace_root).unwrap();
    fs::write(workspace_root.join("note.txt"), "bootstrap-root").unwrap();

    let runtime = build_runtime_from_config(test_config(&root)).await.unwrap();
    let bootstrap =
        build_desktop_bootstrap_with_workspace_root(runtime, workspace_root.clone()).unwrap();
    let expected_db = root.join("app").join("desktop.sqlite3");

    assert_eq!(bootstrap.provider_settings_db_path, expected_db);
    assert!(expected_db.exists());

    let output = bootstrap
        .session_state
        .tool_runner()
        .execute(
            ToolCall::Builtin(BuiltinToolCall {
                sequence: 0,
                call_id: "call-read".into(),
                builtin: Builtin::Read,
                arguments_json: serde_json::json!({
                    "path": workspace_root.join("note.txt").to_string_lossy().to_string(),
                    "mode": "text",
                })
                .to_string(),
            }),
            tool::ToolContext::new("session-1", "turn-1", CancellationToken::new()),
        )
        .await
        .unwrap();

    assert_eq!(output.output["content"], "bootstrap-root");
}

fn test_config(root: &PathBuf) -> AppConfig {
    AppConfig {
        paths: PathsConfig {
            sqlite: root.join("app").join("sqlite.db"),
            log_file: root.join("app").join("argusx.log"),
        },
        telemetry: TelemetrySection {
            enabled: false,
            clickhouse_url: "http://127.0.0.1:8123".into(),
            database: "argusx".into(),
            table: "telemetry_logs".into(),
            high_priority_batch_size: 5,
            low_priority_batch_size: 500,
            high_priority_flush_interval_ms: 1_000,
            low_priority_flush_interval_ms: 30_000,
            max_in_memory_events: 10_000,
            max_retry_backoff_ms: 30_000,
            full_logging: false,
            delta_events: false,
        },
    }
}

fn temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{prefix}-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}
