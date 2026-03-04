pub mod api;
pub mod config;
pub mod core;
pub mod dispatch;
pub mod error;
pub mod permission;
pub mod persistence;
pub mod tools;

use permission::guard::SpawnGuards;
use persistence::store::{ClaimResult, SqliteThreadStore, ThreadStore};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

#[derive(Debug, Clone, Default)]
struct RuntimeToolSnapshot {
    call_id: String,
    tool_name: String,
    status: String,
}

#[derive(Debug, Clone, Default)]
struct RuntimeThreadSnapshot {
    status: Option<String>,
    agent_name: Option<String>,
    active_tools: HashMap<String, RuntimeToolSnapshot>,
    error: Option<String>,
}

pub struct AgentCenter {
    guards: SpawnGuards,
    store: Arc<SqliteThreadStore>,
    reservations: Arc<Mutex<HashMap<String, permission::guard::SpawnReservation>>>,
    dispatcher: Arc<RwLock<Option<Arc<dyn dispatch::ThreadDispatcher>>>>,
    runtime_snapshots: Arc<Mutex<HashMap<String, RuntimeThreadSnapshot>>>,
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
            max_depth: 1,
            db_path: None,
        }
    }
}

impl AgentCenter {
    pub fn builder() -> AgentCenterBuilder {
        AgentCenterBuilder::default()
    }

    pub async fn spawn(
        &self,
        req: api::center::SpawnRequest,
    ) -> anyhow::Result<api::center::SpawnResponse> {
        // Validate inputs
        if req.parent_thread_id.is_empty() || req.key.is_empty() || req.agent_name.is_empty() {
            return Err(anyhow::anyhow!(
                "parent_thread_id, key, and agent_name must not be empty"
            ));
        }
        if req.key.len() > 256 || req.agent_name.len() > 256 {
            return Err(anyhow::anyhow!(
                "key and agent_name must be <= 256 characters"
            ));
        }

        // Fast path: check if dedup entry already exists (idempotent case)
        // This avoids consuming concurrency quota for duplicate spawns
        if let Some(existing_thread_id) =
            self.store.get_by_dedup(&req.parent_thread_id, &req.key)?
        {
            let (status, agent_name) = self
                .store
                .get_thread(&existing_thread_id)?
                .map(|thread| (thread.status, thread.agent_name))
                .unwrap_or_else(|| ("Running".to_string(), req.agent_name.clone()));
            return Ok(api::center::SpawnResponse {
                thread_id: existing_thread_id,
                status,
                agent_name,
            });
        }

        // Slow path: need to create new thread
        // Look up parent and validate depth
        let (parent_depth, is_root) = if req.parent_thread_id == "root" {
            // Allow "root" as a special case for top-level threads
            (0, true)
        } else if let Some(parent) = self.store.get_thread(&req.parent_thread_id)? {
            // Parent exists - use its depth
            (parent.depth, false)
        } else {
            // Parent doesn't exist and isn't "root" - reject to prevent bypass
            return Err(anyhow::anyhow!(
                "Parent thread {} does not exist. Use 'root' for top-level threads.",
                req.parent_thread_id
            ));
        };

        // Reserve slot (fail fast if at limit)
        let reservation = self.guards.reserve(parent_depth)?;

        // Calculate child depth
        let child_depth = parent_depth + 1;

        // Generate candidate thread ID
        let candidate_id = format!("thread-{}", uuid::Uuid::new_v4());

        // Create thread row with initial_input
        let thread = persistence::models::ThreadRow {
            id: candidate_id.clone(),
            parent_thread_id: if is_root {
                None
            } else {
                Some(req.parent_thread_id.clone())
            },
            status: "Running".to_string(),
            agent_name: req.agent_name.clone(),
            created_at: chrono::Utc::now(),
            depth: child_depth,
            initial_input: Some(req.initial_input.clone()),
        };

        // Atomically claim dedup slot and insert thread
        // Note: In race condition, another caller might have inserted between our get_by_dedup check and here
        match self
            .store
            .atomic_spawn_thread(&req.parent_thread_id, &req.key, &thread)?
        {
            ClaimResult::Existing(existing_thread_id) => {
                // Lost the race - drop reservation (another caller already created the thread)
                drop(reservation);
                let (status, agent_name) = self
                    .store
                    .get_thread(&existing_thread_id)?
                    .map(|thread| (thread.status, thread.agent_name))
                    .unwrap_or_else(|| ("Running".to_string(), req.agent_name.clone()));
                Ok(api::center::SpawnResponse {
                    thread_id: existing_thread_id,
                    status,
                    agent_name,
                })
            }
            ClaimResult::New => {
                // Won the race - thread already inserted atomically
                // Store reservation mapped to thread_id
                self.reservations
                    .lock()
                    .await
                    .insert(candidate_id.clone(), reservation);

                {
                    let mut snapshots = self.runtime_snapshots.lock().await;
                    snapshots.insert(
                        candidate_id.clone(),
                        RuntimeThreadSnapshot {
                            status: Some("Running".to_string()),
                            agent_name: Some(req.agent_name.clone()),
                            active_tools: HashMap::new(),
                            error: None,
                        },
                    );
                }

                let dispatcher = self.dispatcher.read().await.clone();
                if let Some(dispatcher) = dispatcher {
                    let dispatch_req = dispatch::DispatchRequest {
                        thread_id: candidate_id.clone(),
                        parent_thread_id: req.parent_thread_id.clone(),
                        agent_name: req.agent_name.clone(),
                        initial_input: req.initial_input.clone(),
                    };
                    tokio::spawn(async move {
                        if let Err(err) = dispatcher.dispatch(dispatch_req).await {
                            tracing::warn!("sub-agent dispatch failed: {err}");
                        }
                    });
                }

                Ok(api::center::SpawnResponse {
                    thread_id: candidate_id,
                    status: "Running".to_string(),
                    agent_name: req.agent_name,
                })
            }
        }
    }

    pub async fn set_dispatcher(&self, dispatcher: Arc<dyn dispatch::ThreadDispatcher>) {
        let mut guard = self.dispatcher.write().await;
        *guard = Some(dispatcher);
    }

    pub async fn clear_dispatcher(&self) {
        let mut guard = self.dispatcher.write().await;
        *guard = None;
    }

    pub async fn report_thread_status(
        &self,
        thread_id: &str,
        status: impl Into<String>,
    ) -> anyhow::Result<()> {
        let status = status.into();
        if let Some(thread) = self.store.get_thread(thread_id)? {
            if thread.status != status {
                let updated_thread = persistence::models::ThreadRow {
                    status: status.clone(),
                    ..thread
                };
                self.store.upsert_thread(&updated_thread)?;
            }
        }

        {
            let mut snapshots = self.runtime_snapshots.lock().await;
            let snapshot = snapshots.entry(thread_id.to_string()).or_default();
            snapshot.status = Some(status.clone());
            if Self::is_terminal_status(&status) {
                snapshot.active_tools.clear();
            }
        }

        if Self::is_terminal_status(&status) {
            let _ = self.reservations.lock().await.remove(thread_id);
        }

        Ok(())
    }

    pub async fn report_thread_error(
        &self,
        thread_id: &str,
        error: impl Into<String>,
    ) -> anyhow::Result<()> {
        let mut snapshots = self.runtime_snapshots.lock().await;
        let snapshot = snapshots.entry(thread_id.to_string()).or_default();
        snapshot.error = Some(error.into());
        Ok(())
    }

    pub async fn report_thread_tool_status(
        &self,
        thread_id: &str,
        call_id: impl Into<String>,
        tool_name: impl Into<String>,
        status: impl Into<String>,
    ) -> anyhow::Result<()> {
        let call_id = call_id.into();
        let tool_name = tool_name.into();
        let status = status.into();
        let mut snapshots = self.runtime_snapshots.lock().await;
        let snapshot = snapshots.entry(thread_id.to_string()).or_default();

        let is_active = matches!(
            status.as_str(),
            "planned" | "queued" | "waiting" | "running" | "in_progress"
        );
        if is_active {
            let tool_name = if tool_name == "tool" {
                snapshot
                    .active_tools
                    .get(&call_id)
                    .map(|existing| existing.tool_name.clone())
                    .unwrap_or(tool_name)
            } else {
                tool_name
            };
            snapshot.active_tools.insert(
                call_id.clone(),
                RuntimeToolSnapshot {
                    call_id,
                    tool_name,
                    status,
                },
            );
        } else {
            snapshot.active_tools.remove(&call_id);
        }
        Ok(())
    }

    async fn collect_wait_view(
        &self,
        thread_ids: &[String],
    ) -> anyhow::Result<(
        HashMap<String, String>,
        HashMap<String, api::center::ThreadSnapshot>,
        bool,
        bool,
    )> {
        let runtime_snapshots = self.runtime_snapshots.lock().await.clone();
        let mut statuses = HashMap::new();
        let mut snapshots = HashMap::new();
        let mut all_terminal = true;
        let mut any_terminal = false;

        for thread_id in thread_ids {
            let persisted = self.store.get_thread(thread_id)?;
            let runtime = runtime_snapshots.get(thread_id);

            let status = runtime
                .and_then(|snapshot| snapshot.status.clone())
                .or_else(|| persisted.as_ref().map(|thread| thread.status.clone()))
                .unwrap_or_else(|| "NotFound".to_string());
            let is_terminal = Self::is_terminal_status(&status);
            all_terminal &= is_terminal;
            any_terminal |= is_terminal;

            let agent_name = runtime
                .and_then(|snapshot| snapshot.agent_name.clone())
                .or_else(|| persisted.as_ref().map(|thread| thread.agent_name.clone()))
                .unwrap_or_else(|| "unknown".to_string());
            let active_tools = runtime
                .map(|snapshot| {
                    let mut values = snapshot.active_tools.values().cloned().collect::<Vec<_>>();
                    values.sort_by(|a, b| a.call_id.cmp(&b.call_id));
                    values
                        .into_iter()
                        .map(|tool| api::center::ThreadToolSnapshot {
                            call_id: tool.call_id,
                            tool_name: tool.tool_name,
                            status: tool.status,
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let error = runtime.and_then(|snapshot| snapshot.error.clone());

            statuses.insert(thread_id.clone(), status.clone());
            snapshots.insert(
                thread_id.clone(),
                api::center::ThreadSnapshot {
                    thread_id: thread_id.clone(),
                    status,
                    agent_name,
                    active_tools,
                    error,
                },
            );
        }

        Ok((statuses, snapshots, all_terminal, any_terminal))
    }

    pub async fn wait(
        &self,
        req: api::center::WaitRequest,
    ) -> anyhow::Result<api::center::WaitResponse> {
        // Clamp timeout to [1000, 300000] ms
        let timeout_ms = req.timeout_ms.clamp(1000, 300000);
        let timeout = tokio::time::Duration::from_millis(timeout_ms);

        let result = tokio::time::timeout(timeout, async {
            loop {
                let (statuses, snapshots, all_terminal, any_terminal) =
                    self.collect_wait_view(&req.thread_ids).await?;

                // Check if condition satisfied
                let satisfied = match req.mode {
                    api::center::WaitMode::Any => any_terminal,
                    api::center::WaitMode::All => all_terminal,
                };

                if satisfied {
                    return Ok((false, statuses, snapshots));
                }

                // Sleep briefly to avoid busy loop (100ms polling interval)
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        })
        .await;

        match result {
            Ok(Ok((timed_out, statuses, snapshots))) => Ok(api::center::WaitResponse {
                timed_out,
                statuses,
                snapshots,
            }),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                // Timeout - collect final statuses
                let (statuses, snapshots, _all_terminal, _any_terminal) =
                    self.collect_wait_view(&req.thread_ids).await?;
                Ok(api::center::WaitResponse {
                    timed_out: true,
                    statuses,
                    snapshots,
                })
            }
        }
    }

    pub async fn close(
        &self,
        req: api::center::CloseRequest,
    ) -> anyhow::Result<api::center::CloseResponse> {
        // Validate inputs
        if req.thread_id.is_empty() {
            return Err(anyhow::anyhow!("thread_id must not be empty"));
        }

        // Get current thread state
        let thread = self
            .store
            .get_thread(&req.thread_id)?
            .ok_or_else(|| anyhow::anyhow!("Thread not found: {}", req.thread_id))?;

        // Parse current status
        let current_status = self.parse_status(&thread.status)?;

        // If already terminal (Closed, Succeeded, Failed, Cancelled), return idempotently
        if matches!(
            current_status,
            core::lifecycle::ThreadStatus::Closed
                | core::lifecycle::ThreadStatus::Succeeded
                | core::lifecycle::ThreadStatus::Failed
                | core::lifecycle::ThreadStatus::Cancelled
        ) {
            // Release reservation if still held
            if let Some(_reservation) = self.reservations.lock().await.remove(&req.thread_id) {
                // Reservation dropped - slot released
            }

            let mut snapshots = self.runtime_snapshots.lock().await;
            let snapshot = snapshots.entry(req.thread_id.clone()).or_default();
            snapshot.status = Some(thread.status.clone());
            snapshot.active_tools.clear();

            // Return actual persisted terminal status, not "Closed"
            return Ok(api::center::CloseResponse {
                final_status: thread.status.clone(),
            });
        }

        // Create state machine and transition
        let mut sm = core::lifecycle::ThreadStateMachine::new(current_status);

        // Handle force close
        if req.force {
            // Force: Skip Closing state, go directly to Closed
            sm.transition_to(core::lifecycle::ThreadStatus::Closed)
                .map_err(|e| anyhow::anyhow!("Invalid state transition: {:?}", e))?;
        } else {
            // Normal: Transition through Closing state
            if sm.status() != core::lifecycle::ThreadStatus::Closing {
                sm.transition_to(core::lifecycle::ThreadStatus::Closing)
                    .map_err(|e| anyhow::anyhow!("Invalid state transition: {:?}", e))?;
            }

            // Transition to Closed
            sm.transition_to(core::lifecycle::ThreadStatus::Closed)
                .map_err(|e| anyhow::anyhow!("Invalid state transition: {:?}", e))?;
        }

        // Persist final state
        let updated_thread = persistence::models::ThreadRow {
            status: "Closed".to_string(),
            ..thread
        };
        self.store.upsert_thread(&updated_thread)?;

        // Release reservation
        if let Some(_reservation) = self.reservations.lock().await.remove(&req.thread_id) {
            // Reservation dropped - slot released
        }

        let mut snapshots = self.runtime_snapshots.lock().await;
        let snapshot = snapshots.entry(req.thread_id.clone()).or_default();
        snapshot.status = Some("Closed".to_string());
        snapshot.active_tools.clear();

        Ok(api::center::CloseResponse {
            final_status: "Closed".to_string(),
        })
    }

    /// Mark a thread as completed (Succeeded or Failed) and release its slot
    pub async fn mark_thread_complete(&self, thread_id: &str, success: bool) -> anyhow::Result<()> {
        let thread = self
            .store
            .get_thread(thread_id)?
            .ok_or_else(|| anyhow::anyhow!("Thread not found: {}", thread_id))?;

        let final_status = if success { "Succeeded" } else { "Failed" };

        let updated_thread = persistence::models::ThreadRow {
            status: final_status.to_string(),
            ..thread
        };
        self.store.upsert_thread(&updated_thread)?;

        // Release reservation
        if let Some(_reservation) = self.reservations.lock().await.remove(thread_id) {
            // Reservation dropped - slot released
        }

        let mut snapshots = self.runtime_snapshots.lock().await;
        let snapshot = snapshots.entry(thread_id.to_string()).or_default();
        snapshot.status = Some(final_status.to_string());
        snapshot.active_tools.clear();

        Ok(())
    }

    fn is_terminal_status(status: &str) -> bool {
        matches!(status, "Succeeded" | "Failed" | "Cancelled" | "Closed")
    }

    fn parse_status(&self, status: &str) -> anyhow::Result<core::lifecycle::ThreadStatus> {
        match status {
            "Pending" => Ok(core::lifecycle::ThreadStatus::Pending),
            "Running" => Ok(core::lifecycle::ThreadStatus::Running),
            "Succeeded" => Ok(core::lifecycle::ThreadStatus::Succeeded),
            "Failed" => Ok(core::lifecycle::ThreadStatus::Failed),
            "Cancelled" => Ok(core::lifecycle::ThreadStatus::Cancelled),
            "Closing" => Ok(core::lifecycle::ThreadStatus::Closing),
            "Closed" => Ok(core::lifecycle::ThreadStatus::Closed),
            _ => Err(anyhow::anyhow!("Unknown thread status: {}", status)),
        }
    }

    pub async fn reconcile(&self) -> anyhow::Result<api::center::ReconcileReport> {
        // WARNING: This method should only be called during startup, not during normal operation.
        // It marks ALL non-terminal threads as Failed, which is only safe if no runtimes are active.
        //
        // TODO: In production, implement runtime liveness check (e.g., heartbeat) before marking threads failed.

        // Get all threads from persistence
        let threads = self.store.get_all_threads()?;
        let mut repaired_count = 0;

        for thread in threads {
            let status = self.parse_status(&thread.status)?;

            // Check if thread is in non-terminal state
            let is_non_terminal = matches!(
                status,
                core::lifecycle::ThreadStatus::Pending
                    | core::lifecycle::ThreadStatus::Running
                    | core::lifecycle::ThreadStatus::Closing
            );

            if is_non_terminal {
                // Mark orphan thread as Failed (no active runtime)
                let thread_id = thread.id.clone();
                let updated_thread = persistence::models::ThreadRow {
                    status: "Failed".to_string(),
                    depth: thread.depth, // Preserve depth
                    ..thread
                };
                self.store.upsert_thread(&updated_thread)?;

                // Release any held reservation
                if let Some(_reservation) = self.reservations.lock().await.remove(&thread_id) {
                    // Reservation dropped - slot released
                }

                let mut snapshots = self.runtime_snapshots.lock().await;
                let snapshot = snapshots.entry(thread_id).or_default();
                snapshot.status = Some("Failed".to_string());
                snapshot.active_tools.clear();

                repaired_count += 1;
            }
        }

        Ok(api::center::ReconcileReport { repaired_count })
    }

    /// List available tool names
    pub fn list_tools(&self) -> Vec<String> {
        vec![
            "spawn_agent".to_string(),
            "wait".to_string(),
            "close_agent".to_string(),
        ]
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
        // Validate configuration
        if self.max_concurrent == 0 || self.max_concurrent > 10000 {
            return Err(anyhow::anyhow!(
                "max_concurrent must be between 1 and 10000"
            ));
        }
        if self.max_depth == 0 || self.max_depth > 1000 {
            return Err(anyhow::anyhow!("max_depth must be between 1 and 1000"));
        }

        let db_path = self.db_path.unwrap_or_else(|| {
            std::env::temp_dir().join(format!("agent-center-{}.db", uuid::Uuid::new_v4()))
        });

        let store = SqliteThreadStore::new(&db_path)?;

        Ok(AgentCenter {
            guards: SpawnGuards::new(self.max_concurrent, self.max_depth),
            store: Arc::new(store),
            reservations: Arc::new(Mutex::new(HashMap::new())),
            dispatcher: Arc::new(RwLock::new(None)),
            runtime_snapshots: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}
