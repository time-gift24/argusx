use desktop_lib::chat::{DesktopTurnEvent, TurnTargetKind};

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
