use desktop_lib::chat::{
    observer::{map_turn_event, plan_updated_event},
    TurnTargetKind,
};
use turn::ToolOutcome;

#[test]
fn map_turn_started_to_desktop_event() {
    let event = map_turn_event(
        "turn-1",
        TurnTargetKind::Agent,
        "sre-agent",
        &turn::TurnEvent::TurnStarted,
    )
    .expect("turn-started should map");

    assert_eq!(event.turn_id, "turn-1");
    assert_eq!(event.event_type, "turn-started");
    assert_eq!(event.data["targetKind"], "agent");
    assert_eq!(event.data["targetId"], "sre-agent");
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
                    },
                    {
                        "id": "task-2",
                        "title": "Implement minimal fix",
                        "status": "pending"
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
fn plan_updated_event_returns_none_for_invalid_payload() {
    let event = plan_updated_event(
        "turn-1",
        "call-1",
        &ToolOutcome::Success(serde_json::json!({
            "plan": {
                "title": 42,
                "tasks": "invalid"
            }
        })),
    );

    assert!(event.is_none());
}
