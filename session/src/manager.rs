use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use tokio::sync::broadcast;
use turn::{ModelRunner, ToolAuthorizer, ToolRunner, TurnEvent};
use uuid::Uuid;

use crate::{
    store::ThreadStore,
    thread::ThreadRuntime,
    types::{
        SessionRecord, ThreadEvent,
        ThreadLifecycle, ThreadRecord, TurnRecord,
    },
    Thread,
};

const DEFAULT_MODEL: &str = "gpt-5";

#[derive(Debug, Clone)]
pub enum SessionEvent {
    Thread {
        thread_id: Uuid,
        event: ThreadEvent,
    },
    Turn {
        thread_id: Uuid,
        turn_id: Uuid,
        event: TurnEvent,
    },
}

#[derive(Clone)]
pub struct TurnDependencies {
    pub model: Arc<dyn ModelRunner>,
    pub tool_runner: Arc<dyn ToolRunner>,
    pub authorizer: Arc<dyn ToolAuthorizer>,
}

pub struct SessionManager {
    session_id: String,
    store: ThreadStore,
    runtime: Arc<Mutex<SessionRuntime>>,
    events_tx: broadcast::Sender<SessionEvent>,
    /// Cached Thread aggregates for stable identity.
    /// This ensures get_thread() returns the same instance for the same thread_id.
    thread_cache: Mutex<HashMap<Uuid, Arc<Thread>>>,
}

impl SessionManager {
    pub fn new(session_id: String, store: ThreadStore) -> Self {
        let (events_tx, _) = broadcast::channel(64);
        Self {
            session_id,
            store,
            runtime: Arc::new(Mutex::new(SessionRuntime::default())),
            events_tx,
            thread_cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn active_thread_id(&self) -> Option<Uuid> {
        self.lock_runtime().active_thread_id
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SessionEvent> {
        self.events_tx.subscribe()
    }

    pub async fn initialize(&self) -> Result<u64> {
        let interrupted = self.store.mark_incomplete_turns_interrupted().await?;
        let mut runtime = self.lock_runtime();
        runtime.active_thread_id = None;
        runtime.threads.clear();
        Ok(interrupted)
    }

    pub async fn create_thread(&self, title: Option<String>) -> Result<Uuid> {
        self.ensure_session().await?;

        let now = Utc::now();
        let thread = ThreadRecord {
            id: Uuid::new_v4(),
            session_id: self.session_id.clone(),
            title,
            lifecycle: ThreadLifecycle::Open,
            created_at: now,
            updated_at: now,
            last_turn_number: 0,
        };
        self.store.insert_thread(&thread).await?;

        {
            let mut runtime = self.lock_runtime();
            runtime
                .threads
                .entry(thread.id)
                .or_insert_with(|| ThreadRuntime::new(thread.id));
            runtime.active_thread_id = Some(thread.id);
        }

        self.emit_thread_event(thread.id, ThreadEvent::ThreadCreated);
        self.emit_thread_event(thread.id, ThreadEvent::ThreadActivated);
        Ok(thread.id)
    }

    pub async fn switch_thread(&self, thread_id: Uuid) -> Result<()> {
        let thread = self
            .store
            .get_thread(thread_id)
            .await?
            .with_context(|| format!("thread not found: {thread_id}"))?;
        if thread.session_id != self.session_id {
            bail!(
                "thread {thread_id} does not belong to session {}",
                self.session_id
            );
        }

        {
            let mut runtime = self.lock_runtime();
            runtime
                .threads
                .entry(thread_id)
                .or_insert_with(|| ThreadRuntime::new(thread_id));
            runtime.active_thread_id = Some(thread_id);
        }

        self.emit_thread_event(thread_id, ThreadEvent::ThreadActivated);
        Ok(())
    }

    pub async fn list_threads(&self) -> Result<Vec<ThreadRecord>> {
        self.store.list_threads(&self.session_id).await
    }

    pub async fn load_thread_history(&self, thread_id: Uuid) -> Result<Vec<TurnRecord>> {
        self.store.list_turns(thread_id).await
    }

    /// Get a thread aggregate for turn operations.
    /// This allows callers to use Thread's native turn lifecycle methods.
    /// Returns the same Thread instance for the same thread_id (stable identity).
    /// The deps parameter is optional - if not provided, send_message won't work but
    /// other operations like resolve_permission will.
    pub async fn get_thread(&self, thread_id: Uuid, deps: Option<TurnDependencies>) -> Result<Arc<Thread>> {
        let record = self
            .store
            .get_thread(thread_id)
            .await?
            .with_context(|| format!("thread not found: {thread_id}"))?;
        if record.session_id != self.session_id {
            bail!(
                "thread {thread_id} does not belong to session {}",
                self.session_id
            );
        }

        // Check cache first
        {
            let cache = self.thread_cache.lock().unwrap();
            if let Some(cached) = cache.get(&thread_id) {
                return Ok(Arc::clone(cached));
            }
        }

        let (event_tx, _) = broadcast::channel(64);
        let db: crate::database::DynSessionDatabase = Arc::new(self.store.clone()).as_database();
        let events_tx = self.events_tx.clone();

        let thread = Arc::new(Thread::new(
            record,
            self.session_id.clone(),
            event_tx,
            db,
            deps.map(Arc::new),
            Some(events_tx),
        ));

        // Cache the new instance
        {
            let mut cache = self.thread_cache.lock().unwrap();
            cache.insert(thread_id, Arc::clone(&thread));
        }

        Ok(thread)
    }

    // Note: Turn lifecycle methods (send_message, resolve_permission, cancel_turn)
    // have been moved to Thread. Use get_thread() to obtain a Thread aggregate
    // and call methods directly on it.

    fn emit_thread_event(&self, thread_id: Uuid, event: ThreadEvent) {
        let _ = self
            .events_tx
            .send(SessionEvent::Thread { thread_id, event });
    }

    fn lock_runtime(&self) -> MutexGuard<'_, SessionRuntime> {
        lock_session_runtime(&self.runtime)
    }

    async fn ensure_session(&self) -> Result<()> {
        let now = Utc::now();
        self.store
            .upsert_session(&SessionRecord {
                id: self.session_id.clone(),
                user_id: None,
                default_model: DEFAULT_MODEL.into(),
                system_prompt: None,
                created_at: now,
                updated_at: now,
            })
            .await
    }
}

#[derive(Debug, Default)]
pub struct SessionRuntime {
    pub active_thread_id: Option<Uuid>,
    pub threads: HashMap<Uuid, ThreadRuntime>,
}

fn lock_session_runtime(runtime: &Arc<Mutex<SessionRuntime>>) -> MutexGuard<'_, SessionRuntime> {
    // SessionRuntime 只有派生性的内存态，真正的稳定真相在 store 里；锁被 poison 时优先恢复而不是把后续所有调用都 panic 掉。
    runtime.lock().unwrap_or_else(|poisoned| {
        eprintln!("session runtime mutex poisoned, recovering in-memory state");
        poisoned.into_inner()
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[test]
    fn lock_session_runtime_recovers_from_poisoned_mutex() {
        let runtime = Arc::new(Mutex::new(SessionRuntime::default()));
        let poison_runtime = Arc::clone(&runtime);

        let _ = std::panic::catch_unwind(move || {
            let _guard = poison_runtime.lock().unwrap();
            panic!("poison session runtime");
        });

        assert!(runtime.is_poisoned());

        let guard = lock_session_runtime(&runtime);
        assert!(guard.active_thread_id.is_none());
    }
}
