use agent_center::persistence::models::ThreadRow;
use agent_center::persistence::store::ThreadStore;
use agent_center::AgentCenter;
use chrono::Utc;
use tempfile::tempdir;

#[tokio::test]
async fn reconcile_marks_orphan_running_threads_terminal() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let db_path = temp.path().join("test.db");

    // Create store and insert orphan thread (Running but no active runtime)
    let store = agent_center::persistence::store::SqliteThreadStore::new(&db_path)?;

    let orphan_thread = ThreadRow {
        id: "orphan-1".to_string(),
        parent_thread_id: None,
        status: "Running".to_string(),
        agent_name: "test-agent".to_string(),
        created_at: Utc::now(),
        depth: 0,
        initial_input: None,
    };
    store.upsert_thread(&orphan_thread)?;

    // Create AgentCenter and run reconcile
    let center = AgentCenter::builder().db_path(db_path).build()?;

    let report = center.reconcile().await?;

    // Should mark orphan as terminal
    assert!(report.repaired_count > 0, "should repair orphan threads");

    // Verify thread state changed to terminal
    let repaired = store.get_thread("orphan-1")?.expect("thread should exist");
    assert!(
        matches!(repaired.status.as_str(), "Failed" | "Closed"),
        "repaired thread should be in terminal state, got: {}",
        repaired.status
    );

    Ok(())
}

#[tokio::test]
async fn reconcile_is_idempotent() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let db_path = temp.path().join("test.db");

    let center = AgentCenter::builder().db_path(db_path).build()?;

    // Run reconcile twice on clean state
    let report1 = center.reconcile().await?;
    let report2 = center.reconcile().await?;

    // Both should succeed with no repairs
    assert_eq!(report1.repaired_count, 0);
    assert_eq!(report2.repaired_count, 0);

    Ok(())
}
