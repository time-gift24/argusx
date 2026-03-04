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
    pub status: String,
    pub agent_name: String,
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
    #[serde(default)]
    pub snapshots: HashMap<String, ThreadSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThreadToolSnapshot {
    pub call_id: String,
    pub tool_name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThreadSnapshot {
    pub thread_id: String,
    pub status: String,
    pub agent_name: String,
    #[serde(default)]
    pub active_tools: Vec<ThreadToolSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseRequest {
    pub thread_id: String,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseResponse {
    pub final_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconcileReport {
    pub repaired_count: usize,
}
