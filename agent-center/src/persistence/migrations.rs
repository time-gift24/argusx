use anyhow::Result;
use rusqlite::Connection;

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS threads (
            id TEXT PRIMARY KEY,
            parent_thread_id TEXT,
            status TEXT NOT NULL,
            agent_name TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS spawn_dedup (
            parent_thread_id TEXT NOT NULL,
            key TEXT NOT NULL,
            thread_id TEXT NOT NULL,
            PRIMARY KEY (parent_thread_id, key)
        );

        CREATE INDEX IF NOT EXISTS idx_threads_parent ON threads(parent_thread_id);
        "#,
    )?;
    Ok(())
}
