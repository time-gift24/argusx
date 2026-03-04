use agent_center::api::center::SpawnRequest;
use agent_center::AgentCenter;
use tempfile::tempdir;

#[tokio::test]
async fn spawn_idempotent_under_concurrency_limit() -> anyhow::Result<()> {
    // Test that duplicate spawn with same (parent, key) succeeds even at concurrency limit
    let temp = tempdir()?;
    let db_path = temp.path().join("test.db");

    let center = AgentCenter::builder()
        .max_concurrent(1) // Very low limit
        .db_path(db_path)
        .build()?;

    // First spawn should succeed
    let spawn_req = SpawnRequest {
        parent_thread_id: "root".to_string(),
        key: "child-1".to_string(),
        agent_name: "test-agent".to_string(),
        initial_input: "Hello".to_string(),
    };
    let resp1 = center.spawn(spawn_req.clone()).await?;
    let thread_id = resp1.thread_id;
    assert!(!thread_id.is_empty(), "first spawn should return valid thread ID");

    // Second spawn with same (parent, key) should return same thread ID (idempotent)
    // even though we're at concurrency limit (max_concurrent=1)
    let resp2 = center.spawn(spawn_req).await?;
    assert_eq!(resp2.thread_id, thread_id, "idempotent spawn should return same thread ID");

    Ok(())
}

#[tokio::test]
async fn spawn_different_keys_respect_concurrency_limit() -> anyhow::Result<()> {
    // Test that different keys are still limited by concurrency limit
    let temp = tempdir()?;
    let db_path = temp.path().join("test.db");

    let center = AgentCenter::builder()
        .max_concurrent(1) // Very low limit
        .db_path(db_path)
        .build()?;

    // First spawn should succeed
    let spawn_req1 = SpawnRequest {
        parent_thread_id: "root".to_string(),
        key: "child-1".to_string(),
        agent_name: "test-agent".to_string(),
        initial_input: "Hello".to_string(),
    };
    let resp1 = center.spawn(spawn_req1).await?;
    assert!(!resp1.thread_id.is_empty(), "first spawn should succeed");

    // Second spawn with different key should fail (concurrency limit reached)
    let spawn_req2 = SpawnRequest {
        parent_thread_id: "root".to_string(),
        key: "child-2".to_string(), // Different key
        agent_name: "test-agent".to_string(),
        initial_input: "Hello".to_string(),
    };
    let result = center.spawn(spawn_req2).await;
    assert!(result.is_err(), "spawn with different key should fail at concurrency limit");

    Ok(())
}
