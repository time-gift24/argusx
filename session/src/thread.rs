use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::types::{ThreadState, TurnRecord};

/// Thread runtime structure - holds state for a single conversation
pub struct Thread {
    pub id: Uuid,
    pub session_id: String,
    pub title: Option<String>,
    pub state: ThreadState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub history: Vec<TurnRecord>,
}

impl Thread {
    /// Create a new thread with optional title
    pub fn new(session_id: String, title: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            title,
            state: ThreadState::Idle,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            history: Vec::new(),
        }
    }

    /// Update thread state
    pub fn set_state(&mut self, state: ThreadState) {
        self.updated_at = Utc::now();
        self.state = state;
    }

    /// Add a turn to history
    pub fn add_turn(&mut self, turn: TurnRecord) {
        self.history.push(turn);
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ThreadState, TurnRecord, TurnRecordState};
    use uuid::Uuid;

    #[test]
    fn thread_creation() {
        let thread = Thread::new("session-1".to_string(), None);
        assert!(matches!(thread.state, ThreadState::Idle));
        assert!(thread.id != Uuid::nil());
        assert_eq!(thread.session_id, "session-1");
    }

    #[test]
    fn thread_with_title() {
        let thread = Thread::new("session-1".to_string(), Some("My Thread".to_string()));
        assert_eq!(thread.title, Some("My Thread".to_string()));
    }

    #[test]
    fn thread_state_transitions() {
        let mut thread = Thread::new("session-1".to_string(), None);
        assert!(matches!(thread.state, ThreadState::Idle));

        thread.set_state(ThreadState::Processing);
        assert_eq!(thread.state, ThreadState::Processing);

        thread.set_state(ThreadState::Idle);
        assert_eq!(thread.state, ThreadState::Idle);
    }

    #[test]
    fn thread_add_turn() {
        let mut thread = Thread::new("session-1".to_string(), None);
        let turn = TurnRecord {
            turn_number: 1,
            user_input: "Hello".to_string(),
            assistant_response: None,
            tool_calls: vec![],
            started_at: Utc::now(),
            completed_at: None,
            state: TurnRecordState::Completed,
        };
        thread.add_turn(turn);
        assert_eq!(thread.history.len(), 1);
        assert_eq!(thread.history[0].user_input, "Hello");
    }
}
