use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::SessionError;
use crate::types::{SessionRecord, ThreadRecord, TurnRecord};

#[async_trait]
pub trait SessionDatabase: Send + Sync {
    async fn get_session(&self, id: &str) -> Result<Option<SessionRecord>, SessionError>;
    async fn upsert_session(&self, session: &SessionRecord) -> Result<(), SessionError>;
    async fn get_thread(&self, thread_id: Uuid) -> Result<Option<ThreadRecord>, SessionError>;
    async fn list_threads(&self, session_id: &str) -> Result<Vec<ThreadRecord>, SessionError>;
    async fn insert_thread(&self, thread: &ThreadRecord) -> Result<(), SessionError>;
    async fn update_thread(&self, thread: &ThreadRecord) -> Result<(), SessionError>;
    async fn list_turns(&self, thread_id: Uuid) -> Result<Vec<TurnRecord>, SessionError>;
    async fn update_turn(&self, turn: &TurnRecord) -> Result<(), SessionError>;
    async fn insert_turn_and_advance_thread(
        &self,
        turn: &TurnRecord,
        previous_last_turn_number: u32,
        updated_at: DateTime<Utc>,
    ) -> Result<(), SessionError>;
    async fn mark_incomplete_turns_interrupted(&self) -> Result<u64, SessionError>;
}

pub type DynSessionDatabase = Arc<dyn SessionDatabase>;
