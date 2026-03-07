use desktop_lib::chat::{TurnTargetKind, observer::map_turn_event};

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
