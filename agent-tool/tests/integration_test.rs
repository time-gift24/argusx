use agent_tool::{ToolRegistry, ReadFileTool, ShellTool};

#[tokio::test]
async fn test_registry_register_and_list() {
    let registry = ToolRegistry::new();
    registry.register(ReadFileTool).await;
    registry.register(ShellTool).await;

    let tools = registry.list().await;
    assert_eq!(tools.len(), 2);
    assert!(tools.iter().any(|t| t.name == "read_file"));
    assert!(tools.iter().any(|t| t.name == "shell"));
}

#[tokio::test]
async fn test_registry_get_tool() {
    let registry = ToolRegistry::new();
    registry.register(ReadFileTool).await;

    let tool = registry.get("read_file").await;
    assert!(tool.is_some());
}
