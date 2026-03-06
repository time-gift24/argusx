use crate::chunk::ChatCompletionsChunk;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("parse error: {0}")]
    Parse(#[from] serde_json::Error),
}

pub fn parse_chunk(raw: &str) -> Result<ChatCompletionsChunk, Error> {
    serde_json::from_str(raw).map_err(Error::Parse)
}
