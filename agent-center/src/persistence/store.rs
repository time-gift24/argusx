use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

use super::migrations;
use super::models::ThreadRow;

pub enum ClaimResult {
    New,
    Existing(String),
}

pub trait ThreadStore {
    fn upsert_thread(&self, thread: &ThreadRow) -> Result<()>;
    fn get_thread(&self, id: &str) -> Result<Option<ThreadRow>>;
    fn get_all_threads(&self) -> Result<Vec<ThreadRow>>;
    fn get_by_dedup(&self, parent_thread_id: &str, key: &str) -> Result<Option<String>>;
    fn insert_dedup(&self, parent_thread_id: &str, key: &str, thread_id: &str) -> Result<()>;
    fn claim_spawn(&self, parent: &str, key: &str, candidate_id: &str) -> Result<ClaimResult>;

    /// Atomically claim a spawn slot and insert the thread row.
    /// Returns ClaimResult::New if this caller won the race (thread inserted),
    /// or ClaimResult::Existing(existing_id) if another caller already claimed it.
    fn atomic_spawn_thread(&self, parent: &str, key: &str, thread: &ThreadRow) -> Result<ClaimResult>;
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
        let conn = self.conn.lock().map_err(|_| anyhow::anyhow!("store mutex poisoned"))?;
        conn.execute(
            r#"
            INSERT INTO threads (id, parent_thread_id, status, agent_name, created_at, depth)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
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
                thread.depth,
            ],
        )?;
        Ok(())
    }

    fn get_thread(&self, id: &str) -> Result<Option<ThreadRow>> {
        let conn = self.conn.lock().map_err(|_| anyhow::anyhow!("store mutex poisoned"))?;
        let result = conn.query_row(
            "SELECT id, parent_thread_id, status, agent_name, created_at, depth FROM threads WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                Ok(ThreadRow {
                    id: row.get(0)?,
                    parent_thread_id: row.get(1)?,
                    status: row.get(2)?,
                    agent_name: row.get(3)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?,
                    depth: row.get(5)?,
                })
            },
        );

        match result {
            Ok(thread) => Ok(Some(thread)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn get_all_threads(&self) -> Result<Vec<ThreadRow>> {
        let conn = self.conn.lock().map_err(|_| anyhow::anyhow!("store mutex poisoned"))?;
        let mut stmt = conn.prepare(
            "SELECT id, parent_thread_id, status, agent_name, created_at, depth FROM threads"
        )?;

        let threads = stmt.query_map([], |row| {
            Ok(ThreadRow {
                id: row.get(0)?,
                parent_thread_id: row.get(1)?,
                status: row.get(2)?,
                agent_name: row.get(3)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?,
                depth: row.get(5)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(threads)
    }

    fn get_by_dedup(&self, parent_thread_id: &str, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().map_err(|_| anyhow::anyhow!("store mutex poisoned"))?;
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
        let conn = self.conn.lock().map_err(|_| anyhow::anyhow!("store mutex poisoned"))?;
        conn.execute(
            "INSERT OR IGNORE INTO spawn_dedup (parent_thread_id, key, thread_id) VALUES (?1, ?2, ?3)",
            rusqlite::params![parent_thread_id, key, thread_id],
        )?;
        Ok(())
    }

    fn claim_spawn(&self, parent: &str, key: &str, candidate_id: &str) -> Result<ClaimResult> {
        let mut conn = self.conn.lock().map_err(|_| anyhow::anyhow!("store mutex poisoned"))?;
        let tx = conn.transaction()?;

        // Try to insert new dedup entry
        tx.execute(
            "INSERT OR IGNORE INTO spawn_dedup (parent_thread_id, key, thread_id) VALUES (?1, ?2, ?3)",
            rusqlite::params![parent, key, candidate_id],
        )?;

        // Get the winner (either our candidate or existing)
        let winner: String = tx.query_row(
            "SELECT thread_id FROM spawn_dedup WHERE parent_thread_id=?1 AND key=?2",
            rusqlite::params![parent, key],
            |r| r.get(0),
        )?;

        tx.commit()?;

        if winner == candidate_id {
            Ok(ClaimResult::New)
        } else {
            Ok(ClaimResult::Existing(winner))
        }
    }

    fn atomic_spawn_thread(&self, parent: &str, key: &str, thread: &ThreadRow) -> Result<ClaimResult> {
        let mut conn = self.conn.lock().map_err(|_| anyhow::anyhow!("store mutex poisoned"))?;
        let tx = conn.transaction()?;

        // Try to insert new dedup entry
        tx.execute(
            "INSERT OR IGNORE INTO spawn_dedup (parent_thread_id, key, thread_id) VALUES (?1, ?2, ?3)",
            rusqlite::params![parent, key, thread.id],
        )?;

        // Get the winner (either our candidate or existing)
        let winner: String = tx.query_row(
            "SELECT thread_id FROM spawn_dedup WHERE parent_thread_id=?1 AND key=?2",
            rusqlite::params![parent, key],
            |r| r.get(0),
        )?;

        if winner == thread.id {
            // We won the race - insert the thread row
            tx.execute(
                r#"
                INSERT INTO threads (id, parent_thread_id, status, agent_name, created_at, depth)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                rusqlite::params![
                    thread.id,
                    thread.parent_thread_id,
                    thread.status,
                    thread.agent_name,
                    thread.created_at.to_rfc3339(),
                    thread.depth,
                ],
            )?;

            tx.commit()?;
            Ok(ClaimResult::New)
        } else {
            // Someone else won - don't insert thread, just return existing ID
            tx.commit()?;
            Ok(ClaimResult::Existing(winner))
        }
    }
}
