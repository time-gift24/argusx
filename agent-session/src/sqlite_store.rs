use std::path::PathBuf;

use agent_core::{SessionInfo, SessionStatus, TranscriptItem, TurnContext, TurnSummary};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rusqlite::{params, Connection, OptionalExtension};

use crate::storage::{SessionArtifactStore, SessionFilter, SessionStore};

pub struct SqliteSessionStore {
    db_path: PathBuf,
}

impl SqliteSessionStore {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        let this = Self { db_path };
        let conn = this.open_connection()?;
        this.bootstrap_schema(&conn)?;
        Ok(this)
    }

    fn open_connection(&self) -> Result<Connection> {
        let conn = Connection::open(&self.db_path)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(conn)
    }

    fn bootstrap_schema(&self, conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
BEGIN;

CREATE TABLE IF NOT EXISTS sessions (
  session_id TEXT PRIMARY KEY,
  user_id TEXT,
  parent_id TEXT,
  title TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL,
  archived_at_ms INTEGER
);

CREATE TABLE IF NOT EXISTS turn_contexts (
  turn_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  epoch INTEGER NOT NULL,
  started_at_ms INTEGER NOT NULL,
  FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS turns (
  turn_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  epoch INTEGER NOT NULL,
  started_at_ms INTEGER NOT NULL,
  ended_at_ms INTEGER,
  status TEXT NOT NULL,
  final_message TEXT,
  tool_calls_count INTEGER NOT NULL DEFAULT 0,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS transcript_items (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL,
  turn_id TEXT NOT NULL,
  seq INTEGER NOT NULL,
  payload_json TEXT NOT NULL,
  UNIQUE(turn_id, seq),
  FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_turns_session_ended
  ON turns (session_id, ended_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_turns_session_started
  ON turns (session_id, started_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_transcript_turn_seq
  ON transcript_items (turn_id, seq);
CREATE INDEX IF NOT EXISTS idx_transcript_session_turn
  ON transcript_items (session_id, turn_id);

COMMIT;
"#,
        )?;
        Ok(())
    }

    pub async fn save_turn_context(&self, context: &TurnContext) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute(
            r#"
INSERT INTO turn_contexts (turn_id, session_id, epoch, started_at_ms)
VALUES (?1, ?2, ?3, ?4)
ON CONFLICT(turn_id) DO UPDATE SET
  session_id = excluded.session_id,
  epoch = excluded.epoch,
  started_at_ms = excluded.started_at_ms
"#,
            params![
                context.turn_id,
                context.session_id,
                context.epoch as i64,
                context.started_at
            ],
        )?;
        Ok(())
    }

    pub async fn save_turn_summary(&self, session_id: &str, summary: &TurnSummary) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute(
            r#"
INSERT INTO turns (
  turn_id, session_id, epoch, started_at_ms, ended_at_ms, status, final_message,
  tool_calls_count, input_tokens, output_tokens
)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
ON CONFLICT(turn_id) DO UPDATE SET
  session_id = excluded.session_id,
  epoch = excluded.epoch,
  started_at_ms = excluded.started_at_ms,
  ended_at_ms = excluded.ended_at_ms,
  status = excluded.status,
  final_message = excluded.final_message,
  tool_calls_count = excluded.tool_calls_count,
  input_tokens = excluded.input_tokens,
  output_tokens = excluded.output_tokens
"#,
            params![
                summary.turn_id,
                session_id,
                summary.epoch as i64,
                summary.started_at,
                summary.ended_at,
                turn_status_to_str(summary.status),
                summary.final_message,
                summary.tool_calls_count as i64,
                summary.input_tokens as i64,
                summary.output_tokens as i64
            ],
        )?;
        Ok(())
    }

    pub async fn list_turn_summaries(&self, session_id: &str) -> Result<Vec<TurnSummary>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            r#"
SELECT turn_id, epoch, started_at_ms, ended_at_ms, status, final_message,
       tool_calls_count, input_tokens, output_tokens
FROM turns
WHERE session_id = ?1
ORDER BY started_at_ms ASC
"#,
        )?;
        let rows = stmt.query_map([session_id], row_to_turn_summary)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub async fn load_latest_turn_summary(&self, session_id: &str) -> Result<Option<TurnSummary>> {
        let conn = self.open_connection()?;
        conn.query_row(
            r#"
SELECT turn_id, epoch, started_at_ms, ended_at_ms, status, final_message,
       tool_calls_count, input_tokens, output_tokens
FROM turns
WHERE session_id = ?1
ORDER BY started_at_ms DESC
LIMIT 1
"#,
            [session_id],
            row_to_turn_summary,
        )
        .optional()
        .map_err(Into::into)
    }

    pub async fn delete_turn_artifacts(&self, session_id: &str, turn_id: &str) -> Result<()> {
        let conn = self.open_connection()?;
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM transcript_items WHERE session_id = ?1 AND turn_id = ?2",
            params![session_id, turn_id],
        )?;
        tx.execute(
            "DELETE FROM turn_contexts WHERE session_id = ?1 AND turn_id = ?2",
            params![session_id, turn_id],
        )?;
        tx.execute(
            "DELETE FROM turns WHERE session_id = ?1 AND turn_id = ?2",
            params![session_id, turn_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub async fn truncate_turns_after(
        &self,
        session_id: &str,
        restored_turn_id: &str,
    ) -> Result<Vec<String>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT turn_id FROM turns WHERE session_id = ?1 ORDER BY started_at_ms ASC, turn_id ASC",
        )?;
        let rows = stmt.query_map([session_id], |row| row.get::<_, String>(0))?;
        let mut turn_ids = Vec::new();
        for row in rows {
            turn_ids.push(row?);
        }

        let Some(idx) = turn_ids.iter().position(|id| id == restored_turn_id) else {
            return Err(anyhow!(
                "restore target turn {restored_turn_id} not found in session {session_id}"
            ));
        };

        let removed: Vec<String> = turn_ids.into_iter().skip(idx + 1).collect();
        if removed.is_empty() {
            return Ok(removed);
        }

        let tx = conn.unchecked_transaction()?;
        for turn_id in &removed {
            tx.execute(
                "DELETE FROM transcript_items WHERE session_id = ?1 AND turn_id = ?2",
                params![session_id, turn_id],
            )?;
            tx.execute(
                "DELETE FROM turn_contexts WHERE session_id = ?1 AND turn_id = ?2",
                params![session_id, turn_id],
            )?;
            tx.execute(
                "DELETE FROM turns WHERE session_id = ?1 AND turn_id = ?2",
                params![session_id, turn_id],
            )?;
        }
        tx.commit()?;
        Ok(removed)
    }

    pub async fn save_turn_transcript(
        &self,
        session_id: &str,
        turn_id: &str,
        items: &[TranscriptItem],
    ) -> Result<()> {
        let conn = self.open_connection()?;
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM transcript_items WHERE session_id = ?1 AND turn_id = ?2",
            params![session_id, turn_id],
        )?;
        let mut insert_stmt = tx.prepare(
            "INSERT INTO transcript_items (session_id, turn_id, seq, payload_json) VALUES (?1, ?2, ?3, ?4)",
        )?;
        for (idx, item) in items.iter().enumerate() {
            let payload = serde_json::to_string(item)?;
            insert_stmt.execute(params![session_id, turn_id, idx as i64, payload])?;
        }
        drop(insert_stmt);
        tx.commit()?;
        Ok(())
    }

    pub async fn load_turn_transcript(
        &self,
        session_id: &str,
        turn_id: &str,
    ) -> Result<Vec<TranscriptItem>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT payload_json FROM transcript_items WHERE session_id = ?1 AND turn_id = ?2 ORDER BY seq ASC",
        )?;
        let rows = stmt.query_map(params![session_id, turn_id], |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for row in rows {
            let payload = row?;
            out.push(serde_json::from_str::<TranscriptItem>(&payload)?);
        }
        Ok(out)
    }

    pub async fn find_session_id_by_turn_id(&self, turn_id: &str) -> Result<Option<String>> {
        let conn = self.open_connection()?;
        if let Some(session_id) = conn
            .query_row(
                "SELECT session_id FROM turn_contexts WHERE turn_id = ?1 LIMIT 1",
                [turn_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            return Ok(Some(session_id));
        }
        if let Some(session_id) = conn
            .query_row(
                "SELECT session_id FROM turns WHERE turn_id = ?1 LIMIT 1",
                [turn_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            return Ok(Some(session_id));
        }
        conn.query_row(
            "SELECT session_id FROM transcript_items WHERE turn_id = ?1 LIMIT 1",
            [turn_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(Into::into)
    }
}

#[async_trait]
impl SessionStore for SqliteSessionStore {
    async fn create(&self, info: &SessionInfo) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute(
            r#"
INSERT INTO sessions (
  session_id, user_id, parent_id, title, status, created_at_ms, updated_at_ms, archived_at_ms
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
"#,
            params![
                info.session_id,
                info.user_id,
                info.parent_id,
                info.title,
                session_status_to_str(info.status),
                info.created_at,
                info.updated_at,
                info.archived_at
            ],
        )?;
        Ok(())
    }

    async fn get(&self, session_id: &str) -> Result<Option<SessionInfo>> {
        let conn = self.open_connection()?;
        conn.query_row(
            r#"
SELECT session_id, user_id, parent_id, title, status, created_at_ms, updated_at_ms, archived_at_ms
FROM sessions
WHERE session_id = ?1
"#,
            [session_id],
            row_to_session_info,
        )
        .optional()
        .map_err(Into::into)
    }

    async fn update(&self, info: &SessionInfo) -> Result<()> {
        let conn = self.open_connection()?;
        let affected = conn.execute(
            r#"
UPDATE sessions SET
  user_id = ?2,
  parent_id = ?3,
  title = ?4,
  status = ?5,
  created_at_ms = ?6,
  updated_at_ms = ?7,
  archived_at_ms = ?8
WHERE session_id = ?1
"#,
            params![
                info.session_id,
                info.user_id,
                info.parent_id,
                info.title,
                session_status_to_str(info.status),
                info.created_at,
                info.updated_at,
                info.archived_at
            ],
        )?;
        if affected == 0 {
            anyhow::bail!("Session not found: {}", info.session_id);
        }
        Ok(())
    }

    async fn delete(&self, session_id: &str) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute("DELETE FROM sessions WHERE session_id = ?1", [session_id])?;
        Ok(())
    }

    async fn list(&self, filter: SessionFilter) -> Result<Vec<SessionInfo>> {
        let conn = self.open_connection()?;

        let mut sql = String::from(
            r#"
SELECT
  s.session_id, s.user_id, s.parent_id, s.title, s.status,
  s.created_at_ms, s.updated_at_ms, s.archived_at_ms,
  MAX(t.ended_at_ms) AS last_ended_at
FROM sessions s
LEFT JOIN turns t ON t.session_id = s.session_id
WHERE 1 = 1
"#,
        );
        let mut binds: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(user_id) = filter.user_id {
            sql.push_str(" AND s.user_id = ? ");
            binds.push(Box::new(user_id));
        }
        if let Some(status) = filter.status {
            sql.push_str(" AND s.status = ? ");
            binds.push(Box::new(session_status_to_str(status).to_string()));
        }
        if let Some(from_date) = filter.from_date {
            sql.push_str(" AND s.updated_at_ms >= ? ");
            binds.push(Box::new(from_date));
        }
        if let Some(to_date) = filter.to_date {
            sql.push_str(" AND s.updated_at_ms <= ? ");
            binds.push(Box::new(to_date));
        }

        sql.push_str(
            r#"
GROUP BY
  s.session_id, s.user_id, s.parent_id, s.title, s.status,
  s.created_at_ms, s.updated_at_ms, s.archived_at_ms
ORDER BY
  CASE WHEN last_ended_at IS NULL THEN 1 ELSE 0 END ASC,
  last_ended_at DESC,
  s.updated_at_ms DESC
"#,
        );

        if let Some(limit) = filter.limit {
            sql.push_str(" LIMIT ? ");
            binds.push(Box::new(limit as i64));
        }

        let bind_refs: Vec<&dyn rusqlite::ToSql> = binds.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(
            bind_refs.as_slice(),
            row_to_session_info_with_ignored_last_ended,
        )?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

#[async_trait]
impl SessionArtifactStore for SqliteSessionStore {
    async fn save_turn_context(&self, context: &TurnContext) -> Result<()> {
        SqliteSessionStore::save_turn_context(self, context).await
    }

    async fn save_turn_summary(&self, session_id: &str, summary: &TurnSummary) -> Result<()> {
        SqliteSessionStore::save_turn_summary(self, session_id, summary).await
    }

    async fn list_turn_summaries(&self, session_id: &str) -> Result<Vec<TurnSummary>> {
        SqliteSessionStore::list_turn_summaries(self, session_id).await
    }

    async fn load_latest_turn_summary(&self, session_id: &str) -> Result<Option<TurnSummary>> {
        SqliteSessionStore::load_latest_turn_summary(self, session_id).await
    }

    async fn delete_turn_artifacts(&self, session_id: &str, turn_id: &str) -> Result<()> {
        SqliteSessionStore::delete_turn_artifacts(self, session_id, turn_id).await
    }

    async fn truncate_turns_after(
        &self,
        session_id: &str,
        restored_turn_id: &str,
    ) -> Result<Vec<String>> {
        SqliteSessionStore::truncate_turns_after(self, session_id, restored_turn_id).await
    }

    async fn save_turn_transcript(
        &self,
        session_id: &str,
        turn_id: &str,
        items: &[TranscriptItem],
    ) -> Result<()> {
        SqliteSessionStore::save_turn_transcript(self, session_id, turn_id, items).await
    }

    async fn load_turn_transcript(
        &self,
        session_id: &str,
        turn_id: &str,
    ) -> Result<Vec<TranscriptItem>> {
        SqliteSessionStore::load_turn_transcript(self, session_id, turn_id).await
    }

    async fn find_session_id_by_turn_id(&self, turn_id: &str) -> Result<Option<String>> {
        SqliteSessionStore::find_session_id_by_turn_id(self, turn_id).await
    }
}

fn session_status_to_str(status: SessionStatus) -> &'static str {
    match status {
        SessionStatus::Active => "active",
        SessionStatus::Idle => "idle",
        SessionStatus::Archived => "archived",
    }
}

fn str_to_session_status(status: &str) -> std::result::Result<SessionStatus, String> {
    match status {
        "active" => Ok(SessionStatus::Active),
        "idle" => Ok(SessionStatus::Idle),
        "archived" => Ok(SessionStatus::Archived),
        value => Err(format!("unknown session status: {value}")),
    }
}

fn turn_status_to_str(status: agent_core::TurnStatus) -> &'static str {
    match status {
        agent_core::TurnStatus::Running => "running",
        agent_core::TurnStatus::Done => "done",
        agent_core::TurnStatus::Failed => "failed",
        agent_core::TurnStatus::Cancelled => "cancelled",
    }
}

fn str_to_turn_status(status: &str) -> std::result::Result<agent_core::TurnStatus, String> {
    match status {
        "running" => Ok(agent_core::TurnStatus::Running),
        "done" => Ok(agent_core::TurnStatus::Done),
        "failed" => Ok(agent_core::TurnStatus::Failed),
        "cancelled" => Ok(agent_core::TurnStatus::Cancelled),
        value => Err(format!("unknown turn status: {value}")),
    }
}

fn row_to_session_info(row: &rusqlite::Row<'_>) -> rusqlite::Result<SessionInfo> {
    let status: String = row.get(4)?;
    let parsed_status = str_to_session_status(&status).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(
            4,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
        )
    })?;
    Ok(SessionInfo {
        session_id: row.get(0)?,
        user_id: row.get(1)?,
        parent_id: row.get(2)?,
        title: row.get(3)?,
        status: parsed_status,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
        archived_at: row.get(7)?,
    })
}

fn row_to_session_info_with_ignored_last_ended(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<SessionInfo> {
    row_to_session_info(row)
}

fn row_to_turn_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<TurnSummary> {
    let status: String = row.get(4)?;
    let parsed_status = str_to_turn_status(&status).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(
            4,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
        )
    })?;
    Ok(TurnSummary {
        turn_id: row.get(0)?,
        epoch: row.get::<_, i64>(1)? as u64,
        started_at: row.get(2)?,
        ended_at: row.get(3)?,
        status: parsed_status,
        final_message: row.get(5)?,
        tool_calls_count: row.get::<_, i64>(6)? as u32,
        input_tokens: row.get::<_, i64>(7)? as u64,
        output_tokens: row.get::<_, i64>(8)? as u64,
    })
}
