use argusx_common::config::Settings;
use prompt_lab_core::{CheckResultFilter, PromptLab, SourceType, UpsertCheckResultInput};
use std::sync::atomic::{AtomicU64, Ordering};

static DB_COUNTER: AtomicU64 = AtomicU64::new(0);

fn settings_for_temp() -> Settings {
    let seq = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let unique = format!(
        "prompt_lab_core_flow_v2_{}_{}_{}.db",
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

struct TestLab {
    lab: PromptLab,
    _pool: sqlx::SqlitePool,
}

impl TestLab {
    fn check_result_service(&self) -> prompt_lab_core::CheckResultService {
        self.lab.check_result_service()
    }
}

async fn test_lab() -> TestLab {
    let settings = settings_for_temp();
    let db_path = settings.database.path.clone();
    let lab = PromptLab::new(settings).await.expect("init prompt lab");
    let pool = sqlx::SqlitePool::connect(&format!("sqlite://{db_path}"))
        .await
        .expect("connect sqlite");
    seed_check_item(&pool, 7).await;
    TestLab { lab, _pool: pool }
}

async fn seed_check_item(pool: &sqlx::SqlitePool, id: i64) {
    sqlx::query(
        r#"
        INSERT INTO checklist_items (
          id, name, prompt, temperature, context_type,
          result_schema, version, status, created_at, updated_at, deleted_at
        ) VALUES (?1, 'Rule', 'check', 0.0, 'sop', NULL, 1, 'active', ?2, ?2, NULL)
        "#,
    )
    .bind(id)
    .bind(now_ms())
    .execute(pool)
    .await
    .expect("seed checklist item");
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock")
        .as_millis() as i64
}

fn input_manual(check_item_id: Option<i64>, is_pass: bool) -> UpsertCheckResultInput {
    UpsertCheckResultInput {
        id: None,
        context_type: "sop".to_string(),
        context_key: "sop:SOP-1".to_string(),
        check_item_id,
        source_type: SourceType::Manual,
        operator_id: Some("u1".to_string()),
        result: Some(serde_json::json!({"ok": is_pass})),
        is_pass,
    }
}

fn filter_key() -> CheckResultFilter {
    CheckResultFilter {
        context_type: Some("sop".to_string()),
        context_key: Some("sop:SOP-1".to_string()),
        check_item_id: Some(7),
    }
}

#[tokio::test]
async fn manual_with_non_null_check_item_keeps_single_latest() {
    let lab = test_lab().await;
    let first = lab
        .check_result_service()
        .upsert_or_append(input_manual(Some(7), false))
        .await
        .unwrap();
    let second = lab
        .check_result_service()
        .upsert_or_append(input_manual(Some(7), true))
        .await
        .unwrap();
    assert_eq!(first.id, second.id);
    let listed = lab.check_result_service().list(filter_key()).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert!(listed[0].is_pass);
}
