use std::sync::Arc;

use argus_core::{Builtin, BuiltinToolCall, ToolCall};
use desktop_lib::chat::{
    map_turn_event, plan_updated_event, DesktopTurnEvent, DesktopTurnEventMapper, TurnTargetKind,
};
use turn::{ToolOutcome, TurnEvent};

#[test]
fn desktop_turn_event_serializes_type_field() {
    let event = DesktopTurnEvent {
        turn_id: "turn-1".into(),
        event_type: "turn-started".into(),
        data: serde_json::json!({
            "targetKind": TurnTargetKind::Agent,
            "targetId": "sre-agent",
        }),
    };

    let value = serde_json::to_value(event).unwrap();

    assert_eq!(value["type"], "turn-started");
    assert_eq!(value["turnId"], "turn-1");
    assert_eq!(value["data"]["targetKind"], "agent");
    assert_eq!(value["data"]["targetId"], "sre-agent");
}

#[test]
fn map_turn_event_includes_prepared_tool_metadata() {
    let event = map_turn_event(
        "turn-1",
        &TurnEvent::ToolCallPrepared {
            call: Arc::new(ToolCall::Builtin(BuiltinToolCall {
                sequence: 0,
                call_id: "call-1".into(),
                builtin: Builtin::Glob,
                arguments_json: r#"{"path":"."}"#.into(),
            })),
        },
    )
    .expect("tool-call-prepared should map");

    assert_eq!(event.turn_id, "turn-1");
    assert_eq!(event.event_type, "tool-call-prepared");
    assert_eq!(event.data["callId"], "call-1");
    assert_eq!(event.data["name"], "glob");
    assert_eq!(event.data["argumentsJson"], r#"{"path":"."}"#);
}

#[test]
fn plan_updated_event_uses_expected_shape_for_update_plan_success() {
    let event = plan_updated_event(
        "turn-1",
        "call-1",
        &ToolOutcome::Success(serde_json::json!({
            "plan": {
                "title": "Execution Plan",
                "description": "Starting execution",
                "tasks": [
                    {
                        "id": "task-1",
                        "title": "Write failing test",
                        "status": "in_progress"
                    }
                ],
                "is_streaming": true
            }
        })),
    )
    .expect("valid update_plan result should map");

    assert_eq!(event.turn_id, "turn-1");
    assert_eq!(event.event_type, "plan-updated");
    assert_eq!(event.data["sourceCallId"], "call-1");
    assert_eq!(event.data["title"], "Execution Plan");
    assert_eq!(event.data["tasks"][0]["title"], "Write failing test");
    assert_eq!(event.data["tasks"][0]["status"], "in_progress");
}

#[test]
fn turn_event_mapper_emits_plan_updated_after_update_plan_completion() {
    let mut mapper = DesktopTurnEventMapper::default();
    let prepared_events = mapper.map_event(
        "turn-1",
        &TurnEvent::ToolCallPrepared {
            call: Arc::new(ToolCall::Builtin(BuiltinToolCall {
                sequence: 0,
                call_id: "call-update-plan".into(),
                builtin: Builtin::UpdatePlan,
                arguments_json: r#"{"plan":[{"step":"Write failing test","status":"completed"}]}"#
                    .into(),
            })),
        },
    );

    assert_eq!(prepared_events.len(), 1);
    assert_eq!(prepared_events[0].event_type, "tool-call-prepared");

    let completed_events = mapper.map_event(
        "turn-1",
        &TurnEvent::ToolCallCompleted {
            call_id: "call-update-plan".into(),
            result: ToolOutcome::Success(serde_json::json!({
                "plan": {
                    "title": "Execution Plan",
                    "tasks": [
                        {
                            "id": "task-1",
                            "title": "Write failing test",
                            "status": "completed"
                        }
                    ],
                    "is_streaming": false
                }
            })),
        },
    );

    assert_eq!(completed_events.len(), 2);
    assert_eq!(completed_events[0].event_type, "tool-call-completed");
    assert_eq!(completed_events[0].data["callId"], "call-update-plan");
    assert_eq!(completed_events[0].data["result"]["status"], "success");
    assert_eq!(
        completed_events[1].event_type, "plan-updated",
        "update_plan completions should continue to produce plan-updated for the chat UI"
    );
    assert_eq!(completed_events[1].data["sourceCallId"], "call-update-plan");
}
