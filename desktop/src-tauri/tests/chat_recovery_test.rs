use std::{collections::VecDeque, sync::Arc};

use async_trait::async_trait;
use desktop_lib::chat::{
    checkpoints::InMemoryConversationCheckpointRepository,
    commands::restart_conversation_with_manager,
    compression::NoopConversationCompressionPolicy,
    manager::ConversationManager,
    storage::InMemoryConversationRepository,
    threads::{InMemoryConversationThreadRepository, ThreadStatus},
    ContinueConversationInput, ConversationRuntime, ConversationTurnControl,
    RestartConversationInput, RunningConversationTurn, StartConversationInput, TurnTargetKind,
};
use tokio::{
    sync::Mutex,
    task::JoinHandle,
    time::{Duration, timeout},
};
use turn::{CompletedTurn, TurnContext, TurnError, TurnMessage, TurnOutcome};

#[tokio::test(flavor = "current_thread")]
async fn startup_marks_running_threads_restartable_and_restart_reuses_durable_history() {
    let conversation_repository = Arc::new(InMemoryConversationRepository::default());
    let thread_repository = Arc::new(InMemoryConversationThreadRepository::default());
    let checkpoint_repository = Arc::new(InMemoryConversationCheckpointRepository::default());

    let first_runtime = Arc::new(QueuedRuntime::with_turns(vec![
        RuntimeTurn::Completed(completed_turn(
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
        )),
        RuntimeTurn::Pending,
    ]));
    let first_manager = ConversationManager::new_with_repositories(
        first_runtime,
        conversation_repository.clone(),
        Arc::new(NoopConversationCompressionPolicy),
        thread_repository.clone(),
        checkpoint_repository.clone(),
    )
    .unwrap();

    let started = first_manager
        .start_conversation(StartConversationInput {
            prompt: "hello".into(),
            target_kind: TurnTargetKind::Agent,
            target_id: "reviewer".into(),
        })
        .await
        .unwrap();
    wait_for_history_len(&first_manager, &started.conversation_id, 2).await;

    first_manager
        .continue_conversation(ContinueConversationInput {
            conversation_id: started.conversation_id.clone(),
            prompt: "continue".into(),
        })
        .await
        .unwrap();

    let second_runtime = Arc::new(QueuedRuntime::with_turns(vec![RuntimeTurn::Completed(
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
    )]));
    let second_runtime_ref = Arc::clone(&second_runtime);
    let restarted_manager = ConversationManager::new_with_repositories(
        second_runtime,
        conversation_repository,
        Arc::new(NoopConversationCompressionPolicy),
        thread_repository,
        checkpoint_repository,
    )
    .unwrap();

    let threads = restarted_manager.list_threads().await.unwrap();
    let restartable = threads
        .iter()
        .find(|thread| thread.conversation_id == started.conversation_id)
        .unwrap();
    assert_eq!(restartable.status, ThreadStatus::Restartable);

    restart_conversation_with_manager(
        &restarted_manager,
        RestartConversationInput {
            conversation_id: started.conversation_id.clone(),
        },
    )
    .await
    .unwrap();

    let recorded = second_runtime_ref.recorded_turns().await;
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].prompt, "continue");
    assert_eq!(recorded[0].history_len, 2);

    let snapshot = wait_for_history_len(&restarted_manager, &started.conversation_id, 4).await;
    assert_eq!(snapshot.status, ThreadStatus::Idle);
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

#[derive(Clone)]
struct RecordedTurn {
    history_len: usize,
    prompt: String,
}

enum RuntimeTurn {
    Completed(CompletedTurn),
    Pending,
}

struct QueuedRuntime {
    recorded_turns: Mutex<Vec<RecordedTurn>>,
    turns: Mutex<VecDeque<RuntimeTurn>>,
}

impl QueuedRuntime {
    fn with_turns(turns: Vec<RuntimeTurn>) -> Self {
        Self {
            recorded_turns: Mutex::new(Vec::new()),
            turns: Mutex::new(turns.into()),
        }
    }

    async fn recorded_turns(&self) -> Vec<RecordedTurn> {
        self.recorded_turns.lock().await.clone()
    }
}

#[async_trait]
impl ConversationRuntime for QueuedRuntime {
    async fn spawn_turn(
        &self,
        context: TurnContext,
        history: Arc<[Arc<TurnMessage>]>,
    ) -> Result<RunningConversationTurn, TurnError> {
        self.recorded_turns.lock().await.push(RecordedTurn {
            history_len: history.len(),
            prompt: context.user_message.to_string(),
        });

        let controller = Arc::new(FakeTurnController);
        let next_turn = self
            .turns
            .lock()
            .await
            .pop_front()
            .expect("expected queued runtime turn");
        let task: JoinHandle<Result<TurnOutcome, TurnError>> = match next_turn {
            RuntimeTurn::Completed(turn) => {
                tokio::spawn(async move { Ok(TurnOutcome::Completed(turn)) })
            }
            RuntimeTurn::Pending => tokio::spawn(async move {
                std::future::pending::<()>().await;
                unreachable!()
            }),
        };

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
