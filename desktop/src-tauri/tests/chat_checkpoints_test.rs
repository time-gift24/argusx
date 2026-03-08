use std::sync::Arc;

use async_trait::async_trait;
use desktop_lib::chat::{
    commands::{create_checkpoint_with_manager, restore_checkpoint_with_manager},
    ContinueConversationInput, ConversationManager, ConversationRuntime, ConversationTurnControl,
    CreateConversationCheckpointInput, RestoreConversationCheckpointInput,
    RunningConversationTurn, StartConversationInput, ThreadStatus, TurnTargetKind,
};
use tokio::{
    sync::Mutex,
    task::JoinHandle,
    time::{Duration, timeout},
};
use turn::{CompletedTurn, TurnContext, TurnError, TurnMessage, TurnOutcome};

#[tokio::test(flavor = "current_thread")]
async fn checkpoints_restore_history_by_creating_a_new_branch() {
    let runtime = Arc::new(FakeConversationRuntime::with_completed_turns(vec![
        completed_turn(
            "turn-1",
            vec![
                TurnMessage::User {
                    content: "hello".into(),
                },
                TurnMessage::AssistantText {
                    content: "hi".into(),
                },
            ],
            "hi",
        ),
        completed_turn(
            "turn-2",
            vec![
                TurnMessage::User {
                    content: "hello".into(),
                },
                TurnMessage::AssistantText {
                    content: "hi".into(),
                },
                TurnMessage::User {
                    content: "continue".into(),
                },
                TurnMessage::AssistantText {
                    content: "done".into(),
                },
            ],
            "done",
        ),
    ]));
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

    let checkpoint = create_checkpoint_with_manager(
        &manager,
        CreateConversationCheckpointInput {
            conversation_id: started.conversation_id.clone(),
            title: Some("Before Continue".into()),
        },
    )
    .await
    .unwrap();

    manager
        .continue_conversation(ContinueConversationInput {
            conversation_id: started.conversation_id.clone(),
            prompt: "continue".into(),
        })
        .await
        .unwrap();
    wait_for_history_len(&manager, &started.conversation_id, 4).await;

    let restored = restore_checkpoint_with_manager(
        &manager,
        RestoreConversationCheckpointInput {
            checkpoint_id: checkpoint.checkpoint_id.clone(),
            title: Some("Undo Branch".into()),
        },
    )
    .await
    .unwrap();

    assert_ne!(restored.conversation_id, started.conversation_id);
    assert_eq!(restored.title, "Undo Branch");
    assert_eq!(restored.status, ThreadStatus::Idle);
    assert!(restored.is_active);

    let original_snapshot = manager.snapshot(&started.conversation_id).await.unwrap();
    let restored_snapshot = wait_for_history_len(&manager, &restored.conversation_id, 2).await;
    assert_eq!(original_snapshot.history.len(), 4);
    assert_eq!(restored_snapshot.history.len(), 2);
    assert!(matches!(
        restored_snapshot.history[0].as_ref(),
        TurnMessage::User { content } if content.as_ref() == "hello"
    ));
    assert!(matches!(
        restored_snapshot.history[1].as_ref(),
        TurnMessage::AssistantText { content } if content.as_ref() == "hi"
    ));
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

fn completed_turn(turn_id: &str, transcript: Vec<TurnMessage>, assistant: &str) -> CompletedTurn {
    CompletedTurn {
        turn_id: turn_id.into(),
        transcript: Arc::from(
            transcript
                .into_iter()
                .map(Arc::new)
                .collect::<Vec<_>>(),
        ),
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
