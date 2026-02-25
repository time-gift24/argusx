use argusx_common::config::Settings;
use prompt_lab_core::{
    CheckResultFilter, CreateSopInput, CreateSopStepInput, PromptLab, SopStatus, SourceType,
    UpdateSopInput, UpsertCheckResultInput,
};
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

    fn sop_service(&self) -> prompt_lab_core::SopService {
        self.lab.sop_service()
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

async fn ensure_sop_exists(lab: &TestLab) -> prompt_lab_core::Sop {
    match lab.sop_service().get_sop_by_sop_id("SOP-1").await {
        Ok(sop) => sop,
        Err(_) => {
            lab.sop_service()
                .create_sop(CreateSopInput {
                    sop_id: "SOP-1".to_string(),
                    name: "SOP-1".to_string(),
                    ticket_id: None,
                    version: Some(1),
                    detect: None,
                    handle: None,
                    verification: None,
                    rollback: None,
                    status: SopStatus::Active,
                })
                .await
                .unwrap()
        }
    }
}

async fn create_step_named(lab: &TestLab, name: &str) -> prompt_lab_core::SopStep {
    let _ = ensure_sop_exists(lab).await;
    lab.sop_service()
        .create_sop_step(CreateSopStepInput {
            sop_id: "SOP-1".to_string(),
            name: name.to_string(),
            version: Some(1),
            operation: None,
            verification: None,
            impact_analysis: None,
            rollback: None,
        })
        .await
        .unwrap()
}

async fn create_sop_with_detect_refs(lab: &TestLab, detect_refs: Vec<serde_json::Value>) {
    let sop = ensure_sop_exists(lab).await;
    lab.sop_service()
        .update_sop(UpdateSopInput {
            id: sop.id,
            sop_id: None,
            name: Some("SOP-1".to_string()),
            ticket_id: None,
            version: Some(1),
            detect: Some(serde_json::json!(detect_refs)),
            handle: None,
            verification: None,
            rollback: None,
            status: Some(SopStatus::Active),
        })
        .await
        .unwrap();
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

#[tokio::test]
async fn get_sop_returns_aggregate_and_normalizes_snapshot_names() {
    let lab = test_lab().await;
    let step = create_step_named(&lab, "真实名称").await;
    create_sop_with_detect_refs(
        &lab,
        vec![serde_json::json!({"sop_step_id": step.id, "name": "旧名称"})],
    )
    .await;
    let agg = lab
        .sop_service()
        .get_sop_aggregate_by_sop_id("SOP-1")
        .await
        .unwrap();
    assert_eq!(agg.detect_steps[0].name, "真实名称");
}
