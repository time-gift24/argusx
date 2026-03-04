use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadRow {
    pub id: String,
    pub parent_thread_id: Option<String>,
    pub status: String,
    pub agent_name: String,
    pub created_at: DateTime<Utc>,
    pub depth: u32,
    pub initial_input: Option<String>,
}
