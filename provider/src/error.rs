use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Transport,
    HttpStatus,
    Parse,
    Protocol,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamError {
    pub kind: ErrorKind,
    pub message: String,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid config: {0}")]
    Config(String),
    #[error(transparent)]
    Openai(#[from] crate::dialect::openai::mapper::Error),
    #[error(transparent)]
    Zai(#[from] crate::dialect::zai::mapper::Error),
}
