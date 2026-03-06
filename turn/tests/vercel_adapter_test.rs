use argus_core::{Builtin, BuiltinToolCall, ToolCall};
use serde_json::json;
use turn::{
    PermissionDecision, PermissionRequest, StepFinishReason, ToolOutcome, TurnEvent,
    TurnFinishReason,
};

#[test]
fn vercel_adapter_maps_permission_and_tool_events() {
    let lines = turn::vercel::map_events(vec![
        TurnEvent::ToolCallPermissionRequested {
            request: PermissionRequest {
                request_id: "perm-1".into(),
                tool_call_id: "call-1".into(),
            },
        },
        TurnEvent::ToolCallPrepared {
            call: ToolCall::Builtin(BuiltinToolCall {
                sequence: 0,
                call_id: "call-1".into(),
                builtin: Builtin::Read,
                arguments_json: "{}".into(),
            }),
        },
        TurnEvent::ToolCallCompleted {
            call_id: "call-1".into(),
            result: ToolOutcome::Success(json!({"ok": true})),
        },
        TurnEvent::ToolCallPermissionResolved {
            request_id: "perm-1".into(),
            decision: PermissionDecision::Allow,
        },
        TurnEvent::StepFinished {
            step_index: 0,
            reason: StepFinishReason::ToolCalls,
        },
        TurnEvent::TurnFinished {
            reason: TurnFinishReason::Completed,
        },
    ]);

    assert!(lines.iter().any(|line| line.starts_with("1:")));
    assert!(lines.iter().any(|line| line.starts_with("7:")));
    assert!(lines.iter().any(|line| line.starts_with("8:")));
    assert!(lines.iter().any(|line| line.starts_with("e:")));
    assert!(lines.iter().any(|line| line.starts_with("d:")));
}
