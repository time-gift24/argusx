use argusx_common::config::Settings;
use std::sync::atomic::{AtomicU64, Ordering};

static DB_COUNTER: AtomicU64 = AtomicU64::new(0);

fn settings_for_temp() -> Settings {
    let seq = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let unique = format!(
        "prompt_lab_sql_schema_v2_{}_{}_{}.db",
        std::process::id(),
        seq,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
    );
    let path = std::env::temp_dir().join(unique);
    Settings {
        database: argusx_common::config::DatabaseConfig {
            path: path.to_string_lossy().to_string(),
            busy_timeout_ms: 5_000,
            max_connections: 5,
        },
        logging: argusx_common::config::LoggingConfig::default(),
    }
}

#[tokio::test]
async fn check_results_manual_unique_index_contract() {
    let settings = settings_for_temp();
    let db_path = settings.database.path.clone();
    let _lab = prompt_lab_core::PromptLab::new(settings)
        .await
        .expect("init prompt lab");
    let pool = sqlx::SqlitePool::connect(&format!("sqlite://{db_path}"))
        .await
        .expect("connect sqlite");
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT sql FROM sqlite_master WHERE type='index' AND name='idx_check_results_manual_latest'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0]
        .0
        .contains("WHERE source_type = 2 AND check_item_id IS NOT NULL"));
}
