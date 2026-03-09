use argus_core::{Builtin, BuiltinToolCall, ToolCall};
use tokio_util::sync::CancellationToken;
use turn::{AuthorizationDecision, ToolAuthorizer, ToolRunner};

#[tokio::test(flavor = "current_thread")]
async fn scheduled_tool_runner_executes_read_only_builtin() {
    let surface = desktop_lib::chat::build_agent_tool_surface(serde_json::json!({
        "builtins": ["glob"]
    }))
    .unwrap();
    let runner = desktop_lib::chat::ScheduledToolRunner::from_tool_surface(&surface).unwrap();
    let call = ToolCall::Builtin(BuiltinToolCall {
        sequence: 0,
        call_id: "call-1".into(),
        builtin: Builtin::Glob,
        arguments_json: r#"{"path":".","pattern":"*.toml","max_results":5}"#.into(),
    });
    let ctx = tool::ToolContext::new("session-1", "turn-1", CancellationToken::new());

    let output = runner.execute(call, ctx).await.unwrap();

    assert!(output.output.is_object());
}

#[tokio::test(flavor = "current_thread")]
async fn allowlisted_authorizer_allows_low_risk_tools_and_prompts_for_others() {
    let surface = desktop_lib::chat::build_agent_tool_surface(serde_json::json!({
        "builtins": ["read", "update_plan", "dispatch_subagent"]
    }))
    .unwrap();
    let authorizer = desktop_lib::chat::AllowListedToolAuthorizer::new(surface);

    let read = ToolCall::Builtin(BuiltinToolCall {
        sequence: 0,
        call_id: "call-read".into(),
        builtin: Builtin::Read,
        arguments_json: "{}".into(),
    });
    let update_plan = ToolCall::Builtin(BuiltinToolCall {
        sequence: 1,
        call_id: "call-update-plan".into(),
        builtin: Builtin::UpdatePlan,
        arguments_json: r#"{"plan":[{"step":"Review","status":"in_progress"}]}"#.into(),
    });
    let shell = ToolCall::Builtin(BuiltinToolCall {
        sequence: 2,
        call_id: "call-shell".into(),
        builtin: Builtin::Shell,
        arguments_json: "{}".into(),
    });
    let function = ToolCall::FunctionCall {
        sequence: 3,
        call_id: "call-function".into(),
        name: "custom_tool".into(),
        arguments_json: "{}".into(),
    };

    assert!(matches!(
        authorizer.authorize(&read).await.unwrap(),
        AuthorizationDecision::Allow
    ));
    assert!(matches!(
        authorizer.authorize(&update_plan).await.unwrap(),
        AuthorizationDecision::Allow
    ));
    assert!(matches!(
        authorizer.authorize(&shell).await.unwrap(),
        AuthorizationDecision::Ask(request)
            if request.request_id == "perm-call-shell"
                && request.tool_call_id == "call-shell"
    ));
    assert!(matches!(
        authorizer.authorize(&function).await.unwrap(),
        AuthorizationDecision::Ask(request)
            if request.request_id == "perm-call-function"
                && request.tool_call_id == "call-function"
    ));
}

#[test]
fn agent_tool_surface_only_registers_allowed_builtins() {
    let surface = desktop_lib::chat::build_agent_tool_surface(serde_json::json!({
        "builtins": ["read", "update_plan", "dispatch_subagent"]
    }))
    .unwrap();

    assert!(surface.has_builtin("dispatch_subagent"));
    assert!(!surface.has_builtin("shell"));
}
