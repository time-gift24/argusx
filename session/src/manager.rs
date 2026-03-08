use std::collections::HashMap;

use uuid::Uuid;

use crate::thread::ThreadRuntime;

#[derive(Debug, Default)]
pub struct SessionManager {
    sessions: HashMap<String, SessionRuntime>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}

#[derive(Debug, Default)]
pub struct SessionRuntime {
    pub active_thread_id: Option<Uuid>,
    pub threads: HashMap<Uuid, ThreadRuntime>,
}
