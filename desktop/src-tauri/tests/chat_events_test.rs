use desktop_lib::chat::{
    CancelConversationInput, ContinueConversationInput, DesktopTurnEvent, StartConversationInput,
    TurnTargetKind,
};

#[test]
fn conversation_inputs_serialize_with_camel_case_fields() {
    let start = serde_json::to_value(StartConversationInput {
        prompt: "hello".into(),
        target_kind: TurnTargetKind::Agent,
        target_id: "reviewer".into(),
    })
    .unwrap();
    let continue_payload = serde_json::to_value(ContinueConversationInput {
        conversation_id: "conversation-1".into(),
        prompt: "continue".into(),
    })
    .unwrap();
    let cancel = serde_json::to_value(CancelConversationInput {
        conversation_id: "conversation-1".into(),
    })
    .unwrap();

    assert_eq!(start["targetKind"], "agent");
    assert_eq!(start["targetId"], "reviewer");
    assert_eq!(continue_payload["conversationId"], "conversation-1");
    assert_eq!(cancel["conversationId"], "conversation-1");
}

#[test]
fn desktop_turn_event_serializes_turn_and_conversation_ids() {
    let event = DesktopTurnEvent::text_delta("conversation-1", "turn-1", "hello");
    let value = serde_json::to_value(event).unwrap();

    assert_eq!(value["conversationId"], "conversation-1");
    assert_eq!(value["turnId"], "turn-1");
    assert_eq!(value["type"], "llm-text-delta");
    assert_eq!(value["data"]["text"], "hello");
}
