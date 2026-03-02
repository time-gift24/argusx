use std::path::Path;

use rusqlite::Connection;
use thiserror::Error;

const SCHEMA_VERSION: i64 = 1;

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

    conn.execute_batch(
        r#"
BEGIN;

CREATE TABLE IF NOT EXISTS sessions (
  session_id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL,
  archived_at_ms INTEGER
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
  item_type TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  UNIQUE(turn_id, seq),
  FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE,
  FOREIGN KEY (turn_id) REFERENCES turns(turn_id) ON DELETE CASCADE
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

PRAGMA user_version = 1;
COMMIT;
"#,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn table_exists(conn: &Connection, table_name: &str) -> bool {
        let sql = "SELECT COUNT(1) FROM sqlite_master WHERE type = 'table' AND name = ?1";
        let count = conn
            .query_row(sql, [table_name], |row| row.get::<_, i64>(0))
            .expect("query sqlite master");
        count > 0
    }

    #[test]
    fn sqlite_schema_bootstraps() {
        let temp = tempdir().expect("create tempdir");
        let db_path = temp.path().join("desktop.db");

        let conn = open_and_bootstrap(&db_path).expect("bootstrap schema");

        assert!(table_exists(&conn, "sessions"));
        assert!(table_exists(&conn, "turns"));
        assert!(table_exists(&conn, "transcript_items"));
        assert!(table_exists(&conn, "llm_runtime_config"));
        assert!(table_exists(&conn, "llm_provider_configs"));
        assert!(db_path.exists());
    }
}
