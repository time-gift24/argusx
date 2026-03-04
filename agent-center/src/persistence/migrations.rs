use anyhow::Result;
use rusqlite::Connection;

pub fn run_migrations(conn: &Connection) -> Result<()> {
    // Create tables
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS threads (
            id TEXT PRIMARY KEY,
            parent_thread_id TEXT,
            status TEXT NOT NULL,
            agent_name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            depth INTEGER NOT NULL DEFAULT 0
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

    // Migration: Add depth column to existing tables (ignore error if column exists)
    let _ = conn.execute("ALTER TABLE threads ADD COLUMN depth INTEGER NOT NULL DEFAULT 0", ());

    Ok(())
}
