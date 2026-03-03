pub mod error;
pub mod core;
pub mod permission;

use permission::guard::SpawnGuards;

pub struct AgentCenter {
    #[allow(dead_code)] // Will be used in Task 6 (spawn_agent implementation)
    guards: SpawnGuards,
}

pub struct AgentCenterBuilder {
    max_concurrent: usize,
    max_depth: u32,
}

impl Default for AgentCenterBuilder {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            max_depth: 3,
        }
    }
}

impl AgentCenter {
    pub fn builder() -> AgentCenterBuilder {
        AgentCenterBuilder::default()
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

    pub fn build(self) -> AgentCenter {
        AgentCenter {
            guards: SpawnGuards::new(self.max_concurrent, self.max_depth),
        }
    }
}
