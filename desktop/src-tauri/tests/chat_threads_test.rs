use std::sync::Arc;

use async_trait::async_trait;
use desktop_lib::chat::{
    commands::{
        create_thread_with_manager, list_threads_with_manager, switch_thread_with_manager,
    },
    ContinueConversationInput, ConversationManager, ConversationRuntime, ConversationTurnControl,
    CreateConversationThreadInput, RunningConversationTurn, StartConversationInput,
    SwitchConversationThreadInput, ThreadStatus, TurnTargetKind,
};
use tokio::{
    sync::Mutex,
    task::JoinHandle,
    time::{Duration, timeout},
};
use turn::{CompletedTurn, TurnContext, TurnError, TurnMessage, TurnOutcome};

#[tokio::test(flavor = "current_thread")]
async fn thread_catalog_lists_titles_and_switches_active_thread_without_mixing_histories() {
    let runtime = Arc::new(FakeConversationRuntime::with_completed_turns(vec![
        completed_turn("turn-1", "hello", "hi"),
        completed_turn("turn-2", "plan", "done"),
    ]));
    let manager = ConversationManager::new(runtime);

    let first = manager
        .start_conversation(StartConversationInput {
            prompt: "hello".into(),
            target_kind: TurnTargetKind::Agent,
            target_id: "reviewer".into(),
        })
        .await
        .unwrap();
    wait_for_history_len(&manager, &first.conversation_id, 2).await;

    let second = create_thread_with_manager(
        &manager,
        CreateConversationThreadInput {
            title: "Planning".into(),
            target_kind: TurnTargetKind::Workflow,
            target_id: "execute".into(),
        },
    )
    .await
    .unwrap();

    let switched = switch_thread_with_manager(
        &manager,
        SwitchConversationThreadInput {
            conversation_id: second.conversation_id.clone(),
        },
    )
    .await
    .unwrap();

    assert_eq!(switched.conversation_id, second.conversation_id);
    assert!(switched.is_active);

    manager
        .continue_conversation(ContinueConversationInput {
            conversation_id: second.conversation_id.clone(),
            prompt: "plan".into(),
        })
        .await
        .unwrap();
    wait_for_history_len(&manager, &second.conversation_id, 2).await;

    let threads = list_threads_with_manager(&manager).await.unwrap();
    assert_eq!(threads.len(), 2);

    let first_thread = threads
        .iter()
        .find(|thread| thread.conversation_id == first.conversation_id)
        .unwrap();
    let second_thread = threads
        .iter()
        .find(|thread| thread.conversation_id == second.conversation_id)
        .unwrap();

    assert_eq!(first_thread.title, "hello");
    assert_eq!(first_thread.status, ThreadStatus::Idle);
    assert!(!first_thread.is_active);
    assert_eq!(second_thread.title, "Planning");
    assert_eq!(second_thread.status, ThreadStatus::Idle);
    assert!(second_thread.updated_at_ms > 0);
    assert!(second_thread.is_active);

    let first_snapshot = manager.snapshot(&first.conversation_id).await.unwrap();
    let second_snapshot = manager.snapshot(&second.conversation_id).await.unwrap();
    assert!(matches!(
        first_snapshot.history[0].as_ref(),
        TurnMessage::User { content } if content.as_ref() == "hello"
    ));
    assert!(matches!(
        second_snapshot.history[0].as_ref(),
        TurnMessage::User { content } if content.as_ref() == "plan"
    ));
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
    completed: Mutex<Vec<CompletedTurn>>,
}

impl FakeConversationRuntime {
    fn with_completed_turns(turns: Vec<CompletedTurn>) -> Self {
        Self {
            completed: Mutex::new(turns),
        }
    }
}

#[async_trait]
impl ConversationRuntime for FakeConversationRuntime {
    async fn spawn_turn(
        &self,
        _context: TurnContext,
        _history: Arc<[Arc<TurnMessage>]>,
    ) -> Result<RunningConversationTurn, TurnError> {
        let controller = Arc::new(FakeTurnController);
        let completed = self.completed.lock().await.remove(0);
        let task: JoinHandle<Result<TurnOutcome, TurnError>> =
            tokio::spawn(async move { Ok(TurnOutcome::Completed(completed)) });

        Ok(RunningConversationTurn { controller, task })
    }
}

struct FakeTurnController;

#[async_trait]
impl ConversationTurnControl for FakeTurnController {
    async fn cancel(&self) -> Result<(), TurnError> {
        Ok(())
    }
}
