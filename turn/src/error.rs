use thiserror::Error;

#[derive(Debug, Error)]
pub enum TurnError {
    #[error("turn runtime error: {0}")]
    Runtime(String),
}
