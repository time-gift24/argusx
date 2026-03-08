use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionRecord {
    pub id: String,
    pub user_id: Option<String>,
    pub default_model: String,
    pub system_prompt: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThreadLifecycle {
    Open,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThreadRecord {
    pub id: Uuid,
    pub session_id: String,
    pub title: Option<String>,
    pub lifecycle: ThreadLifecycle,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_turn_number: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TurnStatus {
    Running,
    WaitingPermission,
    Completed,
    Cancelled,
    Failed,
    Interrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PersistedToolKind {
    Function,
    Builtin,
    McpCall,
    McpListTools,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistedToolCall {
    pub sequence: u32,
    pub call_id: String,
    pub tool_name: String,
    pub arguments: String,
    pub kind: PersistedToolKind,
    pub server_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PersistedMessage {
    User {
        content: String,
    },
    AssistantText {
        content: String,
    },
    AssistantToolCalls {
        content: Option<String>,
        calls: Vec<PersistedToolCall>,
    },
    ToolResult {
        call_id: String,
        tool_name: String,
        content: String,
        is_error: bool,
    },
    SystemNote {
        content: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TurnRecord {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub turn_number: u32,
    pub user_input: String,
    pub status: TurnStatus,
    pub finish_reason: Option<String>,
    pub transcript: Vec<PersistedMessage>,
    pub final_output: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThreadViewState {
    Idle,
    Active,
    RunningForeground,
    RunningBackground,
    WaitingPermission,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThreadEvent {
    ThreadCreated,
    ThreadActivated,
    ThreadUpdated,
    ThreadArchived,
    TurnEventForwarded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThreadEventEnvelope {
    pub thread_id: Uuid,
    pub turn_id: Option<Uuid>,
    pub event: ThreadEvent,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_record_round_trips_with_transcript() {
        let record = TurnRecord {
            id: Uuid::new_v4(),
            thread_id: Uuid::new_v4(),
            turn_number: 2,
            user_input: "continue".into(),
            status: TurnStatus::Completed,
            finish_reason: Some("Completed".into()),
            transcript: vec![
                PersistedMessage::User {
                    content: "hello".into(),
                },
                PersistedMessage::AssistantText {
                    content: "hi".into(),
                },
            ],
            final_output: Some("hi".into()),
            started_at: Utc::now(),
            finished_at: Some(Utc::now()),
        };

        let json = serde_json::to_string(&record).unwrap();
        let decoded: TurnRecord = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.turn_number, 2);
        assert_eq!(decoded.transcript.len(), 2);
        assert_eq!(decoded.final_output.as_deref(), Some("hi"));
    }

    #[test]
    fn thread_event_envelope_round_trips() {
        let envelope = ThreadEventEnvelope {
            thread_id: Uuid::new_v4(),
            turn_id: Some(Uuid::new_v4()),
            event: ThreadEvent::TurnEventForwarded,
        };

        let json = serde_json::to_string(&envelope).unwrap();
        let decoded: ThreadEventEnvelope = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.thread_id, envelope.thread_id);
        assert_eq!(decoded.turn_id, envelope.turn_id);
        assert_eq!(decoded.event, ThreadEvent::TurnEventForwarded);
    }
}
