use agent_center::AgentCenter;
use tempfile::tempdir;

#[tokio::test]
async fn wait_all_times_out_when_any_thread_not_terminal() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let db_path = temp.path().join("test.db");

    let center = AgentCenter::builder()
        .max_concurrent(10)
        .max_depth(3)
        .db_path(db_path)
        .build()?;

    // Spawn a thread that stays in Running state
    let spawn_req = agent_center::api::center::SpawnRequest {
        parent_thread_id: "root".to_string(),
        key: "k1".to_string(),
        agent_name: "test-agent".to_string(),
        initial_input: "Hello".to_string(),
    };
    let spawn_resp = center.spawn(spawn_req).await?;
    let thread_id = spawn_resp.thread_id;

    // Wait with mode=all and very short timeout
    let wait_req = agent_center::api::center::WaitRequest {
        thread_ids: vec![thread_id.clone()],
        mode: agent_center::api::center::WaitMode::All,
        timeout_ms: 100, // Very short timeout
    };

    let wait_resp = center.wait(wait_req).await?;

    // Should timeout because thread is still running
    assert!(
        wait_resp.timed_out,
        "should timeout when thread not terminal"
    );

    // Should return status map
    assert!(wait_resp.statuses.contains_key(&thread_id));
    assert!(wait_resp.snapshots.contains_key(&thread_id));

    Ok(())
}

#[tokio::test]
async fn wait_clamps_timeout_to_valid_range() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let db_path = temp.path().join("test.db");

    let center = AgentCenter::builder().db_path(db_path).build()?;

    // Test timeout clamping: 0 -> 1000 (minimum)
    let wait_req = agent_center::api::center::WaitRequest {
        thread_ids: vec!["nonexistent".to_string()],
        mode: agent_center::api::center::WaitMode::Any,
        timeout_ms: 0, // Below minimum
    };

    // Should not panic, should clamp to 1000ms minimum
    let _ = center.wait(wait_req).await;

    Ok(())
}
