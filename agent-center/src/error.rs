use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentCenterError {
    #[error("Not implemented")]
    NotImplemented,
}
