use agent_core::tools::{ToolExecutionContext, ToolExecutionErrorKind, ToolExecutor, ToolCatalog};
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
async fn default_builtins_list_includes_update_plan() {
    use agent_core::tools::ToolCatalog;
    let rt = AgentToolRuntime::default_with_builtins().await;

    let tools = rt.list_tools().await;
    assert!(tools.iter().any(|t| t.name == "update_plan"));
}

#[tokio::test]
async fn runtime_adapter_executes_update_plan_tool() {
    use agent_core::tools::{ToolExecutionContext, ToolExecutor};
    let rt = AgentToolRuntime::default_with_builtins().await;
    let out = rt
        .execute_tool(
            agent_core::ToolCall::new(
                "update_plan",
                serde_json::json!({
                    "plan": [{ "step": "Write tests", "status": "in_progress" }]
                }),
            ),
            ToolExecutionContext {
                session_id: "s1".into(),
                turn_id: "t1".into(),
                epoch: 0,
                cwd: None,
            },
        )
        .await
        .expect("update_plan should execute");

    assert!(!out.is_error);
    assert_eq!(out.output["plan"]["tasks"][0]["title"], "Write tests");
}

#[tokio::test]
async fn runtime_adapter_executes_registered_tool() {
    let rt = AgentToolRuntime::default_with_builtins().await;

    // Read a file from current working directory (which is the default allowed root)
    let out = rt
        .execute_tool(
            agent_core::ToolCall::new("read", serde_json::json!({"path": "Cargo.toml", "mode": "text"})),
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

#[tokio::test]
async fn default_builtins_expose_only_read_glob_grep() {
    let rt = AgentToolRuntime::default_with_builtins().await;
    let mut names = rt
        .list_tools()
        .await
        .into_iter()
        .map(|t| t.name)
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(names, vec!["glob", "grep", "read"]);
}
