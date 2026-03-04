use agent_center::api::center::SpawnRequest;
use agent_center::AgentCenter;
use tempfile::tempdir;
use std::sync::Arc;

#[tokio::test]
async fn spawn_enforces_depth_limit() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let db_path = temp.path().join("test.db");

    let center = AgentCenter::builder()
        .max_concurrent(10)
        .max_depth(2) // Max depth of 2
        .db_path(db_path)
        .build()?;

    // Spawn root thread (depth 0)
    let spawn_req1 = SpawnRequest {
        parent_thread_id: "root".to_string(),
        key: "child-1".to_string(),
        agent_name: "test-agent".to_string(),
        initial_input: "Hello".to_string(),
    };
    let resp1 = center.spawn(spawn_req1).await?;
    let depth1_id = resp1.thread_id;

    // Spawn depth 1 thread (child of depth1_id)
    let spawn_req2 = SpawnRequest {
        parent_thread_id: depth1_id.clone(),
        key: "child-2".to_string(),
        agent_name: "test-agent".to_string(),
        initial_input: "Hello".to_string(),
    };
    let resp2 = center.spawn(spawn_req2).await?;
    let depth2_id = resp2.thread_id;

    // Try to spawn depth 2 thread - should fail (exceeds max_depth=2)
    let spawn_req3 = SpawnRequest {
        parent_thread_id: depth2_id.clone(),
        key: "child-3".to_string(),
        agent_name: "test-agent".to_string(),
        initial_input: "Hello".to_string(),
    };
    let result = center.spawn(spawn_req3).await;
    assert!(result.is_err(), "should fail when depth limit exceeded");

    Ok(())
}

#[tokio::test]
async fn concurrent_spawn_with_same_key_is_idempotent() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let db_path = temp.path().join("test.db");

    let center = Arc::new(
        AgentCenter::builder()
            .db_path(db_path)
            .max_concurrent(10)
            .build()?
    );

    let parent_id = "parent-1".to_string();
    let key = "child-1".to_string();

    // Spawn 10 concurrent requests with same (parent, key)
    let mut handles = vec![];
    for _ in 0..10 {
        let center_clone = Arc::clone(&center);
        let parent_id = parent_id.clone();
        let key = key.clone();
        let handle = tokio::spawn(async move {
            let req = SpawnRequest {
                parent_thread_id: parent_id,
                key,
                agent_name: "test-agent".to_string(),
                initial_input: "Hello".to_string(),
            };
            center_clone.spawn(req).await
        });
        handles.push(handle);
    }

    // Collect results
    let mut thread_ids = std::collections::HashSet::new();
    for handle in handles {
        let result = handle.await??;
        thread_ids.insert(result.thread_id);
    }

    // All concurrent spawns should return same thread ID (idempotent)
    assert_eq!(thread_ids.len(), 1, "all concurrent spawns should return same thread ID");

    Ok(())
}
