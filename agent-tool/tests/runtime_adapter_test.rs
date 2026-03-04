use agent_core::tools::{ToolExecutionContext, ToolExecutionErrorKind, ToolExecutor};
use agent_tool::AgentToolRuntime;

#[tokio::test]
async fn default_builtins_expose_update_plan_tool_spec() {
    use agent_core::tools::ToolCatalog;
    let rt = AgentToolRuntime::default_with_builtins().await;

    let spec = rt
        .tool_spec("update_plan")
        .await
        .expect("update_plan should be registered by default");

    assert_eq!(spec.name, "update_plan");
    assert!(spec.description.contains("plan"));
    assert_eq!(spec.input_schema["type"], serde_json::json!("object"));
    assert_eq!(spec.input_schema["required"], serde_json::json!(["plan"]));
}

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

#[tokio::test]
async fn unknown_tool_maps_to_user_error_kind() {
    let rt = AgentToolRuntime::default_with_builtins().await;
    let err = rt
        .execute_tool(
            agent_core::ToolCall::new("unknown_tool", serde_json::json!({})),
            ToolExecutionContext {
                session_id: "s1".into(),
                turn_id: "t1".into(),
                epoch: 0,
                cwd: None,
            },
        )
        .await
        .expect_err("unknown tool should fail");

    assert!(matches!(err.kind, ToolExecutionErrorKind::User));
}
