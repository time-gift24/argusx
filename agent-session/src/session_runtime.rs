use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use agent_core::{
    new_id, AgentError, InputEnvelope, RunStreamEvent, Runtime, RuntimeStreams, SessionId,
    SessionInfo, SessionStatus, TranscriptItem, TurnContext, TurnId, TurnRequest, TurnStatus,
    TurnSummary,
};
use agent_turn::{TurnEngineConfig, TurnRuntime};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{info, warn};

use crate::storage::{
    FileSessionStore, FileTurnCheckpointStore, SessionArtifactStore, SessionFilter,
};

pub struct SessionRuntime<L, T>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
    T: agent_core::tools::ToolExecutor + agent_core::tools::ToolCatalog + Send + Sync + 'static,
{
    store: Arc<dyn SessionArtifactStore>,
    checkpoint_store: Arc<FileTurnCheckpointStore>,
    turn_runtime: Arc<TurnRuntime<L, T>>,
    sessions: Arc<RwLock<HashMap<SessionId, SessionState>>>,
}

#[derive(Clone)]
struct SessionState {
    info: SessionInfo,
    turns: Vec<TurnSummary>,
    current_turn_id: Option<TurnId>,
}

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub max_parallel_tools: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_parallel_tools: 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreCheckpointResult {
    pub restored_turn_id: String,
    pub removed_turn_ids: Vec<String>,
}

impl<L, T> SessionRuntime<L, T>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
    T: agent_core::tools::ToolExecutor + agent_core::tools::ToolCatalog + Send + Sync + 'static,
{
    pub fn new(base_path: PathBuf, model: Arc<L>, tools: Arc<T>) -> Self {
        Self::with_config(base_path, model, tools, SessionConfig::default())
    }

    pub fn with_config(
        base_path: PathBuf,
        model: Arc<L>,
        tools: Arc<T>,
        config: SessionConfig,
    ) -> Self {
        let store: Arc<dyn SessionArtifactStore> = Arc::new(FileSessionStore::new(base_path));
        Self::with_store_and_config(store, model, tools, config)
    }

    pub fn with_store(store: Arc<dyn SessionArtifactStore>, model: Arc<L>, tools: Arc<T>) -> Self {
        Self::with_store_and_config(store, model, tools, SessionConfig::default())
    }

    pub fn with_store_and_config(
        store: Arc<dyn SessionArtifactStore>,
        model: Arc<L>,
        tools: Arc<T>,
        config: SessionConfig,
    ) -> Self {
        let checkpoint_store = Arc::new(FileTurnCheckpointStore::new(Arc::clone(&store)));
        let turn_config = TurnEngineConfig {
            max_parallel_tools: config.max_parallel_tools,
            ..TurnEngineConfig::default()
        };
        let turn_runtime = Arc::new(
            TurnRuntime::new(model, tools, turn_config)
                .with_checkpoint_store(checkpoint_store.clone()),
        );

        Self {
            store,
            checkpoint_store,
            turn_runtime,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn ensure_session_loaded(&self, session_id: &SessionId) -> Result<SessionState> {
        // Check in-memory first
        {
            let sessions = self.sessions.read().await;
            if let Some(state) = sessions.get(session_id) {
                return Ok(state.clone());
            }
        }

        // Load from storage
        if let Some(info) = self.store.get(session_id).await? {
            let turns = self.store.list_turn_summaries(session_id).await?;
            let state = SessionState {
                info,
                turns,
                current_turn_id: None,
            };
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), state.clone());
            return Ok(state);
        }

        Err(anyhow!("Session not found: {}", session_id))
    }

    // Session management methods (not part of Runtime trait)
    pub async fn create_session(
        &self,
        user_id: Option<String>,
        title: Option<String>,
    ) -> Result<SessionId> {
        let session_id = new_id();
        let title = title.unwrap_or_else(|| format!("Session {}", &session_id[..8]));

        let mut info = SessionInfo::new(session_id.clone(), title);
        info.user_id = user_id;

        self.store.create(&info).await?;

        let state = SessionState {
            info,
            turns: Vec::new(),
            current_turn_id: None,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), state);

        info!("Created session: {}", session_id);
        Ok(session_id)
    }

    pub async fn list_sessions(&self, filter: SessionFilter) -> Result<Vec<SessionInfo>> {
        self.store.list(filter).await
    }

    pub async fn get_session(&self, session_id: &SessionId) -> Result<Option<SessionInfo>> {
        self.store.get(session_id).await
    }

    pub async fn rename_session(&self, session_id: &SessionId, title: String) -> Result<SessionInfo> {
        self.ensure_session_loaded(session_id).await?;
        let normalized_title = title.trim();
        if normalized_title.is_empty() {
            anyhow::bail!("session title cannot be empty");
        }

        let (previous_info, updated_info) = {
            let mut sessions = self.sessions.write().await;
            let state = sessions
                .get_mut(session_id)
                .ok_or_else(|| anyhow!("Session not found: {session_id}"))?;
            let previous_info = state.info.clone();
            state.info.title = normalized_title.to_string();
            state.info.updated_at = chrono::Utc::now().timestamp_millis();
            (previous_info, state.info.clone())
        };

        if let Err(err) = self.store.update(&updated_info).await {
            let mut sessions = self.sessions.write().await;
            if let Some(state) = sessions.get_mut(session_id) {
                state.info = previous_info;
            }
            return Err(err);
        }

        Ok(updated_info)
    }

    pub async fn list_turn_summaries(&self, session_id: &SessionId) -> Result<Vec<TurnSummary>> {
        self.ensure_session_loaded(session_id)
            .await
            .map(|state| state.turns)
    }

    pub async fn delete_session(&self, session_id: &SessionId) -> Result<()> {
        // Remove from memory
        {
            let mut sessions = self.sessions.write().await;
            sessions.remove(session_id);
        }

        // Delete from storage
        self.store.delete(session_id).await
    }

    pub async fn restore_session(&self, session_id: &SessionId) -> Result<SessionInfo> {
        self.ensure_session_loaded(session_id).await.map(|s| s.info)
    }

    pub async fn find_session_id_by_turn_id(&self, turn_id: &str) -> Result<Option<String>> {
        self.store.find_session_id_by_turn_id(turn_id).await
    }

    pub async fn restore_to_turn(
        &self,
        session_id: &SessionId,
        turn_id: &TurnId,
    ) -> Result<RestoreCheckpointResult> {
        self.ensure_session_loaded(session_id).await?;

        let restore_lock_turn_id = format!("__restore__{}", new_id());
        let (target_turn_id, updated_info, previous_info, kept_turns, previous_turns, removed_turn_ids) = {
            let mut sessions = self.sessions.write().await;
            let state = sessions
                .get_mut(session_id)
                .ok_or_else(|| anyhow!("Session not found: {session_id}"))?;

            if state.current_turn_id.is_some() {
                anyhow::bail!("Session {session_id} is busy");
            }

            let Some(target_index) = state
                .turns
                .iter()
                .position(|summary| summary.turn_id == *turn_id)
            else {
                anyhow::bail!("Turn {turn_id} not found in session {session_id}");
            };
            let target = state.turns[target_index].clone();

            if target.status != TurnStatus::Done {
                anyhow::bail!("Turn {turn_id} is not in done status and cannot be restored");
            }

            let removed_turn_ids: Vec<String> = state
                .turns
                .iter()
                .skip(target_index + 1)
                .map(|summary| summary.turn_id.clone())
                .collect();

            let previous_turns = state.turns.clone();
            let kept_turns = state.turns[..=target_index].to_vec();
            let previous_info = state.info.clone();
            let mut updated_info = previous_info.clone();
            updated_info.status = SessionStatus::Idle;
            updated_info.updated_at = chrono::Utc::now().timestamp_millis();

            // Hold a restore marker while touching storage so concurrent turns fail fast as "busy".
            state.current_turn_id = Some(restore_lock_turn_id.clone());

            (
                target.turn_id.clone(),
                updated_info,
                previous_info,
                kept_turns,
                previous_turns,
                removed_turn_ids,
            )
        };

        let persist_result: Result<Vec<String>> = async {
            let removed_on_disk = self.store.truncate_turns_after(session_id, turn_id).await?;
            if removed_turn_ids != removed_on_disk {
                warn!(
                    "restore_to_turn removed turn mismatch for session {} target {}: memory={:?} disk={:?}",
                    session_id, turn_id, removed_turn_ids, removed_on_disk
                );
            }
            self.store.update(&updated_info).await?;
            Ok(removed_on_disk)
        }
        .await;

        {
            let mut sessions = self.sessions.write().await;
            if let Some(state) = sessions.get_mut(session_id) {
                if state.current_turn_id.as_deref() == Some(restore_lock_turn_id.as_str()) {
                    match persist_result {
                        Ok(_) => {
                            state.turns = kept_turns;
                            state.info = updated_info.clone();
                            state.current_turn_id = None;
                        }
                        Err(_) => {
                            state.turns = previous_turns;
                            state.info = previous_info;
                            state.current_turn_id = None;
                        }
                    }
                } else {
                    warn!(
                        "restore marker lost for session {} while restoring turn {}",
                        session_id, turn_id
                    );
                }
            }
        }

        let removed_on_disk = persist_result?;

        Ok(RestoreCheckpointResult {
            restored_turn_id: target_turn_id,
            removed_turn_ids: removed_on_disk,
        })
    }

    async fn load_previous_transcript(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<TranscriptItem>, AgentError> {
        let last_turn_id = {
            let sessions = self.sessions.read().await;
            sessions
                .get(session_id)
                .and_then(|state| state.turns.last().map(|summary| summary.turn_id.clone()))
        };

        let Some(last_turn_id) = last_turn_id else {
            return Ok(Vec::new());
        };

        self.store
            .load_turn_transcript(session_id, &last_turn_id)
            .await
            .map_err(|e| AgentError::Internal {
                message: format!(
                    "failed to load previous transcript for session {session_id}, turn {last_turn_id}: {e}"
                ),
            })
    }

    fn build_turn_summary(
        event: &RunStreamEvent,
        turn_id: &TurnId,
        started_at: i64,
    ) -> Option<TurnSummary> {
        let ended_at = chrono::Utc::now().timestamp_millis();
        match event {
            RunStreamEvent::TurnDone {
                epoch,
                final_message,
                usage,
                stats,
                ..
            } => Some(TurnSummary {
                turn_id: turn_id.clone(),
                epoch: *epoch,
                started_at,
                ended_at: Some(ended_at),
                status: TurnStatus::Done,
                final_message: final_message.clone(),
                tool_calls_count: stats.tool_calls_count,
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
            }),
            RunStreamEvent::TurnFailed {
                epoch,
                usage,
                stats,
                cancelled,
                ..
            } => Some(TurnSummary {
                turn_id: turn_id.clone(),
                epoch: *epoch,
                started_at,
                ended_at: Some(ended_at),
                status: if *cancelled {
                    TurnStatus::Cancelled
                } else {
                    TurnStatus::Failed
                },
                final_message: None,
                tool_calls_count: stats.tool_calls_count,
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
            }),
            _ => None,
        }
    }

    async fn mark_turn_running(
        &self,
        session_id: &SessionId,
        turn_id: &TurnId,
    ) -> Result<(), AgentError> {
        let (updated_info, previous_status, previous_updated_at) = {
            let mut sessions = self.sessions.write().await;
            let state = sessions
                .get_mut(session_id)
                .ok_or_else(|| AgentError::Internal {
                    message: format!("Session not found: {session_id}"),
                })?;

            if state.current_turn_id.is_some() {
                return Err(AgentError::Internal {
                    message: format!("Session {session_id} is busy"),
                });
            }

            let previous_status = state.info.status;
            let previous_updated_at = state.info.updated_at;
            state.info.status = SessionStatus::Active;
            state.info.updated_at = chrono::Utc::now().timestamp_millis();
            state.current_turn_id = Some(turn_id.clone());
            (state.info.clone(), previous_status, previous_updated_at)
        };

        if let Err(e) = self.store.update(&updated_info).await {
            let mut sessions = self.sessions.write().await;
            if let Some(state) = sessions.get_mut(session_id) {
                if state.current_turn_id.as_deref() == Some(turn_id.as_str()) {
                    state.current_turn_id = None;
                    state.info.status = previous_status;
                    state.info.updated_at = previous_updated_at;
                }
            }
            return Err(AgentError::Internal {
                message: format!("failed to persist active session state: {e}"),
            });
        }

        Ok(())
    }

    async fn clear_active_turn_if_current(
        sessions: Arc<RwLock<HashMap<SessionId, SessionState>>>,
        store: Arc<dyn SessionArtifactStore>,
        session_id: SessionId,
        turn_id: TurnId,
    ) {
        let updated_info = {
            let mut guard = sessions.write().await;
            let Some(state) = guard.get_mut(&session_id) else {
                return;
            };

            if state.current_turn_id.as_deref() != Some(turn_id.as_str()) {
                return;
            }

            state.current_turn_id = None;
            state.info.status = SessionStatus::Idle;
            state.info.updated_at = chrono::Utc::now().timestamp_millis();
            state.info.clone()
        };

        if let Err(err) = store.update(&updated_info).await {
            warn!(
                "failed to persist idle session state for session {} after turn {}: {}",
                session_id, turn_id, err
            );
        }
    }

    async fn complete_turn_if_current(
        sessions: Arc<RwLock<HashMap<SessionId, SessionState>>>,
        store: Arc<dyn SessionArtifactStore>,
        session_id: SessionId,
        summary: TurnSummary,
    ) {
        let turn_id = summary.turn_id.clone();
        let (updated_info, should_persist) = {
            let mut guard = sessions.write().await;
            let Some(state) = guard.get_mut(&session_id) else {
                return;
            };

            if state.current_turn_id.as_deref() != Some(turn_id.as_str()) {
                return;
            }

            state.current_turn_id = None;
            state.turns.push(summary.clone());
            state.info.status = SessionStatus::Idle;
            state.info.updated_at = chrono::Utc::now().timestamp_millis();
            (state.info.clone(), true)
        };

        if should_persist {
            if let Err(err) = store
                .persist_turn_completion(&session_id, &summary, &updated_info)
                .await
            {
                warn!(
                    "failed to persist turn completion state for session {} turn {}: {}",
                    session_id, turn_id, err
                );
            }
        }
    }

    async fn rollback_turn_start(&self, session_id: &SessionId, turn_id: &TurnId) {
        Self::clear_active_turn_if_current(
            Arc::clone(&self.sessions),
            Arc::clone(&self.store),
            session_id.clone(),
            turn_id.clone(),
        )
        .await;
    }
}

#[async_trait]
impl<L, T> Runtime for SessionRuntime<L, T>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
    T: agent_core::tools::ToolExecutor + agent_core::tools::ToolCatalog + Send + Sync + 'static,
{
    async fn run_turn(&self, request: TurnRequest) -> Result<RuntimeStreams, AgentError> {
        let session_id = request.meta.session_id.clone();
        let turn_id = request.meta.turn_id.clone();

        // Ensure session exists (and is loaded into memory)
        self.ensure_session_loaded(&session_id)
            .await
            .map_err(|e| AgentError::Internal {
                message: format!("Session not found: {}", e),
            })?;

        let mut request = request;
        if request.transcript.is_empty() {
            request.transcript = self.load_previous_transcript(&session_id).await?;
        }

        let started_at = chrono::Utc::now().timestamp_millis();
        let context = TurnContext {
            turn_id: turn_id.clone(),
            session_id: session_id.clone(),
            epoch: 0,
            started_at,
        };

        self.mark_turn_running(&session_id, &turn_id).await?;

        if let Err(err) = self.store.save_turn_context(&context).await {
            self.rollback_turn_start(&session_id, &turn_id).await;
            return Err(AgentError::Internal {
                message: format!("failed to persist turn context: {err}"),
            });
        }

        if let Err(err) = self
            .checkpoint_store
            .register_turn(&session_id, &turn_id)
            .await
        {
            self.rollback_turn_start(&session_id, &turn_id).await;
            return Err(AgentError::Internal {
                message: format!("failed to register turn checkpoint: {err}"),
            });
        }

        // Run turn via TurnRuntime
        let streams = match self.turn_runtime.run_turn(request).await {
            Ok(streams) => streams,
            Err(err) => {
                self.rollback_turn_start(&session_id, &turn_id).await;
                return Err(err);
            }
        };

        let mut upstream_run = streams.run;
        let ui = streams.ui;
        let (run_tx, run_rx) = mpsc::unbounded_channel();
        let sessions = Arc::clone(&self.sessions);
        let store = Arc::clone(&self.store);
        let session_id_for_task = session_id.clone();
        let turn_id_for_task = turn_id.clone();

        tokio::spawn(async move {
            let mut cleaned = false;
            let mut pending_summary: Option<TurnSummary> = None;
            let mut downstream_closed = false;
            while let Some(event) = upstream_run.next().await {
                if pending_summary.is_none() {
                    pending_summary =
                        Self::build_turn_summary(&event, &turn_id_for_task, started_at);
                }

                if !downstream_closed && run_tx.send(event).is_err() {
                    downstream_closed = true;
                }
            }

            if let Some(summary) = pending_summary {
                cleaned = true;
                Self::complete_turn_if_current(
                    Arc::clone(&sessions),
                    Arc::clone(&store),
                    session_id_for_task.clone(),
                    summary,
                )
                .await;
            }

            if !cleaned {
                Self::clear_active_turn_if_current(
                    sessions,
                    store,
                    session_id_for_task,
                    turn_id_for_task,
                )
                .await;
            }
        });

        Ok(RuntimeStreams {
            run: Box::pin(UnboundedReceiverStream::new(run_rx)),
            ui,
        })
    }

    async fn inject_input(&self, turn_id: &str, input: InputEnvelope) -> Result<(), AgentError> {
        let turn_exists = {
            let sessions = self.sessions.read().await;
            sessions
                .values()
                .any(|state| state.current_turn_id.as_deref() == Some(turn_id))
        };

        if !turn_exists {
            return Err(AgentError::Internal {
                message: format!("Turn not found: {turn_id}"),
            });
        }

        self.turn_runtime.inject_input(turn_id, input).await
    }

    async fn cancel_turn(&self, turn_id: &str, reason: Option<String>) -> Result<(), AgentError> {
        self.turn_runtime.cancel_turn(turn_id, reason).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_core::{InputPart, ModelOutputEvent, ModelRequest, Runtime, SessionMeta};
    use async_trait::async_trait;
    use futures::{stream, StreamExt};
    use tempfile::TempDir;
    use tokio::sync::Mutex;
    use crate::storage::SessionStore;

    // Dummy implementations for testing
    struct MockModel;
    struct RecordingModel {
        requests: Arc<Mutex<Vec<ModelRequest>>>,
    }
    struct MockTools;
    struct FailingTruncateStore {
        inner: FileSessionStore,
    }

    impl FailingTruncateStore {
        fn new(base_path: PathBuf) -> Self {
            Self {
                inner: FileSessionStore::new(base_path),
            }
        }
    }

    #[async_trait]
    impl agent_core::LanguageModel for MockModel {
        fn model_name(&self) -> &str {
            "mock"
        }

        async fn stream(
            &self,
            _request: ModelRequest,
        ) -> Result<agent_core::ModelEventStream, AgentError> {
            Ok(Box::pin(stream::once(async {
                Ok(ModelOutputEvent::Completed { usage: None })
            })))
        }
    }

    #[async_trait]
    impl agent_core::LanguageModel for RecordingModel {
        fn model_name(&self) -> &str {
            "recording"
        }

        async fn stream(
            &self,
            request: ModelRequest,
        ) -> Result<agent_core::ModelEventStream, AgentError> {
            self.requests.lock().await.push(request);
            Ok(Box::pin(stream::once(async {
                Ok(ModelOutputEvent::Completed { usage: None })
            })))
        }
    }

    #[async_trait]
    impl agent_core::tools::ToolExecutor for MockTools {
        async fn execute_tool(
            &self,
            call: agent_core::ToolCall,
            _ctx: agent_core::tools::ToolExecutionContext,
        ) -> Result<agent_core::ToolResult, agent_core::tools::ToolExecutionError> {
            Ok(agent_core::ToolResult::ok(
                call.call_id,
                serde_json::json!({"result": "ok"}),
            ))
        }
    }

    #[async_trait]
    impl agent_core::tools::ToolCatalog for MockTools {
        async fn list_tools(&self) -> Vec<agent_core::tools::ToolSpec> {
            Vec::new()
        }

        async fn tool_spec(&self, _name: &str) -> Option<agent_core::tools::ToolSpec> {
            None
        }
    }

    #[async_trait]
    impl SessionStore for FailingTruncateStore {
        async fn create(&self, info: &SessionInfo) -> Result<()> {
            self.inner.create(info).await
        }

        async fn get(&self, session_id: &str) -> Result<Option<SessionInfo>> {
            self.inner.get(session_id).await
        }

        async fn update(&self, info: &SessionInfo) -> Result<()> {
            self.inner.update(info).await
        }

        async fn delete(&self, session_id: &str) -> Result<()> {
            self.inner.delete(session_id).await
        }

        async fn list(&self, filter: SessionFilter) -> Result<Vec<SessionInfo>> {
            self.inner.list(filter).await
        }
    }

    #[async_trait]
    impl SessionArtifactStore for FailingTruncateStore {
        async fn save_turn_context(&self, context: &TurnContext) -> Result<()> {
            self.inner.save_turn_context(context).await
        }

        async fn save_turn_summary(&self, session_id: &str, summary: &TurnSummary) -> Result<()> {
            self.inner.save_turn_summary(session_id, summary).await
        }

        async fn list_turn_summaries(&self, session_id: &str) -> Result<Vec<TurnSummary>> {
            self.inner.list_turn_summaries(session_id).await
        }

        async fn load_latest_turn_summary(&self, session_id: &str) -> Result<Option<TurnSummary>> {
            self.inner.load_latest_turn_summary(session_id).await
        }

        async fn delete_turn_artifacts(&self, session_id: &str, turn_id: &str) -> Result<()> {
            self.inner.delete_turn_artifacts(session_id, turn_id).await
        }

        async fn truncate_turns_after(
            &self,
            _session_id: &str,
            _restored_turn_id: &str,
        ) -> Result<Vec<String>> {
            anyhow::bail!("injected truncate failure")
        }

        async fn save_turn_transcript(
            &self,
            session_id: &str,
            turn_id: &str,
            items: &[TranscriptItem],
        ) -> Result<()> {
            self.inner.save_turn_transcript(session_id, turn_id, items).await
        }

        async fn load_turn_transcript(
            &self,
            session_id: &str,
            turn_id: &str,
        ) -> Result<Vec<TranscriptItem>> {
            self.inner.load_turn_transcript(session_id, turn_id).await
        }

        async fn find_session_id_by_turn_id(&self, turn_id: &str) -> Result<Option<String>> {
            self.inner.find_session_id_by_turn_id(turn_id).await
        }
    }

    #[tokio::test]
    async fn test_create_session() {
        let temp_dir = TempDir::new().unwrap();
        let runtime = SessionRuntime::new(
            temp_dir.path().to_path_buf(),
            Arc::new(MockModel),
            Arc::new(MockTools),
        );

        let session_id = runtime
            .create_session(None, Some("Test".into()))
            .await
            .unwrap();
        assert!(!session_id.is_empty());
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let runtime = SessionRuntime::new(
            temp_dir.path().to_path_buf(),
            Arc::new(MockModel),
            Arc::new(MockTools),
        );

        runtime
            .create_session(None, Some("Session 1".into()))
            .await
            .unwrap();
        runtime
            .create_session(None, Some("Session 2".into()))
            .await
            .unwrap();

        let sessions = runtime
            .list_sessions(SessionFilter::default())
            .await
            .unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_rename_session_persists_to_store() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let runtime = SessionRuntime::new(
            temp_dir.path().to_path_buf(),
            Arc::new(MockModel),
            Arc::new(MockTools),
        );

        let session_id = runtime
            .create_session(None, Some("Before".into()))
            .await
            .expect("create session");
        let updated = runtime
            .rename_session(&session_id, "After".into())
            .await
            .expect("rename session");
        assert_eq!(updated.title, "After");

        let reloaded = runtime
            .get_session(&session_id)
            .await
            .expect("load session")
            .expect("session exists");
        assert_eq!(reloaded.title, "After");
    }

    #[tokio::test]
    async fn test_run_turn_releases_busy_state_after_completion() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let runtime = SessionRuntime::new(
            temp_dir.path().to_path_buf(),
            Arc::new(MockModel),
            Arc::new(MockTools),
        );

        let session_id = runtime
            .create_session(None, Some("Test".into()))
            .await
            .expect("create session");

        let first_turn_id = new_id();
        let first_streams = runtime
            .run_turn(TurnRequest {
                meta: SessionMeta::new(session_id.clone(), first_turn_id),
                provider: "bigmodel".to_string(),
                model: "glm-5".to_string(),
                initial_input: InputEnvelope::user_text("hello"),
                transcript: Vec::new(),
            })
            .await
            .expect("run first turn");

        let first_events: Vec<_> = first_streams.run.collect().await;
        assert!(first_events
            .iter()
            .any(|event| matches!(event, RunStreamEvent::TurnDone { .. })));

        let second_streams = runtime
            .run_turn(TurnRequest {
                meta: SessionMeta::new(session_id.clone(), new_id()),
                provider: "bigmodel".to_string(),
                model: "glm-5".to_string(),
                initial_input: InputEnvelope::user_text("hello again"),
                transcript: Vec::new(),
            })
            .await
            .expect("run second turn");

        let second_events: Vec<_> = second_streams.run.collect().await;
        assert!(second_events
            .iter()
            .any(|event| matches!(event, RunStreamEvent::TurnDone { .. })));
    }

    #[tokio::test]
    async fn test_second_turn_includes_previous_transcript() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let requests = Arc::new(Mutex::new(Vec::<ModelRequest>::new()));
        let runtime = SessionRuntime::new(
            temp_dir.path().to_path_buf(),
            Arc::new(RecordingModel {
                requests: Arc::clone(&requests),
            }),
            Arc::new(MockTools),
        );

        let session_id = runtime
            .create_session(None, Some("History".into()))
            .await
            .expect("create session");

        runtime
            .run_turn(TurnRequest {
                meta: SessionMeta::new(session_id.clone(), new_id()),
                provider: "bigmodel".to_string(),
                model: "glm-5".to_string(),
                initial_input: InputEnvelope::user_text("first"),
                transcript: Vec::new(),
            })
            .await
            .expect("run first turn")
            .run
            .collect::<Vec<_>>()
            .await;

        runtime
            .run_turn(TurnRequest {
                meta: SessionMeta::new(session_id, new_id()),
                provider: "bigmodel".to_string(),
                model: "glm-5".to_string(),
                initial_input: InputEnvelope::user_text("second"),
                transcript: Vec::new(),
            })
            .await
            .expect("run second turn")
            .run
            .collect::<Vec<_>>()
            .await;

        let captured = requests.lock().await.clone();
        assert_eq!(captured.len(), 2);
        assert_eq!(captured[0].transcript.len(), 1);
        assert_eq!(captured[1].transcript.len(), 2);
        assert_eq!(
            first_user_text(&captured[1].transcript[0]),
            Some("first".to_string())
        );
    }

    #[tokio::test]
    async fn test_previous_transcript_survives_runtime_restart() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let runtime = SessionRuntime::new(
            temp_dir.path().to_path_buf(),
            Arc::new(MockModel),
            Arc::new(MockTools),
        );

        let session_id = runtime
            .create_session(None, Some("Persisted History".into()))
            .await
            .expect("create session");

        runtime
            .run_turn(TurnRequest {
                meta: SessionMeta::new(session_id.clone(), new_id()),
                provider: "bigmodel".to_string(),
                model: "glm-5".to_string(),
                initial_input: InputEnvelope::user_text("persist me"),
                transcript: Vec::new(),
            })
            .await
            .expect("run first turn")
            .run
            .collect::<Vec<_>>()
            .await;

        drop(runtime);

        let requests = Arc::new(Mutex::new(Vec::<ModelRequest>::new()));
        let runtime2 = SessionRuntime::new(
            temp_dir.path().to_path_buf(),
            Arc::new(RecordingModel {
                requests: Arc::clone(&requests),
            }),
            Arc::new(MockTools),
        );

        runtime2
            .run_turn(TurnRequest {
                meta: SessionMeta::new(session_id, new_id()),
                provider: "bigmodel".to_string(),
                model: "glm-5".to_string(),
                initial_input: InputEnvelope::user_text("new turn"),
                transcript: Vec::new(),
            })
            .await
            .expect("run second turn")
            .run
            .collect::<Vec<_>>()
            .await;

        let captured = requests.lock().await.clone();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].transcript.len(), 2);
        assert_eq!(
            first_user_text(&captured[0].transcript[0]),
            Some("persist me".to_string())
        );
    }

    #[tokio::test]
    async fn restore_to_turn_truncates_state_and_disk() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let runtime = SessionRuntime::new(
            temp_dir.path().to_path_buf(),
            Arc::new(MockModel),
            Arc::new(MockTools),
        );

        let session_id = runtime
            .create_session(None, Some("Restore".into()))
            .await
            .expect("create session");

        let summaries = vec![
            TurnSummary {
                turn_id: "t1".into(),
                epoch: 0,
                started_at: 1,
                ended_at: Some(11),
                status: TurnStatus::Done,
                final_message: Some("one".into()),
                tool_calls_count: 0,
                input_tokens: 1,
                output_tokens: 1,
            },
            TurnSummary {
                turn_id: "t2".into(),
                epoch: 0,
                started_at: 2,
                ended_at: Some(12),
                status: TurnStatus::Done,
                final_message: Some("two".into()),
                tool_calls_count: 0,
                input_tokens: 1,
                output_tokens: 1,
            },
            TurnSummary {
                turn_id: "t3".into(),
                epoch: 0,
                started_at: 3,
                ended_at: Some(13),
                status: TurnStatus::Done,
                final_message: Some("three".into()),
                tool_calls_count: 0,
                input_tokens: 1,
                output_tokens: 1,
            },
        ];
        seed_turn_state_and_artifacts(&runtime, &session_id, &summaries).await;

        let result = runtime
            .restore_to_turn(&session_id, &"t2".to_string())
            .await
            .expect("restore to t2");
        assert_eq!(result.restored_turn_id, "t2");
        assert_eq!(result.removed_turn_ids, vec!["t3".to_string()]);

        let sessions = runtime.sessions.read().await;
        let state = sessions.get(&session_id).expect("session state exists");
        assert_eq!(state.turns.len(), 2);
        assert_eq!(state.turns[0].turn_id, "t1");
        assert_eq!(state.turns[1].turn_id, "t2");
        drop(sessions);

        let persisted = runtime
            .store
            .list_turn_summaries(&session_id)
            .await
            .expect("list summaries");
        assert_eq!(persisted.len(), 2);
        assert_eq!(persisted[0].turn_id, "t1");
        assert_eq!(persisted[1].turn_id, "t2");

        let turns_path = temp_dir.path().join(&session_id).join("turns");
        assert!(turns_path.join("t1").exists());
        assert!(turns_path.join("t2").exists());
        assert!(!turns_path.join("t3").exists());
    }

    #[tokio::test]
    async fn restore_to_turn_rejects_non_done_target() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let runtime = SessionRuntime::new(
            temp_dir.path().to_path_buf(),
            Arc::new(MockModel),
            Arc::new(MockTools),
        );

        let session_id = runtime
            .create_session(None, Some("Restore Failed".into()))
            .await
            .expect("create session");

        let summaries = vec![
            TurnSummary {
                turn_id: "t1".into(),
                epoch: 0,
                started_at: 1,
                ended_at: Some(11),
                status: TurnStatus::Done,
                final_message: Some("one".into()),
                tool_calls_count: 0,
                input_tokens: 1,
                output_tokens: 1,
            },
            TurnSummary {
                turn_id: "t2".into(),
                epoch: 0,
                started_at: 2,
                ended_at: Some(12),
                status: TurnStatus::Failed,
                final_message: None,
                tool_calls_count: 0,
                input_tokens: 1,
                output_tokens: 1,
            },
        ];
        seed_turn_state_and_artifacts(&runtime, &session_id, &summaries).await;

        let err = runtime
            .restore_to_turn(&session_id, &"t2".to_string())
            .await
            .expect_err("restore should fail");
        assert!(err.to_string().contains("not in done status"));

        let persisted = runtime
            .store
            .list_turn_summaries(&session_id)
            .await
            .expect("list summaries");
        assert_eq!(persisted.len(), 2);
        assert_eq!(persisted[0].turn_id, "t1");
        assert_eq!(persisted[1].turn_id, "t2");
    }

    #[tokio::test]
    async fn restore_to_turn_rolls_back_memory_when_storage_truncate_fails() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let store = Arc::new(FailingTruncateStore::new(temp_dir.path().to_path_buf()));
        let runtime = SessionRuntime::with_store(store, Arc::new(MockModel), Arc::new(MockTools));

        let session_id = runtime
            .create_session(None, Some("Restore Rollback".into()))
            .await
            .expect("create session");

        let summaries = vec![
            TurnSummary {
                turn_id: "t1".into(),
                epoch: 0,
                started_at: 1,
                ended_at: Some(11),
                status: TurnStatus::Done,
                final_message: Some("one".into()),
                tool_calls_count: 0,
                input_tokens: 1,
                output_tokens: 1,
            },
            TurnSummary {
                turn_id: "t2".into(),
                epoch: 0,
                started_at: 2,
                ended_at: Some(12),
                status: TurnStatus::Done,
                final_message: Some("two".into()),
                tool_calls_count: 0,
                input_tokens: 1,
                output_tokens: 1,
            },
            TurnSummary {
                turn_id: "t3".into(),
                epoch: 0,
                started_at: 3,
                ended_at: Some(13),
                status: TurnStatus::Done,
                final_message: Some("three".into()),
                tool_calls_count: 0,
                input_tokens: 1,
                output_tokens: 1,
            },
        ];
        seed_turn_state_and_artifacts(&runtime, &session_id, &summaries).await;

        let err = runtime
            .restore_to_turn(&session_id, &"t2".to_string())
            .await
            .expect_err("restore should fail");
        assert!(err.to_string().contains("injected truncate failure"));

        let sessions = runtime.sessions.read().await;
        let state = sessions.get(&session_id).expect("session state exists");
        assert_eq!(state.turns.len(), 3);
        assert_eq!(state.turns[0].turn_id, "t1");
        assert_eq!(state.turns[1].turn_id, "t2");
        assert_eq!(state.turns[2].turn_id, "t3");
        assert!(state.current_turn_id.is_none());
    }

    async fn seed_turn_state_and_artifacts(
        runtime: &SessionRuntime<MockModel, MockTools>,
        session_id: &str,
        summaries: &[TurnSummary],
    ) {
        for summary in summaries {
            let context = TurnContext {
                turn_id: summary.turn_id.clone(),
                session_id: session_id.to_string(),
                epoch: summary.epoch,
                started_at: summary.started_at,
            };
            runtime
                .store
                .save_turn_context(&context)
                .await
                .expect("save turn context");
            runtime
                .store
                .save_turn_summary(session_id, summary)
                .await
                .expect("save turn summary");
            runtime
                .store
                .save_turn_transcript(
                    session_id,
                    &summary.turn_id,
                    &[TranscriptItem::assistant_message(summary.turn_id.clone())],
                )
                .await
                .expect("save turn transcript");
        }

        let mut sessions = runtime.sessions.write().await;
        let state = sessions
            .get_mut(session_id)
            .expect("session state exists for seeding");
        state.turns = summaries.to_vec();
        state.current_turn_id = None;
    }

    fn first_user_text(item: &TranscriptItem) -> Option<String> {
        let TranscriptItem::UserMessage { input, .. } = item else {
            return None;
        };

        input.parts.iter().find_map(|part| match part {
            InputPart::Text { text } => Some(text.clone()),
            InputPart::Json { .. } => None,
        })
    }
}
