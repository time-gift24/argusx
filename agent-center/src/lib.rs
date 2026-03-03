pub mod error;
pub mod core;

pub struct AgentCenter;
pub struct AgentCenterBuilder;

impl AgentCenter {
    pub fn builder() -> AgentCenterBuilder {
        AgentCenterBuilder
    }
}
