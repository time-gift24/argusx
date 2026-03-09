mod support;

use std::sync::Arc;

use argus_core::{FinishReason, ResponseEvent, Usage};
use turn::{TurnDriver, TurnEvent, TurnFinishReason, TurnSeed};

fn expect_shared_text(_: &Arc<str>) {}

#[tokio::test]
async fn text_only_turn_streams_text_and_completes() {
    let context = TurnSeed {
        session_id: "session-1".into(),
        turn_id: "turn-1".into(),
        prior_messages: vec![],
        user_message: "hello".into(),
        system_prompt: None,
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
async fn completed_turn_returns_transcript_and_final_output() {
    let model = Arc::new(support::FakeModelRunner::new(vec![vec![
        ResponseEvent::ContentDelta("hello".into()),
        ResponseEvent::ContentDelta(" world".into()),
        ResponseEvent::Done {
            reason: FinishReason::Stop,
            usage: Some(Usage::zero()),
        },
    ]]));

    let (handle, task) = TurnDriver::spawn(
        TurnSeed {
            session_id: "session-1".into(),
            turn_id: "turn-1".into(),
            prior_messages: vec![],
            user_message: "say hello".into(),
            system_prompt: None,
        },
        model,
        Arc::new(support::FakeToolRunner::default()),
        Arc::new(support::FakeAuthorizer::default()),
        Arc::new(support::FakeObserver),
    );

    while handle.next_event().await.is_some() {}
    let outcome = task.await.unwrap().unwrap();

    assert_eq!(outcome.finish_reason, TurnFinishReason::Completed);
    assert_eq!(outcome.final_output.as_deref(), Some("hello world"));
    assert!(matches!(
        outcome.transcript.last(),
        Some(turn::TurnMessage::AssistantText { content }) if content.as_ref() == "hello world"
    ));
}
