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
pub struct PersistedToolCall {
    pub call_id: String,
    pub tool_name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PersistedMessage {
    User { content: String },
    AssistantText { content: String },
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
    SystemNote { content: String },
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
    ThreadCreated { thread_id: Uuid },
    ThreadActivated { thread_id: Uuid },
    ThreadUpdated { thread_id: Uuid },
    ThreadArchived { thread_id: Uuid },
    TurnEventForwarded { thread_id: Uuid, turn_id: Uuid },
}
