use argus_core::{Builtin, BuiltinToolCall, ToolCall};
use tokio_util::sync::CancellationToken;
use turn::{AuthorizationDecision, ToolAuthorizer, ToolRunner};

#[tokio::test(flavor = "current_thread")]
async fn scheduled_tool_runner_executes_read_only_builtin() {
    let runner = desktop_lib::chat::ScheduledToolRunner::from_current_dir().unwrap();
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
async fn allowlisted_authorizer_only_allows_read_only_builtins() {
    let authorizer = desktop_lib::chat::AllowListedToolAuthorizer;

    let read = ToolCall::Builtin(BuiltinToolCall {
        sequence: 0,
        call_id: "call-read".into(),
        builtin: Builtin::Read,
        arguments_json: "{}".into(),
    });
    let shell = ToolCall::Builtin(BuiltinToolCall {
        sequence: 1,
        call_id: "call-shell".into(),
        builtin: Builtin::Shell,
        arguments_json: "{}".into(),
    });

    assert!(matches!(
        authorizer.authorize(&read).await.unwrap(),
        AuthorizationDecision::Allow
    ));
    assert!(matches!(
        authorizer.authorize(&shell).await.unwrap(),
        AuthorizationDecision::Deny
    ));
}
