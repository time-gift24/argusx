use std::sync::Arc;

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::database::{DynSessionDatabase, SessionDatabase};
use crate::error::SessionError;
use crate::types::{SessionRecord, ThreadLifecycle, ThreadRecord, TurnRecord, TurnStatus};

const SESSION_SCHEMA: &str = include_str!("../../sql/session_schema.sql");

#[derive(Debug, Clone)]
pub struct ThreadStore {
    pool: SqlitePool,
}

impl ThreadStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn init_schema(&self) -> Result<()> {
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&self.pool)
            .await
            .context("enable sqlite foreign keys")?;

        for statement in SESSION_SCHEMA.split(';') {
            let statement = statement.trim();
            if statement.is_empty() {
                continue;
            }

            sqlx::query(statement)
                .execute(&self.pool)
                .await
                .with_context(|| format!("execute schema statement: {statement}"))?;
        }

        Ok(())
    }

    pub async fn get_session(&self, id: &str) -> Result<Option<SessionRecord>> {
        let result = sqlx::query(
            r#"
            SELECT id, user_id, default_model, system_prompt, created_at, updated_at
            FROM sessions WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("get session")?;

        Ok(result.map(decode_session_row))
    }

    pub async fn upsert_session(&self, session: &SessionRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO sessions (id, user_id, default_model, system_prompt, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                user_id = excluded.user_id,
                default_model = excluded.default_model,
                system_prompt = excluded.system_prompt,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&session.id)
        .bind(&session.user_id)
        .bind(&session.default_model)
        .bind(&session.system_prompt)
        .bind(session.created_at.to_rfc3339())
        .bind(session.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .context("upsert session")?;

        Ok(())
    }

    pub async fn insert_thread(&self, thread: &ThreadRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO threads (id, session_id, title, lifecycle, created_at, updated_at, last_turn_number)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(thread.id.to_string())
        .bind(&thread.session_id)
        .bind(&thread.title)
        .bind(thread_lifecycle_to_str(&thread.lifecycle))
        .bind(thread.created_at.to_rfc3339())
        .bind(thread.updated_at.to_rfc3339())
        .bind(i64::from(thread.last_turn_number))
        .execute(&self.pool)
        .await
        .context("insert thread")?;

        Ok(())
    }

    pub async fn update_thread(&self, thread: &ThreadRecord) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE threads
            SET title = ?, lifecycle = ?, updated_at = ?, last_turn_number = ?
            WHERE id = ?
            "#,
        )
        .bind(&thread.title)
        .bind(thread_lifecycle_to_str(&thread.lifecycle))
        .bind(thread.updated_at.to_rfc3339())
        .bind(i64::from(thread.last_turn_number))
        .bind(thread.id.to_string())
        .execute(&self.pool)
        .await
        .context("update thread")?;

        Ok(())
    }

    pub async fn get_thread(&self, thread_id: Uuid) -> Result<Option<ThreadRecord>> {
        let row = sqlx::query(
            r#"
            SELECT id, session_id, title, lifecycle, created_at, updated_at, last_turn_number
            FROM threads
            WHERE id = ?
            "#,
        )
        .bind(thread_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("fetch thread")?;

        row.map(decode_thread_row).transpose()
    }

    pub async fn list_threads(&self, session_id: &str) -> Result<Vec<ThreadRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT id, session_id, title, lifecycle, created_at, updated_at, last_turn_number
            FROM threads
            WHERE session_id = ?
            ORDER BY updated_at DESC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .context("list threads")?;

        rows.into_iter().map(decode_thread_row).collect()
    }

    pub async fn insert_turn(&self, turn: &TurnRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO turns (
                id, thread_id, turn_number, user_input, status, finish_reason,
                transcript_json, final_output, started_at, finished_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(turn.id.to_string())
        .bind(turn.thread_id.to_string())
        .bind(i64::from(turn.turn_number))
        .bind(&turn.user_input)
        .bind(turn_status_to_str(&turn.status))
        .bind(&turn.finish_reason)
        .bind(serde_json::to_string(&turn.transcript).context("serialize turn transcript")?)
        .bind(&turn.final_output)
        .bind(turn.started_at.to_rfc3339())
        .bind(turn.finished_at.map(|value| value.to_rfc3339()))
        .execute(&self.pool)
        .await
        .context("insert turn")?;

        Ok(())
    }

    pub async fn insert_turn_and_advance_thread(
        &self,
        turn: &TurnRecord,
        previous_last_turn_number: u32,
        updated_at: DateTime<Utc>,
    ) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("begin turn startup transaction")?;

        sqlx::query(
            r#"
            INSERT INTO turns (
                id, thread_id, turn_number, user_input, status, finish_reason,
                transcript_json, final_output, started_at, finished_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(turn.id.to_string())
        .bind(turn.thread_id.to_string())
        .bind(i64::from(turn.turn_number))
        .bind(&turn.user_input)
        .bind(turn_status_to_str(&turn.status))
        .bind(&turn.finish_reason)
        .bind(serde_json::to_string(&turn.transcript).context("serialize turn transcript")?)
        .bind(&turn.final_output)
        .bind(turn.started_at.to_rfc3339())
        .bind(turn.finished_at.map(|value| value.to_rfc3339()))
        .execute(&mut *tx)
        .await
        .context("insert turn in startup transaction")?;

        let result = sqlx::query(
            r#"
            UPDATE threads
            SET updated_at = ?, last_turn_number = ?
            WHERE id = ? AND last_turn_number = ?
            "#,
        )
        .bind(updated_at.to_rfc3339())
        .bind(i64::from(turn.turn_number))
        .bind(turn.thread_id.to_string())
        .bind(i64::from(previous_last_turn_number))
        .execute(&mut *tx)
        .await
        .context("advance thread turn number")?;

        if result.rows_affected() != 1 {
            bail!(
                "thread {} turn sequence changed during startup",
                turn.thread_id
            );
        }

        tx.commit()
            .await
            .context("commit turn startup transaction")?;
        Ok(())
    }

    pub async fn update_turn(&self, turn: &TurnRecord) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE turns
            SET status = ?, finish_reason = ?, transcript_json = ?, final_output = ?, finished_at = ?
            WHERE id = ?
            "#,
        )
        .bind(turn_status_to_str(&turn.status))
        .bind(&turn.finish_reason)
        .bind(serde_json::to_string(&turn.transcript).context("serialize turn transcript")?)
        .bind(&turn.final_output)
        .bind(turn.finished_at.map(|value| value.to_rfc3339()))
        .bind(turn.id.to_string())
        .execute(&self.pool)
        .await
        .context("update turn")?;

        Ok(())
    }

    pub async fn list_turns(&self, thread_id: Uuid) -> Result<Vec<TurnRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT id, thread_id, turn_number, user_input, status, finish_reason,
                   transcript_json, final_output, started_at, finished_at
            FROM turns
            WHERE thread_id = ?
            ORDER BY turn_number ASC
            "#,
        )
        .bind(thread_id.to_string())
        .fetch_all(&self.pool)
        .await
        .context("list turns")?;

        rows.into_iter().map(decode_turn_row).collect()
    }

    pub async fn mark_incomplete_turns_interrupted(&self) -> Result<u64> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            r#"
            UPDATE turns
            SET status = 'Interrupted',
                finish_reason = COALESCE(finish_reason, 'Interrupted'),
                finished_at = COALESCE(finished_at, ?)
            WHERE status IN ('Running', 'WaitingPermission')
            "#,
        )
        .bind(now)
        .execute(&self.pool)
        .await
        .context("mark incomplete turns interrupted")?;

        Ok(result.rows_affected())
    }
}

fn decode_session_row(row: sqlx::sqlite::SqliteRow) -> SessionRecord {
    SessionRecord {
        id: row.try_get("id").unwrap(),
        user_id: row.try_get("user_id").unwrap(),
        default_model: row.try_get("default_model").unwrap(),
        system_prompt: row.try_get("system_prompt").unwrap(),
        created_at: parse_utc(&row.try_get::<String, _>("created_at").unwrap()).unwrap(),
        updated_at: parse_utc(&row.try_get::<String, _>("updated_at").unwrap()).unwrap(),
    }
}

fn decode_thread_row(row: sqlx::sqlite::SqliteRow) -> Result<ThreadRecord> {
    Ok(ThreadRecord {
        id: parse_uuid(&row.try_get::<String, _>("id")?)?,
        session_id: row.try_get("session_id")?,
        title: row.try_get("title")?,
        lifecycle: parse_thread_lifecycle(&row.try_get::<String, _>("lifecycle")?)?,
        created_at: parse_utc(&row.try_get::<String, _>("created_at")?)?,
        updated_at: parse_utc(&row.try_get::<String, _>("updated_at")?)?,
        last_turn_number: parse_u32(
            row.try_get::<i64, _>("last_turn_number")?,
            "last_turn_number",
        )?,
    })
}

fn decode_turn_row(row: sqlx::sqlite::SqliteRow) -> Result<TurnRecord> {
    Ok(TurnRecord {
        id: parse_uuid(&row.try_get::<String, _>("id")?)?,
        thread_id: parse_uuid(&row.try_get::<String, _>("thread_id")?)?,
        turn_number: parse_u32(row.try_get::<i64, _>("turn_number")?, "turn_number")?,
        user_input: row.try_get("user_input")?,
        status: parse_turn_status(&row.try_get::<String, _>("status")?)?,
        finish_reason: row.try_get("finish_reason")?,
        transcript: serde_json::from_str(&row.try_get::<String, _>("transcript_json")?)
            .context("deserialize turn transcript")?,
        final_output: row.try_get("final_output")?,
        started_at: parse_utc(&row.try_get::<String, _>("started_at")?)?,
        finished_at: row
            .try_get::<Option<String>, _>("finished_at")?
            .map(|value| parse_utc(&value))
            .transpose()?,
    })
}

fn thread_lifecycle_to_str(lifecycle: &ThreadLifecycle) -> &'static str {
    match lifecycle {
        ThreadLifecycle::Open => "Open",
        ThreadLifecycle::Archived => "Archived",
    }
}

fn parse_thread_lifecycle(value: &str) -> Result<ThreadLifecycle> {
    match value {
        "Open" => Ok(ThreadLifecycle::Open),
        "Archived" => Ok(ThreadLifecycle::Archived),
        other => anyhow::bail!("unknown thread lifecycle: {other}"),
    }
}

fn turn_status_to_str(status: &TurnStatus) -> &'static str {
    match status {
        TurnStatus::Running => "Running",
        TurnStatus::WaitingPermission => "WaitingPermission",
        TurnStatus::Completed => "Completed",
        TurnStatus::Cancelled => "Cancelled",
        TurnStatus::Failed => "Failed",
        TurnStatus::Interrupted => "Interrupted",
    }
}

fn parse_turn_status(value: &str) -> Result<TurnStatus> {
    match value {
        "Running" => Ok(TurnStatus::Running),
        "WaitingPermission" => Ok(TurnStatus::WaitingPermission),
        "Completed" => Ok(TurnStatus::Completed),
        "Cancelled" => Ok(TurnStatus::Cancelled),
        "Failed" => Ok(TurnStatus::Failed),
        "Interrupted" => Ok(TurnStatus::Interrupted),
        other => anyhow::bail!("unknown turn status: {other}"),
    }
}

fn parse_uuid(value: &str) -> Result<Uuid> {
    Uuid::parse_str(value).with_context(|| format!("parse uuid: {value}"))
}

fn parse_utc(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .with_context(|| format!("parse utc timestamp: {value}"))
}

fn parse_u32(value: i64, field_name: &str) -> Result<u32> {
    u32::try_from(value).with_context(|| format!("convert {field_name}={value} to u32"))
}

#[async_trait]
impl SessionDatabase for ThreadStore {
    async fn get_session(&self, id: &str) -> Result<Option<SessionRecord>, SessionError> {
        Ok(ThreadStore::get_session(self, id).await?)
    }

    async fn upsert_session(&self, session: &SessionRecord) -> Result<(), SessionError> {
        Ok(ThreadStore::upsert_session(self, session).await?)
    }

    async fn get_thread(&self, thread_id: Uuid) -> Result<Option<ThreadRecord>, SessionError> {
        Ok(ThreadStore::get_thread(self, thread_id).await?)
    }

    async fn list_threads(&self, session_id: &str) -> Result<Vec<ThreadRecord>, SessionError> {
        Ok(ThreadStore::list_threads(self, session_id).await?)
    }

    async fn insert_thread(&self, thread: &ThreadRecord) -> Result<(), SessionError> {
        Ok(ThreadStore::insert_thread(self, thread).await?)
    }

    async fn update_thread(&self, thread: &ThreadRecord) -> Result<(), SessionError> {
        Ok(ThreadStore::update_thread(self, thread).await?)
    }

    async fn list_turns(&self, thread_id: Uuid) -> Result<Vec<TurnRecord>, SessionError> {
        Ok(ThreadStore::list_turns(self, thread_id).await?)
    }

    async fn update_turn(&self, turn: &TurnRecord) -> Result<(), SessionError> {
        Ok(ThreadStore::update_turn(self, turn).await?)
    }

    async fn insert_turn_and_advance_thread(
        &self,
        turn: &TurnRecord,
        previous_last_turn_number: u32,
        updated_at: DateTime<Utc>,
    ) -> Result<(), SessionError> {
        Ok(ThreadStore::insert_turn_and_advance_thread(
            self,
            turn,
            previous_last_turn_number,
            updated_at,
        )
        .await?)
    }

    async fn mark_incomplete_turns_interrupted(&self) -> Result<u64, SessionError> {
        Ok(ThreadStore::mark_incomplete_turns_interrupted(self).await?)
    }
}

impl ThreadStore {
    pub fn as_database(self: Arc<Self>) -> DynSessionDatabase {
        self
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sqlx::sqlite::SqlitePoolOptions;
    use uuid::Uuid;

    use super::*;
    use crate::types::{PersistedMessage, SessionRecord};

    async fn test_store() -> ThreadStore {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let store = ThreadStore::new(pool);
        store.init_schema().await.unwrap();
        store
    }

    #[tokio::test]
    async fn store_round_trips_thread_and_turn_history() {
        let store = test_store().await;

        let session = SessionRecord {
            id: "session-1".into(),
            user_id: None,
            default_model: "gpt-5".into(),
            system_prompt: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        store.upsert_session(&session).await.unwrap();

        let thread = ThreadRecord {
            id: Uuid::new_v4(),
            session_id: session.id.clone(),
            title: Some("Test".into()),
            lifecycle: ThreadLifecycle::Open,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_turn_number: 1,
        };
        store.insert_thread(&thread).await.unwrap();

        let turn = TurnRecord {
            id: Uuid::new_v4(),
            thread_id: thread.id,
            turn_number: 1,
            user_input: "hello".into(),
            status: TurnStatus::Completed,
            finish_reason: Some("Completed".into()),
            transcript: vec![PersistedMessage::User {
                content: "hello".into(),
            }],
            final_output: Some("hi".into()),
            started_at: Utc::now(),
            finished_at: Some(Utc::now()),
        };
        store.insert_turn(&turn).await.unwrap();

        let loaded_thread = store.get_thread(thread.id).await.unwrap().unwrap();
        assert_eq!(loaded_thread.title.as_deref(), Some("Test"));
        assert_eq!(loaded_thread.last_turn_number, 1);

        let threads = store.list_threads(&session.id).await.unwrap();
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].id, thread.id);

        let history = store.list_turns(thread.id).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].final_output.as_deref(), Some("hi"));
        assert_eq!(history[0].transcript.len(), 1);
    }

    #[tokio::test]
    async fn store_marks_incomplete_turns_interrupted() {
        let store = test_store().await;

        let session = SessionRecord {
            id: "session-1".into(),
            user_id: None,
            default_model: "gpt-5".into(),
            system_prompt: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        store.upsert_session(&session).await.unwrap();

        let thread = ThreadRecord {
            id: Uuid::new_v4(),
            session_id: session.id.clone(),
            title: None,
            lifecycle: ThreadLifecycle::Open,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_turn_number: 2,
        };
        store.insert_thread(&thread).await.unwrap();

        for (turn_number, status) in [(1, TurnStatus::Running), (2, TurnStatus::WaitingPermission)]
        {
            let turn = TurnRecord {
                id: Uuid::new_v4(),
                thread_id: thread.id,
                turn_number,
                user_input: format!("turn-{turn_number}"),
                status,
                finish_reason: None,
                transcript: vec![],
                final_output: None,
                started_at: Utc::now(),
                finished_at: None,
            };
            store.insert_turn(&turn).await.unwrap();
        }

        let affected = store.mark_incomplete_turns_interrupted().await.unwrap();
        assert_eq!(affected, 2);

        let turns = store.list_turns(thread.id).await.unwrap();
        assert!(
            turns
                .iter()
                .all(|turn| turn.status == TurnStatus::Interrupted)
        );
        assert!(turns.iter().all(|turn| turn.finished_at.is_some()));
    }
}
