#[tokio::test(flavor = "current_thread")]
async fn resolver_merges_session_prompt_builtin_role_and_thread_snapshot() {
    let snapshot = agent::ThreadAgentSnapshot {
        profile_id: "builtin-main".into(),
        display_name_snapshot: "Planner".into(),
        system_prompt_snapshot: "You are the main planner.".into(),
        tool_policy_snapshot_json: serde_json::json!({
            "builtins": ["read", "update_plan"]
        }),
        model_config_snapshot_json: serde_json::Value::Null,
        allow_subagent_dispatch_snapshot: false,
    };

    let resolved = agent::AgentExecutionResolver::new()
        .resolve("Session base prompt", &snapshot)
        .unwrap();

    assert!(resolved.system_prompt.contains("Session base prompt"));
    assert!(resolved.system_prompt.contains("You are the main planner."));
    assert!(resolved.system_prompt.contains("read, update_plan"));
    assert!(!resolved.system_prompt.contains("dispatch_subagent"));
    assert!(!resolved.allow_subagent_dispatch);
}
