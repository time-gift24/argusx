//! Desktop chat control layer.
//!
//! This module centralizes desktop chat control behavior, including:
//! - Active thread bootstrap
//! - Turn routing via turn_id
//! - Permission resolution routing

use std::sync::Arc;

use async_trait::async_trait;
use session::manager::SessionManager;
use tauri::AppHandle;
use turn::{TurnError, TurnObserver};
use uuid::Uuid;

use crate::chat::{
    submission::{PermissionDecision, PromptInput, ThreadCreated, ThreadHistoryLoaded, ThreadSwitched, TurnStarted},
    TauriTurnObserver, TurnManager, TurnTargetKind,
};

use super::submission::Submission;
use crate::session_commands::DesktopSessionState;

/// Errors that can occur in the chat control layer.
#[derive(Debug)]
pub enum ControlError {
    /// Invalid input provided.
    InvalidInput(String),
    /// The requested turn was not found.
    NotFound(String),
    /// A conflict occurred (e.g., active turn already running).
    Conflict(String),
    /// The operation is unavailable.
    Unavailable(String),
    /// An internal error occurred.
    Internal(String),
}

impl std::fmt::Display for ControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            Self::NotFound(msg) => write!(f, "Not found: {}", msg),
            Self::Conflict(msg) => write!(f, "Conflict: {}", msg),
            Self::Unavailable(msg) => write!(f, "Unavailable: {}", msg),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for ControlError {}

/// Result type for control operations.
pub type ControlResult<T> = Result<T, ControlError>;

/// ChatController owns desktop control flow for chat operations.
///
/// It composes existing SessionManager operations and manages
/// turn_id -> thread_id routing through TurnManager.
pub struct ChatController<'a> {
    session_manager: Arc<SessionManager>,
    turn_manager: Arc<TurnManager>,
    desktop_state: &'a DesktopSessionState,
}

impl<'a> ChatController<'a> {
    /// Create a new ChatController.
    pub fn new(
        session_manager: Arc<SessionManager>,
        turn_manager: Arc<TurnManager>,
        desktop_state: &'a DesktopSessionState,
    ) -> Self {
        Self {
            session_manager,
            turn_manager,
            desktop_state,
        }
    }

    /// Load the active chat thread and its history.
    pub async fn load_active_thread(&self) -> ControlResult<ThreadHistoryLoaded> {
        let thread_id = self
            .desktop_state
            .ensure_active_chat_thread()
            .await
            .map_err(|e| ControlError::Internal(e.to_string()))?;

        let history = self
            .session_manager
            .load_thread_history(thread_id)
            .await
            .map_err(|e| ControlError::Internal(e.to_string()))?;

        let turns = history
            .iter()
            .map(|turn| crate::session_commands::hydrate_chat_turn(turn))
            .collect();

        Ok(ThreadHistoryLoaded { thread_id, turns })
    }

    /// Submit a control action to the chat controller.
    pub async fn submit(&self, submission: Submission) -> ControlResult<SubmissionResult> {
        match submission {
            Submission::Prompt { text } => {
                let result = self.start_prompt_turn(text, None).await?;
                Ok(SubmissionResult::TurnStarted(result))
            }
            Submission::NewThread { title } => {
                let result = self.create_thread(title).await?;
                Ok(SubmissionResult::ThreadCreated(result))
            }
            Submission::SwitchThread { thread_id } => {
                let result = self.switch_thread(thread_id).await?;
                Ok(SubmissionResult::ThreadSwitched(result))
            }
            Submission::CancelTurn { turn_id } => {
                self.cancel_turn(turn_id).await?;
                Ok(SubmissionResult::Cancelled)
            }
            Submission::ResolvePermission {
                turn_id,
                request_id,
                decision,
            } => {
                self.resolve_permission(turn_id, request_id, decision)
                    .await?;
                Ok(SubmissionResult::PermissionResolved)
            }
        }
    }

    /// Start a new prompt turn on the active thread.
    ///
    /// Note: This method requires AppHandle for observer creation.
    /// Use `start_prompt_turn_with_app` instead when AppHandle is available.
    #[allow(unused)]
    pub async fn start_prompt_turn(
        &self,
        _text: String,
        _prompt_input: Option<PromptInput>,
    ) -> ControlResult<TurnStarted> {
        // Ensure active thread exists
        let _thread_id = self
            .desktop_state
            .ensure_active_chat_thread()
            .await
            .map_err(|e| ControlError::Internal(e.to_string()))?;

        // Allocate turn_id
        let _turn_id = Uuid::new_v4();

        // Get target info from prompt_input or use defaults
        let (_target_kind, _target_id) = if let Some(input) = _prompt_input {
            (input.target_kind, input.target_id)
        } else {
            (TurnTargetKind::Agent, String::new())
        };

        // Build observer - we need AppHandle which requires a different approach
        // For now, we'll return an error if AppHandle is not available
        // The actual implementation will need AppHandle passed in differently
        // Let me check how to handle this...

        // Actually, looking at the existing code in commands.rs, the observer is built
        // with an AppHandle which comes from the Tauri command. We need a way to
        // inject this. Let's create a method that takes AppHandle.

        Err(ControlError::Internal(
            "start_prompt_turn requires AppHandle - use start_prompt_turn_with_app".to_string(),
        ))
    }

    /// Start a new prompt turn with AppHandle for observer creation.
    pub async fn start_prompt_turn_with_app(
        &self,
        text: String,
        app: AppHandle,
        prompt_input: Option<PromptInput>,
    ) -> ControlResult<TurnStarted> {
        // Ensure active thread exists
        let thread_id = self
            .desktop_state
            .ensure_active_chat_thread()
            .await
            .map_err(|e| ControlError::Internal(e.to_string()))?;

        // Allocate turn_id
        let turn_id = Uuid::new_v4();

        // Get target info from prompt_input or use defaults
        let (target_kind, target_id) = if let Some(input) = prompt_input {
            (input.target_kind, input.target_id)
        } else {
            (TurnTargetKind::Agent, String::new())
        };

        // Build observer with AppHandle
        let observer: Arc<dyn TurnObserver> = Arc::new(TauriTurnObserver::new(
            app,
            turn_id.to_string(),
            target_kind,
            target_id,
        ));

        // Build turn dependencies
        let deps = self
            .desktop_state
            .build_turn_dependencies(observer)
            .map_err(|e| ControlError::Unavailable(e.to_string()))?;

        // Start the turn
        self.session_manager
            .send_message_with_turn_id(thread_id, turn_id, text, deps)
            .await
            .map_err(|e| ControlError::Internal(e.to_string()))?;

        // Register turn_id -> thread_id mapping
        self.turn_manager.insert(turn_id.to_string(), thread_id).await;

        Ok(TurnStarted {
            turn_id: turn_id.to_string(),
        })
    }

    /// Create a new chat thread.
    pub async fn create_thread(&self, title: Option<String>) -> ControlResult<ThreadCreated> {
        let thread_id = self
            .session_manager
            .create_thread(title)
            .await
            .map_err(|e| ControlError::Internal(e.to_string()))?;

        Ok(ThreadCreated { thread_id })
    }

    /// Switch to a different thread.
    pub async fn switch_thread(&self, thread_id: Uuid) -> ControlResult<ThreadSwitched> {
        self.session_manager
            .switch_thread(thread_id)
            .await
            .map_err(|e| ControlError::Internal(e.to_string()))?;

        Ok(ThreadSwitched { thread_id })
    }

    /// Cancel a turn by turn_id.
    pub async fn cancel_turn(&self, turn_id: String) -> ControlResult<()> {
        let thread_id = self
            .turn_manager
            .get(&turn_id)
            .await
            .ok_or_else(|| ControlError::NotFound(format!("turn `{turn_id}` not found")))?;

        self.session_manager
            .cancel_turn(thread_id)
            .await
            .map_err(|e| ControlError::Internal(e.to_string()))?;

        Ok(())
    }

    /// Resolve a permission request for a turn.
    pub async fn resolve_permission(
        &self,
        turn_id: String,
        request_id: String,
        decision: PermissionDecision,
    ) -> ControlResult<()> {
        let thread_id = self
            .turn_manager
            .get(&turn_id)
            .await
            .ok_or_else(|| ControlError::NotFound(format!("turn `{turn_id}` not found")))?;

        self.session_manager
            .resolve_permission(thread_id, request_id, decision)
            .await
            .map_err(|e| ControlError::Internal(e.to_string()))?;

        Ok(())
    }
}

/// Results from submitting a control action.
#[derive(Debug)]
pub enum SubmissionResult {
    TurnStarted(TurnStarted),
    ThreadCreated(ThreadCreated),
    ThreadSwitched(ThreadSwitched),
    Cancelled,
    PermissionResolved,
}

/// Trait for building turn dependencies - implemented by DesktopSessionState.
#[async_trait]
pub trait TurnDependencyBuilder {
    async fn build_turn_dependencies(
        &self,
        observer: Arc<dyn TurnObserver>,
    ) -> Result<session::manager::TurnDependencies, TurnError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_error_display_formats_correctly() {
        let err = ControlError::NotFound("turn not found".to_string());
        assert_eq!(err.to_string(), "Not found: turn not found");

        let err = ControlError::Unavailable("provider not configured".to_string());
        assert_eq!(err.to_string(), "Unavailable: provider not configured");
    }

    #[test]
    fn control_error_variants() {
        let err = ControlError::InvalidInput("invalid UUID".to_string());
        assert!(matches!(err, ControlError::InvalidInput(_)));

        let err = ControlError::NotFound("missing".to_string());
        assert!(matches!(err, ControlError::NotFound(_)));

        let err = ControlError::Conflict("already running".to_string());
        assert!(matches!(err, ControlError::Conflict(_)));

        let err = ControlError::Unavailable("no provider".to_string());
        assert!(matches!(err, ControlError::Unavailable(_)));

        let err = ControlError::Internal("panic".to_string());
        assert!(matches!(err, ControlError::Internal(_)));
    }

    #[test]
    fn submission_result_variants() {
        let _ = SubmissionResult::Cancelled;
        let _ = SubmissionResult::PermissionResolved;
    }

    #[test]
    fn submission_prompt_variant() {
        use crate::chat::submission::Submission;
        let submission = Submission::Prompt {
            text: "Hello world".to_string(),
        };
        assert!(matches!(submission, Submission::Prompt { text } if text == "Hello world"));
    }

    #[test]
    fn submission_new_thread_variant() {
        use crate::chat::submission::Submission;
        let submission = Submission::NewThread {
            title: Some("My Thread".to_string()),
        };
        assert!(matches!(submission, Submission::NewThread { title: Some(t) } if t == "My Thread"));
    }

    #[test]
    fn submission_switch_thread_variant() {
        use crate::chat::submission::Submission;
        let thread_id = uuid::Uuid::new_v4();
        let submission = Submission::SwitchThread { thread_id };
        assert!(matches!(submission, Submission::SwitchThread { thread_id: id } if id == thread_id));
    }

    #[test]
    fn submission_cancel_turn_variant() {
        use crate::chat::submission::Submission;
        let submission = Submission::CancelTurn {
            turn_id: "turn-123".to_string(),
        };
        assert!(matches!(submission, Submission::CancelTurn { turn_id } if turn_id == "turn-123"));
    }

    #[test]
    fn submission_resolve_permission_variant() {
        use crate::chat::submission::{PermissionDecision, Submission};
        let submission = Submission::ResolvePermission {
            turn_id: "turn-456".to_string(),
            request_id: "req-789".to_string(),
            decision: PermissionDecision::Allow,
        };
        assert!(matches!(
            submission,
            Submission::ResolvePermission { turn_id, request_id, decision }
            if turn_id == "turn-456" && request_id == "req-789" && decision == PermissionDecision::Allow
        ));
    }

    #[test]
    fn turn_started_result_contains_turn_id() {
        let result = TurnStarted {
            turn_id: "test-turn-id".to_string(),
        };
        assert_eq!(result.turn_id, "test-turn-id");
    }

    #[test]
    fn thread_created_result_contains_thread_id() {
        let thread_id = uuid::Uuid::new_v4();
        let result = ThreadCreated { thread_id };
        assert_eq!(result.thread_id, thread_id);
    }

    #[test]
    fn thread_switched_result_contains_thread_id() {
        let thread_id = uuid::Uuid::new_v4();
        let result = ThreadSwitched { thread_id };
        assert_eq!(result.thread_id, thread_id);
    }
}
