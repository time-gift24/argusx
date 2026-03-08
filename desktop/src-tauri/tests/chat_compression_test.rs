use desktop_lib::chat::{
    compression::{
        ConversationCompressionPolicy, NoopConversationCompressionPolicy,
        ThresholdConversationCompressionPolicy,
    },
    storage::{ConversationRecord, StoredConversationMessage},
    TurnTargetKind,
};

#[test]
fn no_op_policy_keeps_history_unchanged() {
    let record = sample_record(vec![
        StoredConversationMessage::User {
            content: "hello".into(),
        },
        StoredConversationMessage::AssistantText {
            content: "hi".into(),
        },
    ]);

    let compressed = NoopConversationCompressionPolicy
        .compress(record.clone())
        .unwrap();

    assert_eq!(compressed.history, record.history);
}

#[test]
fn threshold_policy_replaces_old_history_with_summary_note() {
    let record = sample_record(vec![
        StoredConversationMessage::User {
            content: "1".into(),
        },
        StoredConversationMessage::AssistantText {
            content: "2".into(),
        },
        StoredConversationMessage::User {
            content: "3".into(),
        },
        StoredConversationMessage::AssistantText {
            content: "4".into(),
        },
        StoredConversationMessage::User {
            content: "5".into(),
        },
        StoredConversationMessage::AssistantText {
            content: "6".into(),
        },
    ]);
    let policy = ThresholdConversationCompressionPolicy::new(4, 2);

    let compressed = policy.compress(record).unwrap();

    assert_eq!(compressed.history.len(), 3);
    assert!(matches!(
        compressed.history[0],
        StoredConversationMessage::SystemNote { .. }
    ));
}

#[test]
fn threshold_policy_keeps_recent_messages_verbatim() {
    let record = sample_record(vec![
        StoredConversationMessage::User {
            content: "older".into(),
        },
        StoredConversationMessage::AssistantText {
            content: "reply".into(),
        },
        StoredConversationMessage::User {
            content: "latest-question".into(),
        },
        StoredConversationMessage::AssistantText {
            content: "latest-answer".into(),
        },
    ]);
    let policy = ThresholdConversationCompressionPolicy::new(3, 2);

    let compressed = policy.compress(record).unwrap();

    assert!(matches!(
        compressed.history[1],
        StoredConversationMessage::User { ref content } if content == "latest-question"
    ));
    assert!(matches!(
        compressed.history[2],
        StoredConversationMessage::AssistantText { ref content } if content == "latest-answer"
    ));
}

fn sample_record(history: Vec<StoredConversationMessage>) -> ConversationRecord {
    ConversationRecord {
        conversation_id: "conversation-1".into(),
        target_kind: TurnTargetKind::Agent,
        target_id: "reviewer".into(),
        history,
        updated_at_ms: 100,
    }
}
