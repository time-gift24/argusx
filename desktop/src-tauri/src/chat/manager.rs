use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex, task::JoinHandle};
use turn::{
    ModelRunner, ToolAuthorizer, ToolRunner, TurnContext, TurnDriver, TurnError,
    TurnMessage, TurnMessageSnapshot, TurnOutcome,
};
use uuid::Uuid;

use super::{
    authorizer::AllowListAuthorizer,
    checkpoints::{
        ConversationCheckpointRecord, ConversationCheckpointRepository,
        ConversationCheckpointSummary, CreateConversationCheckpointInput,
        InMemoryConversationCheckpointRepository, RestoreConversationCheckpointInput,
    },
    compression::{ConversationCompressionPolicy, NoopConversationCompressionPolicy},
    events::{ContinueConversationInput, StartConversationInput, TurnTargetKind},
    model::ProviderModelRunner,
    observer::{ChatEventSink, DesktopTurnObserver},
    storage::{ConversationRecord, ConversationRepository, InMemoryConversationRepository},
    threads::{
        ConversationThreadRecord, ConversationThreadRepository, ConversationThreadSummary,
        CreateConversationThreadInput, InMemoryConversationThreadRepository,
        RestartConversationInput, ThreadStatus,
    },
    tools::default_tooling,
};

#[async_trait]
pub trait ConversationTurnControl: Send + Sync {
    async fn cancel(&self) -> Result<(), TurnError>;
}

pub struct RunningConversationTurn {
    pub controller: Arc<dyn ConversationTurnControl>,
    pub task: JoinHandle<Result<TurnOutcome, TurnError>>,
}

#[async_trait]
pub trait ConversationRuntime: Send + Sync {
    async fn spawn_turn(
        &self,
        context: TurnContext,
        history: TurnMessageSnapshot,
    ) -> Result<RunningConversationTurn, TurnError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationTurnStarted {
    pub conversation_id: String,
    pub turn_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationSnapshot {
    pub conversation_id: String,
    pub title: String,
    pub target_kind: TurnTargetKind,
    pub target_id: String,
    pub history: TurnMessageSnapshot,
    pub status: ThreadStatus,
    pub active_turn_id: Option<String>,
}

struct ConversationState {
    title: String,
    target_kind: TurnTargetKind,
    target_id: String,
    history: TurnMessageSnapshot,
    pending_prompt: Option<String>,
    status: ThreadStatus,
    updated_at_ms: u64,
    active_turn_id: Option<String>,
    active_controller: Option<Arc<dyn ConversationTurnControl>>,
}

#[derive(Clone)]
pub struct ConversationManager {
    runtime: Arc<dyn ConversationRuntime>,
    active_conversation_id: Arc<Mutex<Option<String>>>,
    conversations: Arc<Mutex<HashMap<String, ConversationState>>>,
    checkpoint_repository: Arc<dyn ConversationCheckpointRepository>,
    repository: Arc<dyn ConversationRepository>,
    compression: Arc<dyn ConversationCompressionPolicy>,
    thread_repository: Arc<dyn ConversationThreadRepository>,
}

impl ConversationManager {
    pub fn new(runtime: Arc<dyn ConversationRuntime>) -> Self {
        Self::new_with_thread_repository(
            runtime,
            Arc::new(InMemoryConversationRepository::default()),
            Arc::new(NoopConversationCompressionPolicy),
            Arc::new(InMemoryConversationThreadRepository::default()),
            Arc::new(InMemoryConversationCheckpointRepository::default()),
        )
        .expect("default conversation manager services must initialize")
    }

    pub fn new_with_services(
        runtime: Arc<dyn ConversationRuntime>,
        repository: Arc<dyn ConversationRepository>,
        compression: Arc<dyn ConversationCompressionPolicy>,
    ) -> Result<Self, TurnError> {
        Self::new_with_thread_repository(
            runtime,
            repository,
            compression,
            Arc::new(InMemoryConversationThreadRepository::default()),
            Arc::new(InMemoryConversationCheckpointRepository::default()),
        )
    }

    pub fn new_with_thread_repository(
        runtime: Arc<dyn ConversationRuntime>,
        repository: Arc<dyn ConversationRepository>,
        compression: Arc<dyn ConversationCompressionPolicy>,
        thread_repository: Arc<dyn ConversationThreadRepository>,
        checkpoint_repository: Arc<dyn ConversationCheckpointRepository>,
    ) -> Result<Self, TurnError> {
        Self::new_with_repositories(
            runtime,
            repository,
            compression,
            thread_repository,
            checkpoint_repository,
        )
    }

    pub fn new_with_repositories(
        runtime: Arc<dyn ConversationRuntime>,
        repository: Arc<dyn ConversationRepository>,
        compression: Arc<dyn ConversationCompressionPolicy>,
        thread_repository: Arc<dyn ConversationThreadRepository>,
        checkpoint_repository: Arc<dyn ConversationCheckpointRepository>,
    ) -> Result<Self, TurnError> {
        let mut conversations = repository
            .list()?
            .into_iter()
            .map(conversation_state_from_record)
            .collect::<HashMap<_, _>>();

        for thread in thread_repository.list_threads()? {
            let conversation_id = thread.conversation_id.clone();
            if let Some(state) = conversations.get_mut(&conversation_id) {
                state.title = thread.title;
                state.target_kind = thread.target_kind;
                state.target_id = thread.target_id;
                state.updated_at_ms = state.updated_at_ms.max(thread.updated_at_ms);
                state.pending_prompt = thread.pending_prompt;
                state.status = normalize_thread_status(thread.status);
            } else {
                conversations.insert(
                    conversation_id,
                    ConversationState {
                        title: thread.title,
                        target_kind: thread.target_kind,
                        target_id: thread.target_id,
                        history: Arc::from([]),
                        pending_prompt: thread.pending_prompt,
                        status: normalize_thread_status(thread.status),
                        updated_at_ms: thread.updated_at_ms,
                        active_turn_id: None,
                        active_controller: None,
                    },
                );
            }
        }

        let active_conversation_id = thread_repository
            .active_thread_id()?
            .filter(|conversation_id| conversations.contains_key(conversation_id));

        Ok(Self {
            runtime,
            active_conversation_id: Arc::new(Mutex::new(active_conversation_id)),
            conversations: Arc::new(Mutex::new(conversations)),
            checkpoint_repository,
            repository,
            compression,
            thread_repository,
        })
    }

    pub async fn create_thread(
        &self,
        input: CreateConversationThreadInput,
    ) -> Result<ConversationThreadSummary, TurnError> {
        let conversation_id = Uuid::new_v4().to_string();
        let state = ConversationState {
            title: input.title,
            target_kind: input.target_kind,
            target_id: input.target_id,
            history: Arc::from([]),
            pending_prompt: None,
            status: ThreadStatus::Idle,
            updated_at_ms: now_timestamp_ms(),
            active_turn_id: None,
            active_controller: None,
        };
        let summary = ConversationThreadSummary::from_record(
            thread_record(&conversation_id, &state),
            true,
        );

        self.conversations
            .lock()
            .await
            .insert(conversation_id.clone(), state);
        self.thread_repository
            .save_thread(thread_record_from_summary(&summary))?;
        self.activate_conversation(Some(conversation_id)).await?;

        Ok(summary)
    }

    pub async fn list_threads(&self) -> Result<Vec<ConversationThreadSummary>, TurnError> {
        let active_conversation_id = self.active_conversation_id.lock().await.clone();
        let conversations = self.conversations.lock().await;
        let mut threads = conversations
            .iter()
            .map(|(conversation_id, state)| {
                ConversationThreadSummary::from_record(
                    thread_record(conversation_id, state),
                    active_conversation_id.as_deref() == Some(conversation_id.as_str()),
                )
            })
            .collect::<Vec<_>>();
        threads.sort_by(|left, right| {
            right
                .updated_at_ms
                .cmp(&left.updated_at_ms)
                .then_with(|| left.conversation_id.cmp(&right.conversation_id))
        });
        Ok(threads)
    }

    pub async fn switch_thread(
        &self,
        conversation_id: String,
    ) -> Result<ConversationThreadSummary, TurnError> {
        let summary = {
            let conversations = self.conversations.lock().await;
            let state = conversations.get(&conversation_id).ok_or_else(|| {
                TurnError::Runtime(format!("conversation not found: {conversation_id}"))
            })?;
            ConversationThreadSummary::from_record(thread_record(&conversation_id, state), true)
        };

        self.activate_conversation(Some(conversation_id)).await?;

        Ok(summary)
    }

    pub async fn create_checkpoint(
        &self,
        input: CreateConversationCheckpointInput,
    ) -> Result<ConversationCheckpointSummary, TurnError> {
        let record = {
            let conversations = self.conversations.lock().await;
            let state = conversations.get(&input.conversation_id).ok_or_else(|| {
                TurnError::Runtime(format!(
                    "conversation not found: {}",
                    input.conversation_id
                ))
            })?;
            if state.active_turn_id.is_some() {
                return Err(TurnError::Runtime(
                    "cannot checkpoint a conversation with an active turn".to_string(),
                ));
            }

            ConversationCheckpointRecord::from_snapshot(
                Uuid::new_v4().to_string(),
                input.conversation_id.clone(),
                input.title.unwrap_or_else(|| state.title.clone()),
                state.target_kind.clone(),
                state.target_id.clone(),
                state.history.clone(),
                now_timestamp_ms(),
            )
        };
        let summary = ConversationCheckpointSummary::from(&record);
        self.checkpoint_repository.save(record)?;
        Ok(summary)
    }

    pub async fn restore_checkpoint(
        &self,
        input: RestoreConversationCheckpointInput,
    ) -> Result<ConversationThreadSummary, TurnError> {
        let checkpoint = self
            .checkpoint_repository
            .load(&input.checkpoint_id)?
            .ok_or_else(|| TurnError::Runtime(format!("checkpoint not found: {}", input.checkpoint_id)))?;
        let conversation_id = Uuid::new_v4().to_string();
        let title = input
            .title
            .unwrap_or_else(|| format!("{} (restored)", checkpoint.title));
        let history = checkpoint.to_snapshot();
        let updated_at_ms = now_timestamp_ms();
        let state = ConversationState {
            title,
            target_kind: checkpoint.target_kind,
            target_id: checkpoint.target_id,
            history: history.clone(),
            pending_prompt: None,
            status: ThreadStatus::Idle,
            updated_at_ms,
            active_turn_id: None,
            active_controller: None,
        };
        let summary = ConversationThreadSummary::from_record(
            thread_record(&conversation_id, &state),
            true,
        );

        self.conversations
            .lock()
            .await
            .insert(conversation_id.clone(), state);
        self.repository.save(ConversationRecord::from_snapshot(
            conversation_id.clone(),
            summary.target_kind.clone(),
            summary.target_id.clone(),
            history,
            updated_at_ms,
        ))?;
        self.thread_repository
            .save_thread(thread_record_from_summary(&summary))?;
        self.activate_conversation(Some(conversation_id)).await?;

        Ok(summary)
    }

    pub async fn restart_conversation(
        &self,
        input: RestartConversationInput,
    ) -> Result<ConversationTurnStarted, TurnError> {
        let turn_id = Uuid::new_v4().to_string();
        let (history, prompt) = {
            let conversations = self.conversations.lock().await;
            let state = conversations.get(&input.conversation_id).ok_or_else(|| {
                TurnError::Runtime(format!(
                    "conversation not found: {}",
                    input.conversation_id
                ))
            })?;
            if state.active_turn_id.is_some() {
                return Err(TurnError::Runtime(
                    "conversation already has an active turn".to_string(),
                ));
            }
            if state.status != ThreadStatus::Restartable {
                return Err(TurnError::Runtime(
                    "conversation is not restartable".to_string(),
                ));
            }
            let prompt = state.pending_prompt.clone().ok_or_else(|| {
                TurnError::Runtime(
                    "restartable conversation is missing the interrupted prompt".to_string(),
                )
            })?;
            (state.history.clone(), prompt)
        };

        let running = self
            .runtime
            .spawn_turn(
                TurnContext {
                    session_id: input.conversation_id.clone(),
                    turn_id: turn_id.clone(),
                    user_message: prompt.clone(),
                },
                history,
            )
            .await?;

        if let Some(state) = self.conversations.lock().await.get_mut(&input.conversation_id) {
            state.updated_at_ms = now_timestamp_ms();
            state.status = ThreadStatus::Running;
            state.active_turn_id = Some(turn_id.clone());
            state.active_controller = Some(Arc::clone(&running.controller));
        }
        self.activate_conversation_best_effort(Some(input.conversation_id.clone()))
            .await;
        self.persist_thread_state_best_effort(&input.conversation_id)
            .await;

        self.spawn_outcome_watcher(input.conversation_id.clone(), turn_id.clone(), running.task);

        Ok(ConversationTurnStarted {
            conversation_id: input.conversation_id,
            turn_id,
        })
    }

    pub async fn start_conversation(
        &self,
        input: StartConversationInput,
    ) -> Result<ConversationTurnStarted, TurnError> {
        let conversation_id = Uuid::new_v4().to_string();
        let turn_id = Uuid::new_v4().to_string();
        let history: TurnMessageSnapshot = Arc::from([]);
        let prompt = input.prompt.clone();
        let title = conversation_title(&prompt);

        let running = self
            .runtime
            .spawn_turn(
                TurnContext {
                    session_id: conversation_id.clone(),
                    turn_id: turn_id.clone(),
                    user_message: prompt.clone(),
                },
                history.clone(),
            )
            .await?;

        {
            self.conversations.lock().await.insert(
                conversation_id.clone(),
                ConversationState {
                    title,
                    target_kind: input.target_kind,
                    target_id: input.target_id,
                    history,
                    pending_prompt: Some(prompt),
                    status: ThreadStatus::Running,
                    updated_at_ms: now_timestamp_ms(),
                    active_turn_id: Some(turn_id.clone()),
                    active_controller: Some(Arc::clone(&running.controller)),
                },
            );
        }
        self.activate_conversation_best_effort(Some(conversation_id.clone()))
            .await;
        self.persist_thread_state_best_effort(&conversation_id).await;

        self.spawn_outcome_watcher(conversation_id.clone(), turn_id.clone(), running.task);

        Ok(ConversationTurnStarted {
            conversation_id,
            turn_id,
        })
    }

    pub async fn continue_conversation(
        &self,
        input: ContinueConversationInput,
    ) -> Result<ConversationTurnStarted, TurnError> {
        let turn_id = Uuid::new_v4().to_string();
        let prompt = input.prompt.clone();
        let history = {
            let conversations = self.conversations.lock().await;
            let state = conversations.get(&input.conversation_id).ok_or_else(|| {
                TurnError::Runtime(format!(
                    "conversation not found: {}",
                    input.conversation_id
                ))
            })?;
            if state.active_turn_id.is_some() {
                return Err(TurnError::Runtime(
                    "conversation already has an active turn".to_string(),
                ));
            }
            state.history.clone()
        };

        let running = self
            .runtime
            .spawn_turn(
                TurnContext {
                    session_id: input.conversation_id.clone(),
                    turn_id: turn_id.clone(),
                    user_message: prompt.clone(),
                },
                history,
            )
            .await?;

        if let Some(state) = self.conversations.lock().await.get_mut(&input.conversation_id) {
            state.updated_at_ms = now_timestamp_ms();
            state.pending_prompt = Some(prompt);
            state.status = ThreadStatus::Running;
            state.active_turn_id = Some(turn_id.clone());
            state.active_controller = Some(Arc::clone(&running.controller));
        }
        self.activate_conversation_best_effort(Some(input.conversation_id.clone()))
            .await;
        self.persist_thread_state_best_effort(&input.conversation_id)
            .await;

        self.spawn_outcome_watcher(input.conversation_id.clone(), turn_id.clone(), running.task);

        Ok(ConversationTurnStarted {
            conversation_id: input.conversation_id,
            turn_id,
        })
    }

    pub async fn cancel_conversation(&self, conversation_id: String) -> Result<(), TurnError> {
        let controller = {
            let mut conversations = self.conversations.lock().await;
            let state = conversations.get_mut(&conversation_id).ok_or_else(|| {
                TurnError::Runtime(format!("conversation not found: {conversation_id}"))
            })?;
            state.pending_prompt = None;
            state.status = ThreadStatus::Idle;
            state.active_turn_id = None;
            state.active_controller.take()
        };
        self.persist_thread_state_best_effort(&conversation_id).await;

        if let Some(controller) = controller {
            controller.cancel().await?;
        }

        Ok(())
    }

    pub async fn snapshot(&self, conversation_id: &str) -> Option<ConversationSnapshot> {
        self.conversations
            .lock()
            .await
            .get(conversation_id)
            .map(|state| ConversationSnapshot {
                conversation_id: conversation_id.to_string(),
                title: state.title.clone(),
                target_kind: state.target_kind.clone(),
                target_id: state.target_id.clone(),
                history: state.history.clone(),
                status: state.status.clone(),
                active_turn_id: state.active_turn_id.clone(),
            })
    }

    fn spawn_outcome_watcher(
        &self,
        conversation_id: String,
        turn_id: String,
        task: JoinHandle<Result<TurnOutcome, TurnError>>,
    ) {
        let conversations = Arc::clone(&self.conversations);
        let repository = Arc::clone(&self.repository);
        let compression = Arc::clone(&self.compression);
        let thread_repository = Arc::clone(&self.thread_repository);
        tokio::spawn(async move {
            let result = task.await;
            let mut conversations = conversations.lock().await;
            let Some(state) = conversations.get_mut(&conversation_id) else {
                return;
            };

            if state.active_turn_id.as_deref() == Some(turn_id.as_str()) {
                state.active_turn_id = None;
                state.active_controller = None;
            }

            match result {
                Ok(Ok(TurnOutcome::Completed(completed))) => {
                    state.history = completed.transcript;
                    state.pending_prompt = None;
                    state.updated_at_ms = now_timestamp_ms();
                    state.status = ThreadStatus::Idle;

                    let record = ConversationRecord::from_snapshot(
                        conversation_id.clone(),
                        state.target_kind.clone(),
                        state.target_id.clone(),
                        state.history.clone(),
                        state.updated_at_ms,
                    );

                    match compression.compress(record).and_then(|record| repository.save(record)) {
                        Ok(()) => {}
                        Err(error) => {
                            tracing::warn!(
                                conversation_id = %conversation_id,
                                error = %error,
                                "failed to persist conversation history"
                            );
                        }
                    }
                }
                Ok(Ok(TurnOutcome::Cancelled(_))) => {
                    state.pending_prompt = None;
                    state.updated_at_ms = now_timestamp_ms();
                    state.status = ThreadStatus::Idle;
                }
                Ok(Ok(TurnOutcome::Failed(_))) | Ok(Err(_)) | Err(_) => {
                    state.updated_at_ms = now_timestamp_ms();
                    state.status = ThreadStatus::Restartable;
                }
            }

            if let Err(error) = thread_repository.save_thread(thread_record(&conversation_id, state))
            {
                tracing::warn!(
                    conversation_id = %conversation_id,
                    error = %error,
                    "failed to persist conversation thread"
                );
            }
        });
    }

    async fn activate_conversation(
        &self,
        conversation_id: Option<String>,
    ) -> Result<(), TurnError> {
        self.thread_repository
            .set_active_thread(conversation_id.clone())?;
        *self.active_conversation_id.lock().await = conversation_id;
        Ok(())
    }

    async fn activate_conversation_best_effort(&self, conversation_id: Option<String>) {
        if let Err(error) = self.activate_conversation(conversation_id.clone()).await {
            tracing::warn!(
                conversation_id = ?conversation_id,
                error = %error,
                "failed to persist active conversation thread"
            );
        }
    }

    async fn persist_thread_state_best_effort(&self, conversation_id: &str) {
        let record = {
            let conversations = self.conversations.lock().await;
            conversations
                .get(conversation_id)
                .map(|state| thread_record(conversation_id, state))
        };

        if let Some(record) = record {
            if let Err(error) = self.thread_repository.save_thread(record) {
                tracing::warn!(
                    conversation_id = %conversation_id,
                    error = %error,
                    "failed to persist conversation thread"
                );
            }
        }
    }
}

fn conversation_state_from_record(record: ConversationRecord) -> (String, ConversationState) {
    let conversation_id = record.conversation_id.clone();
    let target_kind = record.target_kind.clone();
    let target_id = record.target_id.clone();
    let history = record.to_snapshot();
    let updated_at_ms = record.updated_at_ms;
    let title = title_from_history(&history, &target_id);

    (
        conversation_id,
        ConversationState {
            title,
            target_kind,
            target_id,
            history,
            pending_prompt: None,
            status: ThreadStatus::Idle,
            updated_at_ms,
            active_turn_id: None,
            active_controller: None,
        },
    )
}

fn thread_record(conversation_id: &str, state: &ConversationState) -> ConversationThreadRecord {
    ConversationThreadRecord {
        conversation_id: conversation_id.to_string(),
        title: state.title.clone(),
        target_kind: state.target_kind.clone(),
        target_id: state.target_id.clone(),
        updated_at_ms: state.updated_at_ms,
        pending_prompt: state.pending_prompt.clone(),
        status: state.status.clone(),
    }
}

fn thread_record_from_summary(summary: &ConversationThreadSummary) -> ConversationThreadRecord {
    ConversationThreadRecord {
        conversation_id: summary.conversation_id.clone(),
        title: summary.title.clone(),
        target_kind: summary.target_kind.clone(),
        target_id: summary.target_id.clone(),
        updated_at_ms: summary.updated_at_ms,
        pending_prompt: None,
        status: summary.status.clone(),
    }
}

fn normalize_thread_status(status: ThreadStatus) -> ThreadStatus {
    match status {
        ThreadStatus::Running => ThreadStatus::Restartable,
        other => other,
    }
}

fn title_from_history(history: &TurnMessageSnapshot, target_id: &str) -> String {
    history
        .iter()
        .find_map(|message| match message.as_ref() {
            TurnMessage::User { content } => Some(conversation_title(content)),
            _ => None,
        })
        .unwrap_or_else(|| target_id.to_string())
}

fn conversation_title(prompt: &str) -> String {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return "New Thread".to_string();
    }

    let first_line = trimmed.lines().next().unwrap_or(trimmed);
    let mut title = first_line.chars().take(48).collect::<String>();
    if title.is_empty() {
        title = "New Thread".to_string();
    }
    title
}

fn now_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Clone)]
pub struct DesktopConversationRuntime {
    event_sink: Arc<dyn ChatEventSink>,
    mode: Arc<DesktopRuntimeMode>,
}

impl DesktopConversationRuntime {
    pub fn from_environment(event_sink: Arc<dyn ChatEventSink>) -> Self {
        let mode = match default_tooling().and_then(|tooling| {
            let model = ProviderModelRunner::from_environment(&tooling.specs)?;
            Ok(DesktopRuntimeStack {
                model: Arc::new(model),
                tool_runner: tooling.runner,
                authorizer: Arc::new(AllowListAuthorizer::default()),
            })
        }) {
            Ok(stack) => DesktopRuntimeMode::Ready(stack),
            Err(error) => DesktopRuntimeMode::Unavailable(error.to_string()),
        };

        Self {
            event_sink,
            mode: Arc::new(mode),
        }
    }
}

enum DesktopRuntimeMode {
    Ready(DesktopRuntimeStack),
    Unavailable(String),
}

struct DesktopRuntimeStack {
    model: Arc<dyn ModelRunner>,
    tool_runner: Arc<dyn ToolRunner>,
    authorizer: Arc<dyn ToolAuthorizer>,
}

#[async_trait]
impl ConversationRuntime for DesktopConversationRuntime {
    async fn spawn_turn(
        &self,
        context: TurnContext,
        history: TurnMessageSnapshot,
    ) -> Result<RunningConversationTurn, TurnError> {
        let stack = match self.mode.as_ref() {
            DesktopRuntimeMode::Ready(stack) => stack,
            DesktopRuntimeMode::Unavailable(message) => {
                return Err(TurnError::Runtime(message.clone()));
            }
        };

        let observer = Arc::new(DesktopTurnObserver::new(
            context.session_id.clone(),
            context.turn_id.clone(),
            Arc::clone(&self.event_sink),
        ));
        let (handle, task) = TurnDriver::spawn_recording(
            context,
            history,
            Arc::clone(&stack.model),
            Arc::clone(&stack.tool_runner),
            Arc::clone(&stack.authorizer),
            observer,
        );
        let controller = Arc::new(DriverTurnController {
            controller: handle.controller(),
        });

        tokio::spawn(async move {
            while handle.next_event().await.is_some() {}
        });

        Ok(RunningConversationTurn { controller, task })
    }
}

struct DriverTurnController {
    controller: turn::TurnController,
}

#[async_trait]
impl ConversationTurnControl for DriverTurnController {
    async fn cancel(&self) -> Result<(), TurnError> {
        self.controller.cancel().await
    }
}
