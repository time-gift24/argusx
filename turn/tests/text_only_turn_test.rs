mod support;

use std::sync::Arc;

use turn::{TurnContext, TurnDriver, TurnEvent, TurnFinishReason};

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
        TurnEvent::LlmTextDelta { text } if text == "hel"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        TurnEvent::TurnFinished {
            reason: TurnFinishReason::Completed
        }
    )));
}
