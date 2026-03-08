mod support;

use std::sync::Arc;

use turn::{TurnContext, TurnDriver, TurnEvent, TurnFinishReason, TurnMessage, TurnOutcome};

fn expect_shared_text(_: &Arc<str>) {}

fn message_at(messages: &Arc<[Arc<TurnMessage>]>, index: usize) -> &TurnMessage {
    messages[index].as_ref()
}

#[tokio::test]
async fn text_only_turn_streams_text_and_completes() {
    let context = TurnContext {
        session_id: "session-1".into(),
        turn_id: "turn-1".into(),
        user_message: "hello".into(),
    };

    let (handle, task) = TurnDriver::spawn(
        context,
        Arc::new(support::text_only_model(["hel", "lo"])),
        Arc::new(support::FakeToolRunner::default()),
        Arc::new(support::FakeAuthorizer::default()),
        Arc::new(support::FakeObserver),
    );

    let mut events = Vec::new();
    while let Some(event) = handle.next_event().await {
        events.push(event);
    }

    task.await.unwrap().unwrap();

    assert!(events.iter().any(|event| matches!(
        event,
        TurnEvent::LlmTextDelta { text } if {
            expect_shared_text(text);
            text.as_ref() == "hel"
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        TurnEvent::TurnFinished {
            reason: TurnFinishReason::Completed
        }
    )));
}

#[tokio::test]
async fn recording_text_only_turn_returns_completed_transcript_artifact() {
    let context = TurnContext {
        session_id: "session-1".into(),
        turn_id: "turn-2".into(),
        user_message: "hello".into(),
    };

    let (handle, task) = TurnDriver::spawn_recording(
        context,
        Arc::from([]),
        Arc::new(support::text_only_model(["hel", "lo"])),
        Arc::new(support::FakeToolRunner::default()),
        Arc::new(support::FakeAuthorizer::default()),
        Arc::new(support::FakeObserver),
    );

    while handle.next_event().await.is_some() {}

    let outcome = task.await.unwrap().unwrap();

    match outcome {
        TurnOutcome::Completed(completed) => {
            assert_eq!(completed.turn_id, "turn-2");
            assert_eq!(completed.transcript.len(), 2);
            assert!(matches!(
                message_at(&completed.transcript, 0),
                TurnMessage::User { content } if content.as_ref() == "hello"
            ));
            assert!(matches!(
                message_at(&completed.transcript, 1),
                TurnMessage::AssistantText { content } if content.as_ref() == "hello"
            ));
        }
        other => panic!("expected completed outcome, got: {other:?}"),
    }
}
