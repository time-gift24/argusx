use std::sync::Arc;

use desktop_lib::chat::{
    storage::{ConversationRecord, InMemoryConversationRepository, StoredConversationMessage},
    ConversationRepository, TurnTargetKind,
};

#[test]
fn in_memory_repository_saves_loads_and_lists_conversations() {
    let repository = InMemoryConversationRepository::default();

    repository
        .save(ConversationRecord {
            conversation_id: "conversation-1".into(),
            target_kind: TurnTargetKind::Agent,
            target_id: "reviewer".into(),
            history: vec![
                StoredConversationMessage::User {
                    content: "hello".into(),
                },
                StoredConversationMessage::AssistantText {
                    content: "hi".into(),
                },
            ],
            updated_at_ms: 100,
        })
        .unwrap();
    repository
        .save(ConversationRecord {
            conversation_id: "conversation-2".into(),
            target_kind: TurnTargetKind::Workflow,
            target_id: "design".into(),
            history: vec![StoredConversationMessage::User {
                content: "plan this".into(),
            }],
            updated_at_ms: 200,
        })
        .unwrap();

    let loaded = repository.load("conversation-1").unwrap().unwrap();
    let listed = repository.list().unwrap();

    assert_eq!(loaded.target_id, "reviewer");
    assert_eq!(loaded.history.len(), 2);
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].conversation_id, "conversation-2");
    assert_eq!(listed[1].conversation_id, "conversation-1");
}

#[test]
fn conversation_record_converts_turn_snapshots_into_owned_storage_messages() {
    let snapshot: Arc<[Arc<turn::TurnMessage>]> = Arc::from([
        Arc::new(turn::TurnMessage::User {
            content: "hello".into(),
        }),
        Arc::new(turn::TurnMessage::AssistantText {
            content: "hi".into(),
        }),
    ]);

    let record = ConversationRecord::from_snapshot(
        "conversation-1",
        TurnTargetKind::Agent,
        "reviewer",
        snapshot.clone(),
        100,
    );

    let roundtrip = record.to_snapshot();

    assert_eq!(record.history.len(), 2);
    assert_eq!(roundtrip.len(), snapshot.len());
    assert!(matches!(
        roundtrip[0].as_ref(),
        turn::TurnMessage::User { content } if content.as_ref() == "hello"
    ));
    assert!(matches!(
        roundtrip[1].as_ref(),
        turn::TurnMessage::AssistantText { content } if content.as_ref() == "hi"
    ));
}
