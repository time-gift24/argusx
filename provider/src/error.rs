use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Openai(#[from] crate::dialect::openai::mapper::Error),
    #[error(transparent)]
    Zai(#[from] crate::dialect::zai::mapper::Error),
}
