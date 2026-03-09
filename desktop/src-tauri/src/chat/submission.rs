//! Typed desktop chat control actions.
//!
//! This module normalizes raw user intent into typed control actions,
//! separating "what the user intends" from "how Tauri exposes it".

use uuid::Uuid;

pub use turn::PermissionDecision;

use crate::chat::TurnTargetKind;

/// Desktop chat control actions.
///
/// These are the supported operations that the control layer can execute.
/// Deferred variants (Compact, Undo, Resume, Summarize, Suggest, Heartbeat)
/// are not included because they lack backing runtime support.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Submission {
    /// Submit a new prompt to the active thread.
    Prompt {
        /// The text content of the prompt.
        text: String,
    },
    /// Create a new chat thread.
    NewThread {
        /// Optional title for the new thread.
        title: Option<String>,
    },
    /// Switch to a different thread.
    SwitchThread {
        /// The ID of the thread to switch to.
        thread_id: Uuid,
    },
    /// Cancel an in-progress turn.
    CancelTurn {
        /// The turn ID to cancel.
        turn_id: String,
    },
    /// Resolve a permission request for a turn.
    ResolvePermission {
        /// The turn ID associated with the permission request.
        turn_id: String,
        /// The permission request ID.
        request_id: String,
        /// The permission decision.
        decision: PermissionDecision,
    },
}

/// Input for starting a prompt turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptInput {
    /// The prompt text.
    pub text: String,
    /// The target kind (agent or workflow).
    pub target_kind: TurnTargetKind,
    /// The target ID.
    pub target_id: String,
}

/// Result of starting a turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnStarted {
    /// The allocated turn ID.
    pub turn_id: String,
}

/// Result of creating a thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadCreated {
    /// The ID of the created thread.
    pub thread_id: Uuid,
}

/// Result of loading thread history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadHistoryLoaded {
    /// The thread ID.
    pub thread_id: Uuid,
    /// The turns in the thread history.
    pub turns: Vec<crate::chat::HydratedChatTurn>,
}

/// Result of switching threads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadSwitched {
    /// The ID of the thread switched to.
    pub thread_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submission_prompt_variant() {
        let submission = Submission::Prompt {
            text: "Hello".into(),
        };
        assert!(matches!(submission, Submission::Prompt { text } if text == "Hello"));
    }

    #[test]
    fn submission_new_thread_with_title() {
        let submission = Submission::NewThread {
            title: Some("My Thread".into()),
        };
        assert!(matches!(submission, Submission::NewThread { title: Some(t) } if t == "My Thread"));
    }

    #[test]
    fn submission_new_thread_without_title() {
        let submission = Submission::NewThread { title: None };
        assert!(matches!(submission, Submission::NewThread { title: None }));
    }

    #[test]
    fn submission_switch_thread() {
        let thread_id = Uuid::new_v4();
        let submission = Submission::SwitchThread { thread_id };
        assert!(matches!(submission, Submission::SwitchThread { thread_id: id } if id == thread_id));
    }

    #[test]
    fn submission_cancel_turn() {
        let submission = Submission::CancelTurn {
            turn_id: "turn-123".into(),
        };
        assert!(matches!(submission, Submission::CancelTurn { turn_id } if turn_id == "turn-123"));
    }

    #[test]
    fn submission_resolve_permission_allow() {
        let submission = Submission::ResolvePermission {
            turn_id: "turn-456".into(),
            request_id: "req-789".into(),
            decision: PermissionDecision::Allow,
        };
        assert!(matches!(
            submission,
            Submission::ResolvePermission { turn_id, request_id, decision }
                if turn_id == "turn-456"
                    && request_id == "req-789"
                    && matches!(decision, PermissionDecision::Allow)
        ));
    }

    #[test]
    fn submission_resolve_permission_deny() {
        let submission = Submission::ResolvePermission {
            turn_id: "turn-456".into(),
            request_id: "req-789".into(),
            decision: PermissionDecision::Deny,
        };
        assert!(matches!(
            submission,
            Submission::ResolvePermission { decision: PermissionDecision::Deny, .. }
        ));
    }

    #[test]
    fn turn_started_stores_turn_id() {
        let result = TurnStarted {
            turn_id: "turn-abc".into(),
        };
        assert_eq!(result.turn_id, "turn-abc");
    }

    #[test]
    fn thread_created_stores_thread_id() {
        let thread_id = Uuid::new_v4();
        let result = ThreadCreated { thread_id };
        assert_eq!(result.thread_id, thread_id);
    }

    #[test]
    fn thread_switched_stores_thread_id() {
        let thread_id = Uuid::new_v4();
        let result = ThreadSwitched { thread_id };
        assert_eq!(result.thread_id, thread_id);
    }
}
