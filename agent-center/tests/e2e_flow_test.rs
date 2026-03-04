use agent_center::api::center::{CloseRequest, SpawnRequest, WaitMode, WaitRequest};
use agent_center::AgentCenter;
use tempfile::tempdir;

#[tokio::test]
async fn e2e_spawn_wait_close_flow() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let db_path = temp.path().join("test.db");

    let center = AgentCenter::builder()
        .max_concurrent(10)
        .max_depth(3)
        .db_path(db_path)
        .build()?;

    // Step 1: Spawn child agent
    let spawn_req = SpawnRequest {
        parent_thread_id: "root".to_string(),
        key: "child-1".to_string(),
        agent_name: "test-agent".to_string(),
        initial_input: "Hello from parent".to_string(),
    };
    let spawn_resp = center.spawn(spawn_req).await?;
    let thread_id = spawn_resp.thread_id;
    assert!(
        !thread_id.is_empty(),
        "spawned thread should have non-empty ID"
    );
    assert_eq!(spawn_resp.status, "Running");
    assert_eq!(spawn_resp.agent_name, "test-agent");

    // Step 2: Wait for thread (should timeout since thread is still Running)
    let wait_req = WaitRequest {
        thread_ids: vec![thread_id.clone()],
        mode: WaitMode::Any,
        timeout_ms: 100, // Short timeout
    };
    let wait_resp = center.wait(wait_req).await?;
    assert!(
        wait_resp.timed_out,
        "wait should timeout for running thread"
    );

    // Step 3: Close the thread
    let close_req = CloseRequest {
        thread_id: thread_id.clone(),
        force: false,
    };
    let close_resp = center.close(close_req).await?;
    assert_eq!(close_resp.final_status, "Closed", "thread should be closed");

    // Step 4: Wait for closed thread (should succeed immediately)
    let wait_req2 = WaitRequest {
        thread_ids: vec![thread_id.clone()],
        mode: WaitMode::All,
        timeout_ms: 1000,
    };
    let wait_resp2 = center.wait(wait_req2).await?;
    assert!(
        !wait_resp2.timed_out,
        "wait should not timeout for closed thread"
    );
    assert_eq!(
        wait_resp2.statuses.get(&thread_id),
        Some(&"Closed".to_string()),
        "thread status should be Closed"
    );
    assert_eq!(
        wait_resp2
            .snapshots
            .get(&thread_id)
            .map(|snapshot| snapshot.status.as_str()),
        Some("Closed")
    );

    // Step 5: Verify close is idempotent
    let close_req2 = CloseRequest {
        thread_id: thread_id.clone(),
        force: false,
    };
    let close_resp2 = center.close(close_req2).await?;
    assert_eq!(
        close_resp2.final_status, "Closed",
        "close should be idempotent"
    );

    Ok(())
}
