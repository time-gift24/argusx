use agent_center::AgentCenter;
use tempfile::tempdir;

#[tokio::test]
async fn runtime_lists_agent_center_tools() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let db_path = temp.path().join("test.db");

    let center = AgentCenter::builder()
        .db_path(db_path)
        .build()?;

    // Get available tool names from the center
    let tools = center.list_tools();

    // Should include spawn_agent, wait, close_agent
    assert!(tools.contains(&"spawn_agent".to_string()), "should have spawn_agent tool");
    assert!(tools.contains(&"wait".to_string()), "should have wait tool");
    assert!(tools.contains(&"close_agent".to_string()), "should have close_agent tool");

    Ok(())
}
