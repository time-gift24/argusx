use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::{broadcast, Mutex};
use uuid::Uuid;

use crate::error::{SessionError, SessionResult};
use crate::store::ThreadStore;
use crate::types::{ThreadEvent, ThreadEventEnvelope, ThreadLifecycle, ThreadRecord};

pub struct Session {
    session_id: String,
    active_thread_id: Option<Uuid>,
    threads: HashMap<Uuid, Arc<Mutex<Thread>>>,
    db: Arc<ThreadStore>,
    event_tx: broadcast::Sender<ThreadEventEnvelope>,
}

struct Thread {
    thread_id: Uuid,
    session_id: String,
    lifecycle: ThreadLifecycle,
}

impl Session {
    pub fn new(session_id: String, db: Arc<ThreadStore>) -> Self {
        let (event_tx, _) = broadcast::channel(64);
        Self {
            session_id,
            active_thread_id: None,
            threads: HashMap::new(),
            db,
            event_tx,
        }
    }

    pub async fn create_thread(&mut self, title: Option<String>) -> SessionResult<Uuid> {
        let thread_id = Uuid::new_v4();
        let now = Utc::now();

        let record = ThreadRecord {
            id: thread_id,
            session_id: self.session_id.clone(),
            title,
            lifecycle: ThreadLifecycle::Open,
            created_at: now,
            updated_at: now,
            last_turn_number: 0,
        };

        self.db.insert_thread(&record).await.map_err(SessionError::Database)?;

        let thread = Thread {
            thread_id,
            session_id: self.session_id.clone(),
            lifecycle: ThreadLifecycle::Open,
        };

        self.threads.insert(thread_id, Arc::new(Mutex::new(thread)));
        self.active_thread_id = Some(thread_id);

        // Emit thread created event
        let _ = self.event_tx.send(ThreadEventEnvelope {
            thread_id,
            turn_id: None,
            event: ThreadEvent::ThreadCreated,
        });

        Ok(thread_id)
    }

    pub async fn switch_thread(&mut self, thread_id: Uuid) -> SessionResult<()> {
        if !self.threads.contains_key(&thread_id) {
            // Try to load from DB
            if let Some(record) = self.db.get_thread(thread_id).await.map_err(SessionError::Database)? {
                let thread = Thread {
                    thread_id: record.id,
                    session_id: record.session_id,
                    lifecycle: record.lifecycle,
                };
                self.threads.insert(thread_id, Arc::new(Mutex::new(thread)));
            } else {
                return Err(SessionError::ThreadNotFound(thread_id));
            }
        }

        self.active_thread_id = Some(thread_id);

        // Emit thread activated event
        let _ = self.event_tx.send(ThreadEventEnvelope {
            thread_id,
            turn_id: None,
            event: ThreadEvent::ThreadActivated,
        });

        Ok(())
    }

    pub async fn list_threads(&self) -> SessionResult<Vec<ThreadRecord>> {
        self.db.list_threads(&self.session_id).await.map_err(SessionError::Database)
    }

    pub async fn get_thread(
        &mut self,
        thread_id: Uuid,
    ) -> SessionResult<Arc<Mutex<Thread>>> {
        if let Some(thread) = self.threads.get(&thread_id) {
            return Ok(thread.clone());
        }

        // Try to load from DB
        if let Some(record) = self.db.get_thread(thread_id).await.map_err(SessionError::Database)? {
            let thread = Thread {
                thread_id: record.id,
                session_id: record.session_id,
                lifecycle: record.lifecycle,
            };
            let thread = Arc::new(Mutex::new(thread));
            self.threads.insert(thread_id, thread.clone());
            Ok(thread)
        } else {
            Err(SessionError::ThreadNotFound(thread_id))
        }
    }

    pub async fn subscribe(
        &mut self,
        thread_id: Uuid,
    ) -> SessionResult<broadcast::Receiver<ThreadEventEnvelope>> {
        // Ensure thread exists
        self.get_thread(thread_id).await?;
        Ok(self.event_tx.subscribe())
    }
}
