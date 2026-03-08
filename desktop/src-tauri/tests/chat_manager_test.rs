use std::{collections::VecDeque, sync::Arc};

use async_trait::async_trait;
use desktop_lib::chat::{
    ContinueConversationInput, ConversationManager, ConversationRuntime, ConversationTurnControl,
    RunningConversationTurn, StartConversationInput, TurnTargetKind,
};
use tokio::{
    sync::{Mutex, Notify},
    task::JoinHandle,
    time::{Duration, timeout},
};
use turn::{CompletedTurn, TurnContext, TurnError, TurnMessage, TurnOutcome};

#[tokio::test(flavor = "current_thread")]
async fn starting_conversation_returns_ids_and_persists_completed_history() {
    let runtime = Arc::new(FakeConversationRuntime::with_completed_turns(vec![completed_turn(
        "turn-1",
        "hello",
        "hi",
    )]));
    let manager = ConversationManager::new(runtime);

    let started = manager
        .start_conversation(StartConversationInput {
            prompt: "hello".into(),
            target_kind: TurnTargetKind::Agent,
            target_id: "reviewer".into(),
        })
        .await
        .unwrap();

    assert!(!started.conversation_id.is_empty());
    assert!(!started.turn_id.is_empty());

    let snapshot = wait_for_history_len(&manager, &started.conversation_id, 2).await;
    assert_eq!(snapshot.history.len(), 2);
    assert!(snapshot.active_turn_id.is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn continuing_conversation_reuses_completed_turn_history() {
    let runtime = Arc::new(FakeConversationRuntime::with_completed_turns(vec![
        completed_turn("turn-1", "hello", "hi"),
        completed_turn("turn-2", "continue", "done"),
    ]));
    let runtime_ref = Arc::clone(&runtime);
    let manager = ConversationManager::new(runtime);

    let started = manager
        .start_conversation(StartConversationInput {
            prompt: "hello".into(),
            target_kind: TurnTargetKind::Agent,
            target_id: "reviewer".into(),
        })
        .await
        .unwrap();
    wait_for_history_len(&manager, &started.conversation_id, 2).await;

    manager
        .continue_conversation(ContinueConversationInput {
            conversation_id: started.conversation_id.clone(),
            prompt: "continue".into(),
        })
        .await
        .unwrap();

    timeout(Duration::from_secs(1), async {
        loop {
            if runtime_ref.recorded_histories().await.len() >= 2 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();

    let histories = runtime_ref.recorded_histories().await;
    assert_eq!(histories[1].len(), 2);
    assert!(matches!(
        histories[1][0].as_ref(),
        TurnMessage::User { content } if content.as_ref() == "hello"
    ));
    assert!(matches!(
        histories[1][1].as_ref(),
        TurnMessage::AssistantText { content } if content.as_ref() == "hi"
    ));
}

#[tokio::test(flavor = "current_thread")]
async fn cancelling_active_conversation_clears_live_controller_without_erasing_history() {
    let runtime = Arc::new(FakeConversationRuntime::with_pending_turn());
    let runtime_ref = Arc::clone(&runtime);
    let manager = ConversationManager::new(runtime);

    let started = manager
        .start_conversation(StartConversationInput {
            prompt: "hello".into(),
            target_kind: TurnTargetKind::Agent,
            target_id: "reviewer".into(),
        })
        .await
        .unwrap();

    manager
        .cancel_conversation(started.conversation_id.clone())
        .await
        .unwrap();

    let snapshot = wait_for_snapshot(&manager, &started.conversation_id).await;
    assert!(snapshot.active_turn_id.is_none());
    assert!(runtime_ref.cancelled().await);
    assert!(snapshot.history.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn continuing_conversation_while_turn_is_running_returns_error() {
    let runtime = Arc::new(FakeConversationRuntime::with_pending_turn());
    let runtime_ref = Arc::clone(&runtime);
    let manager = ConversationManager::new(runtime);

    let started = manager
        .start_conversation(StartConversationInput {
            prompt: "hello".into(),
            target_kind: TurnTargetKind::Agent,
            target_id: "reviewer".into(),
        })
        .await
        .unwrap();

    let error = manager
        .continue_conversation(ContinueConversationInput {
            conversation_id: started.conversation_id.clone(),
            prompt: "continue".into(),
        })
        .await
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("conversation already has an active turn"));

    let snapshot = wait_for_snapshot(&manager, &started.conversation_id).await;
    assert_eq!(snapshot.active_turn_id.as_deref(), Some(started.turn_id.as_str()));
    assert_eq!(runtime_ref.recorded_histories().await.len(), 1);
}

async fn wait_for_snapshot(
    manager: &ConversationManager,
    conversation_id: &str,
) -> desktop_lib::chat::ConversationSnapshot {
    timeout(Duration::from_secs(1), async {
        loop {
            if let Some(snapshot) = manager.snapshot(conversation_id).await {
                return snapshot;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap()
}

async fn wait_for_history_len(
    manager: &ConversationManager,
    conversation_id: &str,
    expected_len: usize,
) -> desktop_lib::chat::ConversationSnapshot {
    timeout(Duration::from_secs(1), async {
        loop {
            if let Some(snapshot) = manager.snapshot(conversation_id).await {
                if snapshot.history.len() == expected_len {
                    return snapshot;
                }
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap()
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
    pending_notify: Arc<Notify>,
    cancel_state: Arc<Mutex<bool>>,
    pending_mode: bool,
}

impl FakeConversationRuntime {
    fn with_completed_turns(turns: Vec<CompletedTurn>) -> Self {
        Self {
            recorded_histories: Mutex::new(Vec::new()),
            completed: Mutex::new(turns.into()),
            pending_notify: Arc::new(Notify::new()),
            cancel_state: Arc::new(Mutex::new(false)),
            pending_mode: false,
        }
    }

    fn with_pending_turn() -> Self {
        Self {
            recorded_histories: Mutex::new(Vec::new()),
            completed: Mutex::new(VecDeque::new()),
            pending_notify: Arc::new(Notify::new()),
            cancel_state: Arc::new(Mutex::new(false)),
            pending_mode: true,
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
        context: TurnContext,
        history: Arc<[Arc<TurnMessage>]>,
    ) -> Result<RunningConversationTurn, TurnError> {
        self.recorded_histories
            .lock()
            .await
            .push(history.iter().cloned().collect());

        let controller = Arc::new(FakeTurnController {
            cancelled: Arc::clone(&self.cancel_state),
        });

        let task: JoinHandle<Result<TurnOutcome, TurnError>> = if self.pending_mode {
            let notify = Arc::clone(&self.pending_notify);
            tokio::spawn(async move {
                notify.notified().await;
                Ok(TurnOutcome::Cancelled(turn::TurnSummary {
                    turn_id: context.turn_id,
                }))
            })
        } else {
            let completed = self
                .completed
                .lock()
                .await
                .pop_front()
                .expect("missing completed fake turn");
            tokio::spawn(async move { Ok(TurnOutcome::Completed(completed)) })
        };

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
