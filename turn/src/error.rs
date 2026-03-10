use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TurnError {
    #[error("turn runtime error: {0}")]
    Runtime(String),
}
