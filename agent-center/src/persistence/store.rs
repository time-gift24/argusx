use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

use super::migrations;
use super::models::ThreadRow;

pub trait ThreadStore {
    fn upsert_thread(&self, thread: &ThreadRow) -> Result<()>;
    fn get_by_dedup(&self, parent_thread_id: &str, key: &str) -> Result<Option<String>>;
    fn insert_dedup(&self, parent_thread_id: &str, key: &str, thread_id: &str) -> Result<()>;
}

pub struct SqliteThreadStore {
    conn: Mutex<Connection>,
}

impl SqliteThreadStore {
    pub fn new(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        migrations::run_migrations(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }
}

impl ThreadStore for SqliteThreadStore {
    fn upsert_thread(&self, thread: &ThreadRow) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO threads (id, parent_thread_id, status, agent_name, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                agent_name = excluded.agent_name
            "#,
            rusqlite::params![
                thread.id,
                thread.parent_thread_id,
                thread.status,
                thread.agent_name,
                thread.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    fn get_by_dedup(&self, parent_thread_id: &str, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT thread_id FROM spawn_dedup WHERE parent_thread_id = ?1 AND key = ?2",
            rusqlite::params![parent_thread_id, key],
            |row| row.get(0),
        );

        match result {
            Ok(thread_id) => Ok(Some(thread_id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn insert_dedup(&self, parent_thread_id: &str, key: &str, thread_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO spawn_dedup (parent_thread_id, key, thread_id) VALUES (?1, ?2, ?3)",
            rusqlite::params![parent_thread_id, key, thread_id],
        )?;
        Ok(())
    }
}
