use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnRequest {
    pub parent_thread_id: String,
    pub key: String,
    pub agent_name: String,
    pub initial_input: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnResponse {
    pub thread_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WaitMode {
    Any,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitRequest {
    pub thread_ids: Vec<String>,
    pub mode: WaitMode,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitResponse {
    pub timed_out: bool,
    pub statuses: HashMap<String, String>,
}

