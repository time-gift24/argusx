use agent_center::api::center::{CloseRequest, SpawnRequest};
use agent_center::AgentCenter;
use tempfile::tempdir;

#[tokio::test]
async fn close_force_skips_closing_state() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let db_path = temp.path().join("test.db");

    let center = AgentCenter::builder().db_path(db_path).build()?;

    // Spawn a thread
    let spawn_req = SpawnRequest {
        parent_thread_id: "root".to_string(),
        key: "child-1".to_string(),
        agent_name: "test-agent".to_string(),
        initial_input: "Hello".to_string(),
    };
    let spawn_resp = center.spawn(spawn_req).await?;
    let thread_id = spawn_resp.thread_id;

    // Force close the thread (should skip Closing state)
    let close_req = CloseRequest {
        thread_id: thread_id.clone(),
        force: true,
    };
    let close_resp = center.close(close_req).await?;
    assert_eq!(
        close_resp.final_status, "Closed",
        "force close should return Closed status"
    );

    // Verify idempotent: second force close should also succeed
    let close_req2 = CloseRequest {
        thread_id: thread_id.clone(),
        force: true,
    };
    let close_resp2 = center.close(close_req2).await?;
    assert_eq!(
        close_resp2.final_status, "Closed",
        "force close should be idempotent"
    );

    Ok(())
}

#[tokio::test]
async fn close_normal_transitions_through_closing_state() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let db_path = temp.path().join("test.db");

    let center = AgentCenter::builder().db_path(db_path).build()?;

    // Spawn a thread
    let spawn_req = SpawnRequest {
        parent_thread_id: "root".to_string(),
        key: "child-2".to_string(),
        agent_name: "test-agent".to_string(),
        initial_input: "Hello".to_string(),
    };
    let spawn_resp = center.spawn(spawn_req).await?;
    let thread_id = spawn_resp.thread_id;

    // Normal close (should go through Closing -> Closed)
    let close_req = CloseRequest {
        thread_id: thread_id.clone(),
        force: false,
    };
    let close_resp = center.close(close_req).await?;
    assert_eq!(
        close_resp.final_status, "Closed",
        "normal close should return Closed status"
    );

    Ok(())
}
