use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub name: String,
    pub version: String,
    pub prompt: String,
    #[serde(default)]
    pub tools: Vec<String>,
    pub max_concurrent: Option<usize>,
    pub max_depth: Option<u32>,
}
