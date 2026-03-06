use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Openai(#[from] crate::dialect::openai::mapper::Error),
    #[error("zai dialect is not implemented yet")]
    ZaiNotImplemented,
}
