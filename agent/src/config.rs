use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub store_dir: PathBuf,
    pub max_parallel_tools: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            store_dir: PathBuf::from(".agent/sessions"),
            max_parallel_tools: 4,
        }
    }
}
