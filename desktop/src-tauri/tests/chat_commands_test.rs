use std::sync::Arc;

use async_trait::async_trait;
use desktop_lib::chat::{
    commands::{
        cancel_conversation_with_manager, continue_conversation_with_manager,
        start_conversation_with_manager,
    },
    ContinueConversationInput, ConversationManager, ConversationRuntime, ConversationTurnControl,
    RunningConversationTurn, StartConversationInput, TurnTargetKind,
};
use tokio::{
    sync::Mutex,
    task::JoinHandle,
    time::{Duration, timeout},
};
use turn::{CompletedTurn, TurnContext, TurnError, TurnMessage, TurnOutcome};

#[tokio::test(flavor = "current_thread")]
async fn command_helpers_start_continue_and_cancel_conversations() {
    let runtime = Arc::new(FakeConversationRuntime::with_completed_turns(vec![
        completed_turn("turn-1", "hello", "hi"),
        completed_turn("turn-2", "continue", "done"),
    ]));
    let runtime_ref = Arc::clone(&runtime);
    let manager = ConversationManager::new(runtime);

    let started = start_conversation_with_manager(
        &manager,
        StartConversationInput {
            prompt: "hello".into(),
            target_kind: TurnTargetKind::Agent,
            target_id: "reviewer".into(),
        },
    )
    .await
    .unwrap();
    wait_for_history_len(&manager, &started.conversation_id, 2).await;

    let continued = continue_conversation_with_manager(
        &manager,
        ContinueConversationInput {
            conversation_id: started.conversation_id.clone(),
            prompt: "continue".into(),
        },
    )
    .await
    .unwrap();

    cancel_conversation_with_manager(&manager, started.conversation_id.clone())
        .await
        .unwrap();

    let histories = runtime_ref.recorded_histories().await;
    assert_eq!(histories.len(), 2);
    assert_eq!(continued.conversation_id, started.conversation_id);
    assert_ne!(continued.turn_id, started.turn_id);
    assert!(runtime_ref.cancelled().await);
}

async fn wait_for_history_len(
    manager: &ConversationManager,
    conversation_id: &str,
    expected_len: usize,
) {
    timeout(Duration::from_secs(1), async {
        loop {
            if let Some(snapshot) = manager.snapshot(conversation_id).await {
                if snapshot.history.len() == expected_len {
                    return;
                }
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
    completed: Mutex<Vec<CompletedTurn>>,
    cancel_state: Arc<Mutex<bool>>,
}

impl FakeConversationRuntime {
    fn with_completed_turns(turns: Vec<CompletedTurn>) -> Self {
        Self {
            recorded_histories: Mutex::new(Vec::new()),
            completed: Mutex::new(turns),
            cancel_state: Arc::new(Mutex::new(false)),
        }
    }

    async fn recorded_histories(&self) -> Vec<Vec<Arc<TurnMessage>>> {
        self.recorded_histories.lock().await.clone()
    }

    async fn cancelled(&self) -> bool {
        *self.cancel_state.lock().await
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

        let controller = Arc::new(FakeTurnController {
            cancelled: Arc::clone(&self.cancel_state),
        });
        let completed = self
            .completed
            .lock()
            .await
            .remove(0);
        let task: JoinHandle<Result<TurnOutcome, TurnError>> =
            tokio::spawn(async move { Ok(TurnOutcome::Completed(completed)) });

        Ok(RunningConversationTurn { controller, task })
    }
}

struct FakeTurnController {
    cancelled: Arc<Mutex<bool>>,
}

#[async_trait]
impl ConversationTurnControl for FakeTurnController {
    async fn cancel(&self) -> Result<(), TurnError> {
        *self.cancelled.lock().await = true;
        Ok(())
    }
}
