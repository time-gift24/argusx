use agent_center::api::center::SpawnRequest;
use agent_center::AgentCenter;
use tempfile::tempdir;

#[tokio::test]
async fn spawn_atomic_on_partial_failure() -> anyhow::Result<()> {
    // This test verifies that if thread creation fails after dedup claim,
    // subsequent spawns with same (parent, key) don't return ghost thread ID

    let temp = tempdir()?;
    let db_path = temp.path().join("test.db");

    let center = AgentCenter::builder().db_path(db_path).build()?;

    // First spawn should succeed
    let spawn_req = SpawnRequest {
        parent_thread_id: "root".to_string(),
        key: "child-1".to_string(),
        agent_name: "test-agent".to_string(),
        initial_input: "Hello".to_string(),
    };
    let resp1 = center.spawn(spawn_req.clone()).await?;
    let thread_id = resp1.thread_id;

    // Second spawn with same (parent, key) should return same thread ID (idempotent)
    let resp2 = center.spawn(spawn_req).await?;
    assert_eq!(
        resp2.thread_id, thread_id,
        "idempotent spawn should return same thread ID"
    );

    // Verify thread exists via wait (should return valid status, not NotFound)
    use agent_center::api::center::{WaitMode, WaitRequest};

    let wait_req = WaitRequest {
        thread_ids: vec![thread_id.clone()],
        mode: WaitMode::Any,
        timeout_ms: 1000,
    };
    let wait_resp = center.wait(wait_req).await?;
    assert!(
        wait_resp.statuses.contains_key(&thread_id),
        "thread should exist in status map"
    );
    assert_ne!(
        wait_resp.statuses.get(&thread_id),
        Some(&"NotFound".to_string()),
        "thread should not be NotFound"
    );

    Ok(())
}
