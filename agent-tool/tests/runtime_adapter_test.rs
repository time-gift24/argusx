use agent_core::tools::{ToolExecutionContext, ToolExecutor};
use agent_tool::AgentToolRuntime;

#[tokio::test]
async fn runtime_adapter_executes_registered_tool() {
    let rt = AgentToolRuntime::default_with_builtins().await;
    let out = rt
        .execute_tool(
            agent_core::ToolCall::new("shell", serde_json::json!({"command": "echo ok"})),
            ToolExecutionContext {
                session_id: "s1".into(),
                turn_id: "t1".into(),
                epoch: 0,
                cwd: None,
            },
        )
        .await
        .expect("tool should run");
    assert!(!out.is_error);
}
