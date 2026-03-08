use std::sync::Arc;

use tauri::{Emitter, Manager, State};

use super::{
    DesktopConversationRuntime,
    checkpoints::{
        ConversationCheckpointRepository, ConversationCheckpointSummary,
        CreateConversationCheckpointInput, InMemoryConversationCheckpointRepository,
        JsonConversationCheckpointRepository, RestoreConversationCheckpointInput,
    },
    compression::ThresholdConversationCompressionPolicy,
    events::{StartConversationInput, TURN_EVENT_NAME},
    manager::ConversationManager,
    observer::ChatEventSink,
    storage::{ConversationRepository, InMemoryConversationRepository, JsonConversationRepository},
    threads::{ConversationThreadRepository, InMemoryConversationThreadRepository, JsonConversationThreadRepository},
    CancelConversationInput, ContinueConversationInput, ConversationThreadSummary,
    ConversationTurnStarted, CreateConversationThreadInput, RestartConversationInput,
    SwitchConversationThreadInput,
};

pub struct ChatState {
    manager: ConversationManager,
}

impl ChatState {
    pub fn new(manager: ConversationManager) -> Self {
        Self { manager }
    }

    pub fn from_app(app: tauri::AppHandle) -> Self {
        let sink = Arc::new(TauriEventSink { app: app.clone() });
        let runtime: Arc<dyn super::ConversationRuntime> =
            Arc::new(DesktopConversationRuntime::from_environment(sink));
        let repository = build_repository(&app);
        let thread_repository = build_thread_repository(&app);
        let checkpoint_repository = build_checkpoint_repository(&app);
        let compression = Arc::new(ThresholdConversationCompressionPolicy::new(32, 12));
        let manager = ConversationManager::new_with_repositories(
            Arc::clone(&runtime),
            repository,
            compression,
            thread_repository,
            checkpoint_repository,
        )
        .unwrap_or_else(|error| {
            tracing::warn!(error = %error, "falling back to in-memory chat repository");
            ConversationManager::new(runtime)
        });

        Self::new(manager)
    }

    pub fn manager(&self) -> &ConversationManager {
        &self.manager
    }
}

pub async fn start_conversation_with_manager(
    manager: &ConversationManager,
    input: StartConversationInput,
) -> Result<ConversationTurnStarted, String> {
    manager
        .start_conversation(input)
        .await
        .map_err(|error| error.to_string())
}

pub async fn continue_conversation_with_manager(
    manager: &ConversationManager,
    input: ContinueConversationInput,
) -> Result<ConversationTurnStarted, String> {
    manager
        .continue_conversation(input)
        .await
        .map_err(|error| error.to_string())
}

pub async fn cancel_conversation_with_manager(
    manager: &ConversationManager,
    conversation_id: String,
) -> Result<(), String> {
    manager
        .cancel_conversation(conversation_id)
        .await
        .map_err(|error| error.to_string())
}

pub async fn create_thread_with_manager(
    manager: &ConversationManager,
    input: CreateConversationThreadInput,
) -> Result<ConversationThreadSummary, String> {
    manager
        .create_thread(input)
        .await
        .map_err(|error| error.to_string())
}

pub async fn list_threads_with_manager(
    manager: &ConversationManager,
) -> Result<Vec<ConversationThreadSummary>, String> {
    manager
        .list_threads()
        .await
        .map_err(|error| error.to_string())
}

pub async fn switch_thread_with_manager(
    manager: &ConversationManager,
    input: SwitchConversationThreadInput,
) -> Result<ConversationThreadSummary, String> {
    manager
        .switch_thread(input.conversation_id)
        .await
        .map_err(|error| error.to_string())
}

pub async fn create_checkpoint_with_manager(
    manager: &ConversationManager,
    input: CreateConversationCheckpointInput,
) -> Result<ConversationCheckpointSummary, String> {
    manager
        .create_checkpoint(input)
        .await
        .map_err(|error| error.to_string())
}

pub async fn restore_checkpoint_with_manager(
    manager: &ConversationManager,
    input: RestoreConversationCheckpointInput,
) -> Result<ConversationThreadSummary, String> {
    manager
        .restore_checkpoint(input)
        .await
        .map_err(|error| error.to_string())
}

pub async fn restart_conversation_with_manager(
    manager: &ConversationManager,
    input: RestartConversationInput,
) -> Result<ConversationTurnStarted, String> {
    manager
        .restart_conversation(input)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn start_conversation(
    state: State<'_, ChatState>,
    input: StartConversationInput,
) -> Result<ConversationTurnStarted, String> {
    start_conversation_with_manager(state.manager(), input).await
}

#[tauri::command]
pub async fn continue_conversation(
    state: State<'_, ChatState>,
    input: ContinueConversationInput,
) -> Result<ConversationTurnStarted, String> {
    continue_conversation_with_manager(state.manager(), input).await
}

#[tauri::command]
pub async fn cancel_conversation(
    state: State<'_, ChatState>,
    input: CancelConversationInput,
) -> Result<(), String> {
    cancel_conversation_with_manager(state.manager(), input.conversation_id).await
}

#[tauri::command]
pub async fn create_conversation_thread(
    state: State<'_, ChatState>,
    input: CreateConversationThreadInput,
) -> Result<ConversationThreadSummary, String> {
    create_thread_with_manager(state.manager(), input).await
}

#[tauri::command]
pub async fn list_conversation_threads(
    state: State<'_, ChatState>,
) -> Result<Vec<ConversationThreadSummary>, String> {
    list_threads_with_manager(state.manager()).await
}

#[tauri::command]
pub async fn switch_conversation_thread(
    state: State<'_, ChatState>,
    input: SwitchConversationThreadInput,
) -> Result<ConversationThreadSummary, String> {
    switch_thread_with_manager(state.manager(), input).await
}

#[tauri::command]
pub async fn create_conversation_checkpoint(
    state: State<'_, ChatState>,
    input: CreateConversationCheckpointInput,
) -> Result<ConversationCheckpointSummary, String> {
    create_checkpoint_with_manager(state.manager(), input).await
}

#[tauri::command]
pub async fn restore_conversation_checkpoint(
    state: State<'_, ChatState>,
    input: RestoreConversationCheckpointInput,
) -> Result<ConversationThreadSummary, String> {
    restore_checkpoint_with_manager(state.manager(), input).await
}

#[tauri::command]
pub async fn restart_conversation(
    state: State<'_, ChatState>,
    input: RestartConversationInput,
) -> Result<ConversationTurnStarted, String> {
    restart_conversation_with_manager(state.manager(), input).await
}

struct TauriEventSink {
    app: tauri::AppHandle,
}

impl ChatEventSink for TauriEventSink {
    fn emit(&self, event: &super::DesktopTurnEvent) -> Result<(), turn::TurnError> {
        self.app
            .emit(TURN_EVENT_NAME, event.clone())
            .map_err(|error| turn::TurnError::Runtime(error.to_string()))
    }
}

fn build_repository(app: &tauri::AppHandle) -> Arc<dyn ConversationRepository> {
    let path = app
        .path()
        .app_data_dir()
        .map(|dir| dir.join("chat").join("conversations.json"));

    match path {
        Ok(path) => match JsonConversationRepository::new(path) {
            Ok(repository) => Arc::new(repository),
            Err(error) => {
                tracing::warn!(error = %error, "failed to initialize json chat repository");
                Arc::new(InMemoryConversationRepository::default())
            }
        },
        Err(error) => {
            tracing::warn!(error = %error, "failed to resolve app data directory for chat repository");
            Arc::new(InMemoryConversationRepository::default())
        }
    }
}

fn build_thread_repository(app: &tauri::AppHandle) -> Arc<dyn ConversationThreadRepository> {
    let path = app
        .path()
        .app_data_dir()
        .map(|dir| dir.join("chat").join("threads.json"));

    match path {
        Ok(path) => match JsonConversationThreadRepository::new(path) {
            Ok(repository) => Arc::new(repository),
            Err(error) => {
                tracing::warn!(error = %error, "failed to initialize json chat thread repository");
                Arc::new(InMemoryConversationThreadRepository::default())
            }
        },
        Err(error) => {
            tracing::warn!(error = %error, "failed to resolve app data directory for chat thread repository");
            Arc::new(InMemoryConversationThreadRepository::default())
        }
    }
}

fn build_checkpoint_repository(
    app: &tauri::AppHandle,
) -> Arc<dyn ConversationCheckpointRepository> {
    let path = app
        .path()
        .app_data_dir()
        .map(|dir| dir.join("chat").join("checkpoints.json"));

    match path {
        Ok(path) => match JsonConversationCheckpointRepository::new(path) {
            Ok(repository) => Arc::new(repository),
            Err(error) => {
                tracing::warn!(error = %error, "failed to initialize json checkpoint repository");
                Arc::new(InMemoryConversationCheckpointRepository::default())
            }
        },
        Err(error) => {
            tracing::warn!(error = %error, "failed to resolve app data directory for checkpoint repository");
            Arc::new(InMemoryConversationCheckpointRepository::default())
        }
    }
}
