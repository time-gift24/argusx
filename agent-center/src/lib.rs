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
            status: "Running".to_string(),  // Spawned threads start in Running state
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

    pub async fn wait(&self, req: api::center::WaitRequest) -> anyhow::Result<api::center::WaitResponse> {
        // Clamp timeout to [1000, 300000] ms
        let timeout_ms = req.timeout_ms.clamp(1000, 300000);
        let timeout = tokio::time::Duration::from_millis(timeout_ms);

        let result = tokio::time::timeout(timeout, async {
            loop {
                // Query all thread statuses
                let mut statuses = std::collections::HashMap::new();
                let mut all_terminal = true;
                let mut any_terminal = false;

                for thread_id in &req.thread_ids {
                    if let Some(thread) = self.store.get_thread(thread_id)? {
                        let is_terminal = matches!(
                            thread.status.as_str(),
                            "Succeeded" | "Failed" | "Cancelled" | "Closed"
                        );
                        statuses.insert(thread_id.clone(), thread.status.clone());
                        all_terminal = all_terminal && is_terminal;
                        any_terminal = any_terminal || is_terminal;
                    } else {
                        statuses.insert(thread_id.clone(), "NotFound".to_string());
                    }
                }

                // Check if condition satisfied
                let satisfied = match req.mode {
                    api::center::WaitMode::Any => any_terminal,
                    api::center::WaitMode::All => all_terminal,
                };

                if satisfied {
                    return Ok((false, statuses));
                }

                // Sleep briefly to avoid busy loop (100ms polling interval)
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }).await;

        match result {
            Ok(Ok((timed_out, statuses))) => Ok(api::center::WaitResponse { timed_out, statuses }),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                // Timeout - collect final statuses
                let mut statuses = std::collections::HashMap::new();
                for thread_id in &req.thread_ids {
                    if let Some(thread) = self.store.get_thread(thread_id)? {
                        statuses.insert(thread_id.clone(), thread.status.clone());
                    } else {
                        statuses.insert(thread_id.clone(), "NotFound".to_string());
                    }
                }
                Ok(api::center::WaitResponse { timed_out: true, statuses })
            }
        }
    }

    pub async fn close(&self, req: api::center::CloseRequest) -> anyhow::Result<api::center::CloseResponse> {
        // Get current thread state
        let thread = self.store.get_thread(&req.thread_id)?
            .ok_or_else(|| anyhow::anyhow!("Thread not found: {}", req.thread_id))?;

        // Parse current status
        let current_status = self.parse_status(&thread.status);

        // If already closed, return idempotently
        if matches!(current_status, core::lifecycle::ThreadStatus::Closed) {
            return Ok(api::center::CloseResponse {
                final_status: "Closed".to_string(),
            });
        }

        // Create state machine and transition
        let mut sm = core::lifecycle::ThreadStateMachine::new(current_status);

        // Transition to Closing
        if sm.status() != core::lifecycle::ThreadStatus::Closing {
            sm.transition_to(core::lifecycle::ThreadStatus::Closing)
                .map_err(|e| anyhow::anyhow!("Invalid state transition: {:?}", e))?;
        }

        // Transition to Closed
        sm.transition_to(core::lifecycle::ThreadStatus::Closed)
            .map_err(|e| anyhow::anyhow!("Invalid state transition: {:?}", e))?;

        // Persist final state
        let updated_thread = persistence::models::ThreadRow {
            status: "Closed".to_string(),
            ..thread
        };
        self.store.upsert_thread(&updated_thread)?;

        // TODO: Release reservation (Task 9 will implement proper reservation tracking)

        Ok(api::center::CloseResponse {
            final_status: "Closed".to_string(),
        })
    }

    fn parse_status(&self, status: &str) -> core::lifecycle::ThreadStatus {
        match status {
            "Pending" => core::lifecycle::ThreadStatus::Pending,
            "Running" => core::lifecycle::ThreadStatus::Running,
            "Succeeded" => core::lifecycle::ThreadStatus::Succeeded,
            "Failed" => core::lifecycle::ThreadStatus::Failed,
            "Cancelled" => core::lifecycle::ThreadStatus::Cancelled,
            "Closing" => core::lifecycle::ThreadStatus::Closing,
            "Closed" => core::lifecycle::ThreadStatus::Closed,
            _ => core::lifecycle::ThreadStatus::Failed, // Default to Failed for unknown states
        }
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
            std::env::temp_dir().join(format!("agent-center-{}.db", uuid::Uuid::new_v4()))
        });

        let store = SqliteThreadStore::new(&db_path)?;

        Ok(AgentCenter {
            guards: SpawnGuards::new(self.max_concurrent, self.max_depth),
            store: Arc::new(store),
            reservations: Arc::new(Mutex::new(Vec::new())),
        })
    }
}
