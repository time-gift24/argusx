use std::sync::Arc;

use crate::{agent::Agent, config::AgentConfig, error::AgentFacadeError};
use agent_session::{SessionConfig, SessionRuntime};

pub struct AgentBuilder<L>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
{
    model: Option<Arc<L>>,
    tools: Option<Arc<agent_tool::AgentToolRuntime>>,
    store_dir: Option<std::path::PathBuf>,
    max_parallel_tools: usize,
}

impl<L> AgentBuilder<L>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            model: None,
            tools: None,
            store_dir: None,
            max_parallel_tools: 4,
        }
    }

    pub fn model(mut self, model: Arc<L>) -> Self {
        self.model = Some(model);
        self
    }

    pub fn tools(mut self, tools: Arc<agent_tool::AgentToolRuntime>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn store_dir(mut self, store_dir: std::path::PathBuf) -> Self {
        self.store_dir = Some(store_dir);
        self
    }

    pub fn max_parallel_tools(mut self, max_parallel_tools: usize) -> Self {
        self.max_parallel_tools = max_parallel_tools;
        self
    }

    pub async fn build(self) -> Result<Agent<L>, AgentFacadeError> {
        let model = self.model.ok_or_else(|| AgentFacadeError::InvalidInput {
            message: "model is required".to_string(),
        })?;

        let tools = match self.tools {
            Some(tools) => tools,
            None => Arc::new(agent_tool::AgentToolRuntime::default_with_builtins().await),
        };

        let default_config = AgentConfig::default();
        let store_dir = self.store_dir.unwrap_or(default_config.store_dir);
        let max_parallel_tools = if self.max_parallel_tools == 0 {
            default_config.max_parallel_tools
        } else {
            self.max_parallel_tools
        };

        let runtime = SessionRuntime::with_config(
            store_dir,
            model,
            tools,
            SessionConfig { max_parallel_tools },
        );

        Ok(Agent::new(Arc::new(runtime)))
    }
}

impl<L> Default for AgentBuilder<L>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}
