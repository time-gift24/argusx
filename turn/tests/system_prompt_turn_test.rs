mod support;

use std::sync::Arc;

use turn::{TurnDriver, TurnMessage, TurnSeed};

#[tokio::test(flavor = "current_thread")]
async fn turn_seed_system_prompt_reaches_model_but_not_transcript() {
    let model = Arc::new(support::text_only_model(["done"]));
    let model_ref = Arc::clone(&model);
    let seed = TurnSeed {
        session_id: "session-1".into(),
        turn_id: "turn-1".into(),
        prior_messages: vec![],
        user_message: "hello".into(),
        system_prompt: Some("You are a planner.".into()),
    };

    let (_handle, task) = TurnDriver::spawn(
        seed,
        model,
        Arc::new(support::FakeToolRunner::default()),
        Arc::new(support::FakeAuthorizer::default()),
        Arc::new(support::FakeObserver),
    );

    let outcome = task.await.unwrap().unwrap();
    let requests = model_ref.received_requests().await;

    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].system_prompt.as_deref(),
        Some("You are a planner.")
    );
    assert!(
        !outcome
            .transcript
            .iter()
            .any(|msg| matches!(msg, TurnMessage::SystemNote { .. }))
    );
}
