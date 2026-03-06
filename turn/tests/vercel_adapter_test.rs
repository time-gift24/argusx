use argus_core::{Builtin, BuiltinToolCall, ToolCall};
use serde_json::{Value, json};
use turn::{
    PermissionDecision, PermissionRequest, StepFinishReason, ToolOutcome, TurnEvent,
    TurnFinishReason,
};

#[test]
fn ui_message_stream_adapter_maps_permission_and_tool_events() {
    let lines = turn::vercel::map_events(vec![
        TurnEvent::TurnStarted,
        TurnEvent::ToolCallPrepared {
            call: ToolCall::Builtin(BuiltinToolCall {
                sequence: 0,
                call_id: "call-1".into(),
                builtin: Builtin::Read,
                arguments_json: "{}".into(),
            }),
        },
        TurnEvent::ToolCallPermissionRequested {
            request: PermissionRequest {
                request_id: "perm-1".into(),
                tool_call_id: "call-1".into(),
            },
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

    let chunks = parse_chunks(&lines);

    assert_eq!(chunks[0]["type"], "start");
    assert_eq!(chunks[1]["type"], "start-step");
    assert_eq!(chunks[2]["type"], "tool-input-available");
    assert_eq!(chunks[2]["toolCallId"], "call-1");
    assert_eq!(chunks[3]["type"], "tool-approval-request");
    assert_eq!(chunks[3]["approvalId"], "perm-1");
    assert_eq!(chunks[4]["type"], "tool-output-available");
    assert_eq!(chunks[5]["type"], "data-turn-control");
    assert_eq!(chunks[6]["type"], "finish-step");
    assert_eq!(chunks[7]["type"], "finish");
    assert_eq!(chunks[7]["finishReason"], "stop");
    assert_eq!(lines.last().unwrap(), "data: [DONE]\n\n");
}

#[test]
fn ui_message_stream_adapter_wraps_text_and_reasoning_parts() {
    let lines = turn::vercel::map_events(vec![
        TurnEvent::TurnStarted,
        TurnEvent::LlmReasoningDelta {
            text: "think".into(),
        },
        TurnEvent::LlmTextDelta { text: "hel".into() },
        TurnEvent::LlmTextDelta { text: "lo".into() },
        TurnEvent::StepFinished {
            step_index: 0,
            reason: StepFinishReason::ToolCalls,
        },
        TurnEvent::TurnFinished {
            reason: TurnFinishReason::Cancelled,
        },
    ]);

    let chunks = parse_chunks(&lines);

    assert_eq!(chunks[0]["type"], "start");
    assert_eq!(chunks[1]["type"], "start-step");
    assert_eq!(chunks[2]["type"], "reasoning-start");
    assert_eq!(chunks[3]["type"], "reasoning-delta");
    assert_eq!(chunks[4]["type"], "text-start");
    assert_eq!(chunks[5]["type"], "text-delta");
    assert_eq!(chunks[5]["delta"], "hel");
    assert_eq!(chunks[6]["type"], "text-delta");
    assert_eq!(chunks[6]["delta"], "lo");
    assert_eq!(chunks[7]["type"], "text-end");
    assert_eq!(chunks[8]["type"], "reasoning-end");
    assert_eq!(chunks[9]["type"], "finish-step");
    assert_eq!(chunks[10]["type"], "data-turn-control");
    assert_eq!(chunks[11]["type"], "finish");
    assert_eq!(chunks[11]["finishReason"], "other");
    assert_eq!(lines.last().unwrap(), "data: [DONE]\n\n");
}

fn parse_chunks(lines: &[String]) -> Vec<Value> {
    lines[..lines.len() - 1]
        .iter()
        .map(|line| {
            let json = line
                .strip_prefix("data: ")
                .and_then(|line| line.strip_suffix("\n\n"))
                .expect("expected SSE data chunk");
            serde_json::from_str(json).expect("chunk should be valid json")
        })
        .collect()
}
