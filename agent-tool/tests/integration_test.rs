use std::time::{SystemTime, UNIX_EPOCH};

use agent_tool::{ReadFileTool, ShellTool, Tool, ToolContext, ToolRegistry, UpdatePlanTool};
use serde_json::json;

fn test_context() -> ToolContext {
    ToolContext {
        session_id: "test-session".to_string(),
        turn_id: "test-turn".to_string(),
    }
}

fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "agent-tool-{prefix}-{nanos}-{}",
        std::process::id()
    ))
}

#[tokio::test]
async fn test_registry_register_and_list() {
    let registry = ToolRegistry::new();
    registry.register(ReadFileTool).await;
    registry.register(ShellTool).await;
    registry.register(UpdatePlanTool).await;

    let tools = registry.list().await;
    assert_eq!(tools.len(), 3);
    assert!(tools.iter().any(|t| t.name == "read_file"));
    assert!(tools.iter().any(|t| t.name == "shell"));
    assert!(tools.iter().any(|t| t.name == "update_plan"));
}

#[tokio::test]
async fn test_registry_get_tool() {
    let registry = ToolRegistry::new();
    registry.register(ReadFileTool).await;

    let tool = registry.get("read_file").await;
    assert!(tool.is_some());
}

#[tokio::test]
async fn shell_should_mark_result_as_error_when_exit_code_non_zero() {
    let tool = ShellTool;

    let result = tool
        .execute(test_context(), json!({ "command": "exit 7" }))
        .await
        .expect("shell execution should succeed");

    assert!(result.is_error);
}

#[tokio::test]
async fn shell_should_honor_cwd_argument() {
    let tool = ShellTool;
    let cwd = unique_temp_dir("cwd");
    tokio::fs::create_dir_all(&cwd)
        .await
        .expect("temp dir should be created");
    tokio::fs::write(cwd.join("marker.txt"), "ok")
        .await
        .expect("marker file should be created");

    let result = tool
        .execute(
            test_context(),
            json!({
                "command": "test -f marker.txt && echo yes || echo no",
                "cwd": cwd,
            }),
        )
        .await
        .expect("shell execution should succeed");

    assert_eq!(result.output["stdout"], "yes\n");

    tokio::fs::remove_dir_all(cwd)
        .await
        .expect("temp dir should be removed");
}
