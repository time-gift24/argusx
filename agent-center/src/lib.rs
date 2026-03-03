pub mod error;
pub mod core;
pub mod permission;

pub struct AgentCenter;
pub struct AgentCenterBuilder;

impl AgentCenter {
    pub fn builder() -> AgentCenterBuilder {
        AgentCenterBuilder
    }
}
