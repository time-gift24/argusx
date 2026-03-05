use agent_center::AgentCenter;
use tempfile::tempdir;

#[tokio::test]
async fn spawn_is_idempotent_by_parent_and_key() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let db_path = temp.path().join("test.db");

    let center = AgentCenter::builder()
        .max_concurrent(10)
        .max_depth(3)
        .db_path(db_path.clone())
        .build()?;

    // First spawn with parent=p1, key=k1
    let req1 = agent_center::api::center::SpawnRequest {
        parent_thread_id: "root".to_string(),
        key: "k1".to_string(),
        agent_name: "test-agent".to_string(),
        initial_input: "Hello".to_string(),
    };

    let resp1 = center.spawn(req1).await?;
    let thread_id_1 = resp1.thread_id;

    // Second spawn with same parent+key should return same thread_id (idempotent)
    let req2 = agent_center::api::center::SpawnRequest {
        parent_thread_id: "root".to_string(),
        key: "k1".to_string(),
        agent_name: "test-agent".to_string(),
        initial_input: "Hello".to_string(),
    };

    let resp2 = center.spawn(req2).await?;
    assert_eq!(resp2.thread_id, thread_id_1, "spawn should be idempotent");

    Ok(())
}

#[tokio::test]
async fn spawn_respects_concurrency_limit() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let db_path = temp.path().join("test.db");

    let center = AgentCenter::builder()
        .max_concurrent(1)
        .max_depth(3)
        .db_path(db_path)
        .build()?;

    // First spawn succeeds
    let req1 = agent_center::api::center::SpawnRequest {
        parent_thread_id: "root".to_string(),
        key: "k1".to_string(),
        agent_name: "test-agent".to_string(),
        initial_input: "Hello".to_string(),
    };
    let _resp1 = center.spawn(req1).await?;

    // Second spawn with different key should fail due to concurrency limit
    let req2 = agent_center::api::center::SpawnRequest {
        parent_thread_id: "root".to_string(),
        key: "k2".to_string(),
        agent_name: "test-agent".to_string(),
        initial_input: "Hello".to_string(),
    };

    let result = center.spawn(req2).await;
    assert!(
        result.is_err(),
        "should fail when concurrency limit reached"
    );

    Ok(())
}
