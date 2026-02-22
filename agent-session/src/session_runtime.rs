use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use agent_core::{
    new_id, AgentError, InputEnvelope, ModelRequest, Runtime, RuntimeEvent, RuntimeStreams,
    SessionId, SessionInfo, SessionStatus, TurnContext, TurnId, TurnRequest,
};
use agent_turn::effect::ToolExecutor;
use agent_turn::{TurnEngineConfig, TurnRuntime};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::info;

use crate::storage::{FileSessionStore, SessionFilter, SessionStore};

pub struct SessionRuntime<L, T>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
    T: ToolExecutor + Send + Sync + 'static,
{
    store: Arc<FileSessionStore>,
    turn_runtime: Arc<TurnRuntime<L, T>>,
    sessions: Arc<RwLock<HashMap<SessionId, SessionState>>>,
    config: SessionConfig,
}

#[derive(Clone)]
struct SessionState {
    info: SessionInfo,
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
    T: ToolExecutor + Send + Sync + 'static,
{
    pub fn new(base_path: PathBuf, model: Arc<L>, tools: Arc<T>) -> Self {
        let store = Arc::new(FileSessionStore::new(base_path));
        let turn_config = TurnEngineConfig::default();
        let turn_runtime = Arc::new(TurnRuntime::new(model, tools, turn_config));

        Self {
            store,
            turn_runtime,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            config: SessionConfig::default(),
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
            let state = SessionState {
                info,
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
}

#[async_trait]
impl<L, T> Runtime for SessionRuntime<L, T>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
    T: ToolExecutor + Send + Sync + 'static,
{
    async fn run_turn(&self, request: TurnRequest) -> Result<RuntimeStreams, AgentError> {
        let session_id = request.meta.session_id.clone();
        let turn_id = request.meta.turn_id.clone();

        // Ensure session is loaded
        let state = self.ensure_session_loaded(&session_id).await.map_err(|e| {
            AgentError::Internal {
                message: format!("Session not found: {}", e),
            }
        })?;

        // Check if session is busy
        if state.current_turn_id.is_some() {
            return Err(AgentError::Internal {
                message: format!("Session {} is busy", session_id),
            });
        }

        // Update session status to Active
        {
            let mut sessions = self.sessions.write().await;
            if let Some(state) = sessions.get_mut(&session_id) {
                state.info.status = SessionStatus::Active;
                state.info.updated_at = chrono::Utc::now().timestamp_millis();
                state.current_turn_id = Some(turn_id.clone());
                self.store.update(&state.info).await.map_err(|e| {
                    AgentError::Internal {
                        message: e.to_string(),
                    }
                })?;
            }
        }

        // Run turn via TurnRuntime
        let streams = self.turn_runtime.run_turn(request).await?;

        Ok(streams)
    }

    async fn inject_input(&self, turn_id: &str, input: InputEnvelope) -> Result<(), AgentError> {
        // Find session with this turn and inject
        let sessions = self.sessions.read().await;
        for (session_id, state) in sessions.iter() {
            if let Some(ref current_turn_id) = state.current_turn_id {
                if current_turn_id == turn_id {
                    // Delegate to turn runtime
                    return self.turn_runtime.inject_input(turn_id, input).await;
                }
            }
        }
        Err(AgentError::Internal {
            message: format!("Turn not found: {}", turn_id),
        })
    }

    async fn cancel_turn(&self, turn_id: &str, reason: Option<String>) -> Result<(), AgentError> {
        // Delegate to turn runtime
        self.turn_runtime
            .cancel_turn(&turn_id.to_string(), reason)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use tempfile::TempDir;

    // Dummy implementations for testing
    struct MockModel;
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
            Ok(Box::pin(futures::stream::empty()))
        }
    }

    #[async_trait]
    impl ToolExecutor for MockTools {
        async fn execute_tool(
            &self,
            _call: agent_core::ToolCall,
            _epoch: u64,
        ) -> Result<serde_json::Value, String> {
            Ok(serde_json::json!({"result": "ok"}))
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

        let sessions = runtime.list_sessions(SessionFilter::default()).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }
}
