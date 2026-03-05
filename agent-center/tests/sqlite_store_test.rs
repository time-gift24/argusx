use agent_center::persistence::models::ThreadRow;
use agent_center::persistence::store::ThreadStore;
use tempfile::tempdir;

#[test]
fn dedup_returns_existing_thread_id() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let db_path = temp.path().join("test.db");
    let store = agent_center::persistence::store::SqliteThreadStore::new(&db_path)?;

    // First spawn with parent=p1, key=k1 creates thread t1
    let thread1 = ThreadRow {
        id: "t1".to_string(),
        parent_thread_id: Some("p1".to_string()),
        status: "Running".to_string(),
        agent_name: "test-agent".to_string(),
        created_at: chrono::Utc::now(),
        depth: 0,
        initial_input: Some("Hello".to_string()),
    };
    store.upsert_thread(&thread1)?;
    store.insert_dedup("p1", "k1", "t1")?;

    // Second spawn with same parent+key should return existing t1
    let existing = store.get_by_dedup("p1", "k1")?;
    assert!(existing.is_some());
    assert_eq!(existing.unwrap(), "t1");

    Ok(())
}

#[test]
fn upsert_thread_creates_and_updates() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let db_path = temp.path().join("test.db");
    let store = agent_center::persistence::store::SqliteThreadStore::new(&db_path)?;

    let thread = ThreadRow {
        id: "t1".to_string(),
        parent_thread_id: None,
        status: "Pending".to_string(),
        agent_name: "test-agent".to_string(),
        created_at: chrono::Utc::now(),
        depth: 0,
        initial_input: None,
    };

    // Create
    store.upsert_thread(&thread)?;

    // Update status
    let updated = ThreadRow {
        status: "Running".to_string(),
        ..thread
    };
    store.upsert_thread(&updated)?;

    Ok(())
}
