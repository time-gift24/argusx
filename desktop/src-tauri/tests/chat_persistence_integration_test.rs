use std::{collections::VecDeque, sync::Arc};

use async_trait::async_trait;
use desktop_lib::chat::{
    compression::NoopConversationCompressionPolicy,
    storage::InMemoryConversationRepository,
    ContinueConversationInput, ConversationManager, ConversationRepository, ConversationRuntime,
    ConversationTurnControl, RunningConversationTurn, StartConversationInput, TurnTargetKind,
};
use tokio::{
    sync::Mutex,
    time::{Duration, timeout},
};
use turn::{CompletedTurn, TurnContext, TurnError, TurnMessage, TurnOutcome};

#[tokio::test(flavor = "current_thread")]
async fn completed_turns_are_saved_and_rehydrated_for_future_managers() {
    let repository = Arc::new(InMemoryConversationRepository::default());
    let compression = Arc::new(NoopConversationCompressionPolicy);

    let runtime = Arc::new(FakeConversationRuntime::with_completed_turns(vec![completed_turn(
        "turn-1",
        "hello",
        "hi",
    )]));
    let manager = ConversationManager::new_with_services(
        runtime,
        repository.clone(),
        compression.clone(),
    )
    .unwrap();

    let started = manager
        .start_conversation(StartConversationInput {
            prompt: "hello".into(),
            target_kind: TurnTargetKind::Agent,
            target_id: "reviewer".into(),
        })
        .await
        .unwrap();

    wait_for_repository_record(&*repository, &started.conversation_id).await;

    let saved = repository.load(&started.conversation_id).unwrap().unwrap();
    assert_eq!(saved.history.len(), 2);

    let resumed_runtime = Arc::new(FakeConversationRuntime::with_completed_turns(vec![completed_turn(
        "turn-2",
        "continue",
        "done",
    )]));
    let resumed_runtime_ref = Arc::clone(&resumed_runtime);
    let rehydrated = ConversationManager::new_with_services(
        resumed_runtime,
        repository.clone(),
        compression,
    )
    .unwrap();

    let snapshot = rehydrated.snapshot(&started.conversation_id).await.unwrap();
    assert_eq!(snapshot.history.len(), 2);
    assert!(snapshot.active_turn_id.is_none());

    rehydrated
        .continue_conversation(ContinueConversationInput {
            conversation_id: started.conversation_id.clone(),
            prompt: "continue".into(),
        })
        .await
        .unwrap();

    timeout(Duration::from_secs(1), async {
        loop {
            if !resumed_runtime_ref.recorded_histories().await.is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();

    let histories = resumed_runtime_ref.recorded_histories().await;
    assert_eq!(histories[0].len(), 2);
    assert!(matches!(
        histories[0][0].as_ref(),
        TurnMessage::User { content } if content.as_ref() == "hello"
    ));
    assert!(matches!(
        histories[0][1].as_ref(),
        TurnMessage::AssistantText { content } if content.as_ref() == "hi"
    ));
}

async fn wait_for_repository_record(
    repository: &dyn ConversationRepository,
    conversation_id: &str,
) {
    timeout(Duration::from_secs(1), async {
        loop {
            if repository.load(conversation_id).unwrap().is_some() {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

fn completed_turn(turn_id: &str, user: &str, assistant: &str) -> CompletedTurn {
    CompletedTurn {
        turn_id: turn_id.into(),
        transcript: Arc::from([
            Arc::new(TurnMessage::User {
                content: user.into(),
            }),
            Arc::new(TurnMessage::AssistantText {
                content: assistant.into(),
            }),
        ]),
        assistant_text: Some(assistant.into()),
        finish_reason: turn::TurnFinishReason::Completed,
    }
}

struct FakeConversationRuntime {
    recorded_histories: Mutex<Vec<Vec<Arc<TurnMessage>>>>,
    completed: Mutex<VecDeque<CompletedTurn>>,
}

impl FakeConversationRuntime {
    fn with_completed_turns(turns: Vec<CompletedTurn>) -> Self {
        Self {
            recorded_histories: Mutex::new(Vec::new()),
            completed: Mutex::new(turns.into()),
        }
    }

    async fn recorded_histories(&self) -> Vec<Vec<Arc<TurnMessage>>> {
        self.recorded_histories.lock().await.clone()
    }
}

#[async_trait]
impl ConversationRuntime for FakeConversationRuntime {
    async fn spawn_turn(
        &self,
        _context: TurnContext,
        history: Arc<[Arc<TurnMessage>]>,
    ) -> Result<RunningConversationTurn, TurnError> {
        self.recorded_histories
            .lock()
            .await
            .push(history.iter().cloned().collect());

        let controller = Arc::new(FakeTurnController);
        let completed = self
            .completed
            .lock()
            .await
            .pop_front()
            .expect("missing completed fake turn");

        Ok(RunningConversationTurn {
            controller,
            task: tokio::spawn(async move { Ok(TurnOutcome::Completed(completed)) }),
        })
    }
}

struct FakeTurnController;

#[async_trait]
impl ConversationTurnControl for FakeTurnController {
    async fn cancel(&self) -> Result<(), TurnError> {
        Ok(())
    }
}
