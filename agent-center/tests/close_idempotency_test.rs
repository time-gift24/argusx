use agent_center::AgentCenter;
use tempfile::tempdir;

#[tokio::test]
async fn close_is_idempotent() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let db_path = temp.path().join("test.db");

    let center = AgentCenter::builder()
        .max_concurrent(10)
        .max_depth(3)
        .db_path(db_path.clone())
        .build()?;

    // Spawn a thread
    let spawn_req = agent_center::api::center::SpawnRequest {
        parent_thread_id: "p1".to_string(),
        key: "k1".to_string(),
        agent_name: "test-agent".to_string(),
        initial_input: "Hello".to_string(),
    };
    let spawn_resp = center.spawn(spawn_req).await?;
    let thread_id = spawn_resp.thread_id;

    // Close once
    let close_req1 = agent_center::api::center::CloseRequest {
        thread_id: thread_id.clone(),
        force: false,
    };
    let close_resp1 = center.close(close_req1).await?;
    assert_eq!(close_resp1.final_status, "Closed");

    // Close again (idempotent) - should return same state
    let close_req2 = agent_center::api::center::CloseRequest {
        thread_id: thread_id.clone(),
        force: false,
    };
    let close_resp2 = center.close(close_req2).await?;
    assert_eq!(close_resp2.final_status, "Closed");
    assert_eq!(close_resp1.final_status, close_resp2.final_status);

    Ok(())
}

#[tokio::test]
async fn close_transitions_through_closing_state() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let db_path = temp.path().join("test.db");

    let center = AgentCenter::builder()
        .db_path(db_path)
        .build()?;

    // Spawn a thread
    let spawn_req = agent_center::api::center::SpawnRequest {
        parent_thread_id: "p1".to_string(),
        key: "k1".to_string(),
        agent_name: "test-agent".to_string(),
        initial_input: "Hello".to_string(),
    };
    let spawn_resp = center.spawn(spawn_req).await?;
    let thread_id = spawn_resp.thread_id;

    // Close should transition Running -> Closing -> Closed
    let close_req = agent_center::api::center::CloseRequest {
        thread_id: thread_id.clone(),
        force: false,
    };
    let close_resp = center.close(close_req).await?;

    // Verify final state is Closed
    assert_eq!(close_resp.final_status, "Closed");

    // Close again to verify idempotency
    let close_req2 = agent_center::api::center::CloseRequest {
        thread_id,
        force: false,
    };
    let close_resp2 = center.close(close_req2).await?;
    assert_eq!(close_resp2.final_status, "Closed");

    Ok(())
}
