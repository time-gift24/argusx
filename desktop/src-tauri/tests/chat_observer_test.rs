use desktop_lib::chat::observer::map_turn_event;
use serde_json::json;
use turn::{TurnEvent, ToolOutcome};

#[test]
fn llm_text_delta_maps_to_desktop_event_with_ids() {
    let event = map_turn_event(
        "conversation-1",
        "turn-1",
        &TurnEvent::LlmTextDelta {
            text: "hello".into(),
        },
    )
    .unwrap();

    assert_eq!(event.conversation_id, "conversation-1");
    assert_eq!(event.turn_id, "turn-1");
    assert_eq!(event.event_type, "llm-text-delta");
    assert_eq!(event.data["text"], "hello");
}

#[test]
fn tool_completion_maps_result_payload() {
    let event = map_turn_event(
        "conversation-1",
        "turn-1",
        &TurnEvent::ToolCallCompleted {
            call_id: "call-1".into(),
            result: ToolOutcome::Success(json!({
                "path": "README.md",
                "bytes": 42,
            })),
        },
    )
    .unwrap();

    assert_eq!(event.event_type, "tool-call-completed");
    assert_eq!(event.data["callId"], "call-1");
    assert_eq!(event.data["status"], "success");
    assert_eq!(event.data["output"]["path"], "README.md");
    assert_eq!(event.data["output"]["bytes"], 42);
}
