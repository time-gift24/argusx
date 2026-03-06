use tokio_util::sync::CancellationToken;
use tool::ToolContext;

#[test]
fn tool_context_carries_runtime_cancellation() {
    let ctx = ToolContext::new("session-1", "turn-1", CancellationToken::new());
    assert_eq!(ctx.session_id, "session-1");
    assert_eq!(ctx.turn_id, "turn-1");
    assert!(!ctx.cancel_token.is_cancelled());
}
