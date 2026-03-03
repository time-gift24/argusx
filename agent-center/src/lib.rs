pub mod error;

pub struct AgentCenter;
pub struct AgentCenterBuilder;

impl AgentCenter {
    pub fn builder() -> AgentCenterBuilder {
        AgentCenterBuilder
    }
}
