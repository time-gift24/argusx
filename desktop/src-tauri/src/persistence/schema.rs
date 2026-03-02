use std::path::Path;

use rusqlite::Connection;
use thiserror::Error;

const SCHEMA_VERSION: i64 = 2;

#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

pub fn open_and_bootstrap(db_path: &Path) -> Result<Connection, SchemaError> {
    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<(), SchemaError> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if current >= SCHEMA_VERSION {
        return Ok(());
    }

    // v1 created transcript_items with a mandatory item_type column and sessions without
    // user_id/parent_id. Normalize to the unified schema used by agent-session sqlite store.
    if table_exists(conn, "transcript_items")?
        && column_exists(conn, "transcript_items", "item_type")?
    {
        conn.execute_batch(
            r#"
BEGIN;
CREATE TABLE IF NOT EXISTS transcript_items_v2 (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL,
  turn_id TEXT NOT NULL,
  seq INTEGER NOT NULL,
  payload_json TEXT NOT NULL,
  UNIQUE(turn_id, seq),
  FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
);
INSERT INTO transcript_items_v2 (session_id, turn_id, seq, payload_json)
SELECT session_id, turn_id, seq, payload_json FROM transcript_items;
DROP TABLE transcript_items;
ALTER TABLE transcript_items_v2 RENAME TO transcript_items;
COMMIT;
"#,
        )?;
    }

    if table_exists(conn, "sessions")? {
        if !column_exists(conn, "sessions", "user_id")? {
            conn.execute("ALTER TABLE sessions ADD COLUMN user_id TEXT", [])?;
        }
        if !column_exists(conn, "sessions", "parent_id")? {
            conn.execute("ALTER TABLE sessions ADD COLUMN parent_id TEXT", [])?;
        }
    }

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

CREATE TABLE IF NOT EXISTS llm_runtime_config (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  default_provider TEXT,
  updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS llm_provider_configs (
  provider_id TEXT PRIMARY KEY,
  base_url TEXT NOT NULL,
  models_json TEXT NOT NULL,
  headers_json TEXT NOT NULL,
  api_key_cipher_json TEXT NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_turns_session_ended
ON turns (session_id, ended_at_ms DESC);

CREATE INDEX IF NOT EXISTS idx_turns_session_started
ON turns (session_id, started_at_ms DESC);

CREATE INDEX IF NOT EXISTS idx_transcript_turn_seq
ON transcript_items (turn_id, seq);

CREATE INDEX IF NOT EXISTS idx_transcript_session_turn
ON transcript_items (session_id, turn_id);

PRAGMA user_version = 2;
COMMIT;
"#,
    )?;

    Ok(())
}

fn table_exists(conn: &Connection, table_name: &str) -> Result<bool, SchemaError> {
    let sql = "SELECT COUNT(1) FROM sqlite_master WHERE type = 'table' AND name = ?1";
    let count = conn.query_row(sql, [table_name], |row| row.get::<_, i64>(0))?;
    Ok(count > 0)
}

fn column_exists(
    conn: &Connection,
    table_name: &str,
    column_name: &str,
) -> Result<bool, SchemaError> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table_name})"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column_name {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn sqlite_schema_bootstraps() {
        let temp = tempdir().expect("create tempdir");
        let db_path = temp.path().join("desktop.db");

        let conn = open_and_bootstrap(&db_path).expect("bootstrap schema");

        assert!(table_exists(&conn, "sessions").expect("sessions table exists"));
        assert!(table_exists(&conn, "turn_contexts").expect("turn_contexts table exists"));
        assert!(table_exists(&conn, "turns").expect("turns table exists"));
        assert!(table_exists(&conn, "transcript_items").expect("transcript_items table exists"));
        assert!(table_exists(&conn, "llm_runtime_config").expect("runtime table exists"));
        assert!(table_exists(&conn, "llm_provider_configs").expect("provider table exists"));
        assert!(column_exists(&conn, "sessions", "user_id").expect("user_id column exists"));
        assert!(!column_exists(&conn, "transcript_items", "item_type").expect("item_type removed"));
        assert!(db_path.exists());
    }
}
