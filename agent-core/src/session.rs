use serde::{Deserialize, Serialize};

pub type SessionId = Id;
pub type TurnId = Id;

pub use crate::model::Id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Idle,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnStatus {
    Running,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: SessionId,
    pub user_id: Option<String>,
    pub parent_id: Option<SessionId>,
    pub title: String,
    pub status: SessionStatus,
    pub created_at: i64,
    pub updated_at: i64,
    pub archived_at: Option<i64>,
}

impl SessionInfo {
    pub fn new(session_id: SessionId, title: String) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            session_id,
            user_id: None,
            parent_id: None,
            title,
            status: SessionStatus::Idle,
            created_at: now,
            updated_at: now,
            archived_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnSummary {
    pub turn_id: TurnId,
    pub epoch: u64,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub status: TurnStatus,
    pub final_message: Option<String>,
    pub tool_calls_count: u32,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnContext {
    pub turn_id: TurnId,
    pub session_id: SessionId,
    pub epoch: u64,
    pub started_at: i64,
}

impl TurnContext {
    pub fn new(session_id: SessionId) -> Self {
        Self {
            turn_id: crate::new_id(),
            session_id,
            epoch: 0,
            started_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub fn next_epoch(&self) -> Self {
        Self {
            turn_id: crate::new_id(),
            session_id: self.session_id.clone(),
            epoch: self.epoch + 1,
            started_at: chrono::Utc::now().timestamp_millis(),
        }
    }
}
