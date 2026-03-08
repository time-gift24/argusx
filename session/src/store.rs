use sqlx::SqlitePool;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde_json;

use crate::types::{ThreadState, TurnRecord, TurnRecordState, AssistantResponse};
use crate::thread::Thread;

/// Thread summary for listing
#[derive(Debug, Clone)]
pub struct ThreadSummary {
    pub id: Uuid,
    pub title: Option<String>,
    pub state: ThreadState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Persistent storage for threads using SQLite
pub struct ThreadStore {
    pool: SqlitePool,
}

impl ThreadStore {
    /// Create a new ThreadStore with the given connection pool
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Initialize the database schema
    pub async fn init_schema(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS threads (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                title TEXT,
                state TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )"#
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS turns (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                thread_id TEXT NOT NULL,
                turn_number INTEGER NOT NULL,
                user_input TEXT NOT NULL,
                assistant_response TEXT,
                state TEXT NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT
            )"#
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Insert a new thread
    pub async fn insert_thread(&self, thread: &Thread) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let state_str = state_to_string(&thread.state);
        sqlx::query(
            r#"INSERT INTO threads (id, session_id, title, state, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)"#
        )
        .bind(thread.id.to_string())
        .bind(&thread.session_id)
        .bind(&thread.title)
        .bind(&state_str)
        .bind(thread.created_at.to_rfc3339())
        .bind(thread.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update thread state
    pub async fn update_thread_state(&self, id: Uuid, state: &ThreadState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let state_str = state_to_string(state);
        sqlx::query(r#"UPDATE threads SET state = ?, updated_at = ? WHERE id = ?"#)
            .bind(&state_str)
            .bind(Utc::now().to_rfc3339())
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List all threads for a session
    pub async fn list_threads(&self, session_id: &str) -> Result<Vec<ThreadSummary>, Box<dyn std::error::Error + Send + Sync>> {
        let rows: Vec<(String, Option<String>, String, String, String)> = sqlx::query_as(
            r#"SELECT id, title, state, created_at, updated_at FROM threads WHERE session_id = ? ORDER BY updated_at DESC"#
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        let mut summaries = Vec::new();
        for row in rows {
            summaries.push(ThreadSummary {
                id: Uuid::parse_str(&row.0)?,
                title: row.1,
                state: string_to_state(&row.2),
                created_at: row.3.parse()?,
                updated_at: row.4.parse()?,
            });
        }
        Ok(summaries)
    }

    /// Delete a thread
    pub async fn delete_thread(&self, id: Uuid) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query(r#"DELETE FROM threads WHERE id = ?"#)
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Save a turn
    pub async fn save_turn(&self, thread_id: Uuid, turn: &TurnRecord) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let state_str = match turn.state {
            TurnRecordState::Completed => "Completed",
            TurnRecordState::Failed => "Failed",
            TurnRecordState::Interrupted => "Interrupted",
        };
        let response_json = turn.assistant_response.as_ref().map(|r| serde_json::to_string(r).unwrap());
        sqlx::query(
            r#"INSERT INTO turns (thread_id, turn_number, user_input, assistant_response, state, started_at, completed_at) VALUES (?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(thread_id.to_string())
        .bind(turn.turn_number as i64)
        .bind(&turn.user_input)
        .bind(&response_json)
        .bind(&state_str)
        .bind(turn.started_at.to_rfc3339())
        .bind(turn.completed_at.map(|t| t.to_rfc3339()))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Load turn history
    pub async fn load_turn_history(&self, thread_id: Uuid) -> Result<Vec<TurnRecord>, Box<dyn std::error::Error + Send + Sync>> {
        let rows: Vec<(i64, String, Option<String>, String, String, Option<String>)> = sqlx::query_as(
            r#"SELECT turn_number, user_input, assistant_response, state, started_at, completed_at FROM turns WHERE thread_id = ? ORDER BY turn_number ASC"#
        )
        .bind(thread_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut turns = Vec::new();
        for row in rows {
            let state = match row.3.as_str() {
                "Completed" => TurnRecordState::Completed,
                "Failed" => TurnRecordState::Failed,
                _ => TurnRecordState::Interrupted,
            };
            let assistant_response: Option<AssistantResponse> = row.2.and_then(|json| serde_json::from_str(&json).ok());
            turns.push(TurnRecord {
                turn_number: row.0 as usize,
                user_input: row.1,
                assistant_response,
                tool_calls: vec![],
                started_at: row.4.parse()?,
                completed_at: row.5.and_then(|t| t.parse().ok()),
                state,
            });
        }
        Ok(turns)
    }
}

fn state_to_string(state: &ThreadState) -> &'static str {
    match state {
        ThreadState::Idle => "Idle",
        ThreadState::Processing => "Processing",
        ThreadState::BackgroundProcessing => "BackgroundProcessing",
        ThreadState::WaitingForPermission => "WaitingForPermission",
        ThreadState::Completed => "Completed",
        ThreadState::Failed(_) => "Failed",
    }
}

fn string_to_state(s: &str) -> ThreadState {
    match s {
        "Idle" => ThreadState::Idle,
        "Processing" => ThreadState::Processing,
        "BackgroundProcessing" => ThreadState::BackgroundProcessing,
        "WaitingForPermission" => ThreadState::WaitingForPermission,
        "Completed" => ThreadState::Completed,
        _ => ThreadState::Failed(s.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init_schema() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let store = ThreadStore::new(pool);
        store.init_schema().await.unwrap();
    }

    #[tokio::test]
    async fn test_insert_and_list_threads() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let store = ThreadStore::new(pool);
        store.init_schema().await.unwrap();

        let thread = Thread::new("session-1".to_string(), Some("Test Thread".to_string()));
        store.insert_thread(&thread).await.unwrap();

        let threads = store.list_threads("session-1").await.unwrap();
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].title, Some("Test Thread".to_string()));
    }
}
