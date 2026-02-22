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
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{info, warn};

use crate::storage::{FileSessionStore, FileTurnCheckpointStore, SessionFilter, SessionStore};

pub struct SessionRuntime<L, T>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
    T: agent_core::tools::ToolExecutor + Send + Sync + 'static,
{
    store: Arc<FileSessionStore>,
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

impl<L, T> SessionRuntime<L, T>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
    T: agent_core::tools::ToolExecutor + Send + Sync + 'static,
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
        let store = Arc::new(FileSessionStore::new(base_path));
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
        store: Arc<FileSessionStore>,
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
        store: Arc<FileSessionStore>,
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
            if let Err(err) = store.save_turn_summary(&session_id, &summary).await {
                warn!(
                    "failed to persist turn summary for session {} turn {}: {}",
                    session_id, turn_id, err
                );
            }
            if let Err(err) = store.update(&updated_info).await {
                warn!(
                    "failed to persist idle session state for session {} after turn {}: {}",
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
    T: agent_core::tools::ToolExecutor + Send + Sync + 'static,
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

    // Dummy implementations for testing
    struct MockModel;
    struct RecordingModel {
        requests: Arc<Mutex<Vec<ModelRequest>>>,
    }
    struct MockTools;

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
