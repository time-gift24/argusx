pub mod error;
pub mod core;
pub mod permission;
pub mod persistence;
pub mod config;
pub mod api;

use permission::guard::SpawnGuards;
use persistence::store::{SqliteThreadStore, ThreadStore};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AgentCenter {
    guards: SpawnGuards,
    store: Arc<SqliteThreadStore>,
    reservations: Arc<Mutex<Vec<permission::guard::SpawnReservation>>>,
}

pub struct AgentCenterBuilder {
    max_concurrent: usize,
    max_depth: u32,
    db_path: Option<PathBuf>,
}

impl Default for AgentCenterBuilder {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            max_depth: 3,
            db_path: None,
        }
    }
}

impl AgentCenter {
    pub fn builder() -> AgentCenterBuilder {
        AgentCenterBuilder::default()
    }

    pub async fn spawn(&self, req: api::center::SpawnRequest) -> anyhow::Result<api::center::SpawnResponse> {
        // Check dedup first (idempotency)
        if let Some(existing_thread_id) = self.store.get_by_dedup(&req.parent_thread_id, &req.key)? {
            return Ok(api::center::SpawnResponse {
                thread_id: existing_thread_id,
            });
        }

        // Reserve slot (concurrency control)
        let parent_depth = 0; // TODO: Look up parent depth from store
        let reservation = self.guards.reserve(parent_depth)?;

        // Generate new thread ID
        let thread_id = format!("thread-{}", uuid::Uuid::new_v4());

        // Create thread record
        let thread = persistence::models::ThreadRow {
            id: thread_id.clone(),
            parent_thread_id: Some(req.parent_thread_id.clone()),
            status: "Pending".to_string(),
            agent_name: req.agent_name.clone(),
            created_at: chrono::Utc::now(),
        };

        // Persist thread and dedup mapping
        self.store.upsert_thread(&thread)?;
        self.store.insert_dedup(&req.parent_thread_id, &req.key, &thread_id)?;

        // Store reservation to keep slot occupied
        self.reservations.lock().await.push(reservation);

        // TODO: Dispatch initial input to agent runtime (Task 10)

        Ok(api::center::SpawnResponse { thread_id })
    }
}

impl AgentCenterBuilder {
    pub fn max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }

    pub fn max_depth(mut self, max: u32) -> Self {
        self.max_depth = max;
        self
    }

    pub fn db_path(mut self, path: PathBuf) -> Self {
        self.db_path = Some(path);
        self
    }

    pub fn build(self) -> anyhow::Result<AgentCenter> {
        let db_path = self.db_path.unwrap_or_else(|| {
            std::env::temp_dir().join("agent-center-default.db")
        });

        let store = SqliteThreadStore::new(&db_path)?;

        Ok(AgentCenter {
            guards: SpawnGuards::new(self.max_concurrent, self.max_depth),
            store: Arc::new(store),
            reservations: Arc::new(Mutex::new(Vec::new())),
        })
    }
}
