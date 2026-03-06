// Adapted from reqwest-eventsource v0.6.0 (MIT OR Apache-2.0).
// Local modifications:
// - Error payloads simplified for llm-client.
// - Added conversion from internal EventStream parser errors.

use crate::event_stream::EventStreamError;
use reqwest::Error as ReqwestError;
use reqwest::StatusCode;
use std::fmt;
use std::string::FromUtf8Error;

/// Error raised when a `RequestBuilder` cannot be cloned.
#[derive(Debug, Clone, Copy)]
pub struct CannotCloneRequestError;

impl fmt::Display for CannotCloneRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("expected a cloneable request")
    }
}

impl std::error::Error for CannotCloneRequestError {}

/// SSE connection/stream errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Utf8(FromUtf8Error),

    #[error("invalid SSE parser frame: {0}")]
    Parser(String),

    #[error(transparent)]
    Transport(ReqwestError),

    #[error("invalid content-type for SSE: {0:?}")]
    InvalidContentType(Option<String>),

    #[error("invalid status code for SSE: {0}")]
    InvalidStatusCode(StatusCode),

    #[error("invalid Last-Event-ID: {0}")]
    InvalidLastEventId(String),

    #[error("stream ended")]
    StreamEnded,

    #[error(transparent)]
    CannotCloneRequest(#[from] CannotCloneRequestError),
}

impl From<EventStreamError<ReqwestError>> for Error {
    fn from(err: EventStreamError<ReqwestError>) -> Self {
        match err {
            EventStreamError::Utf8(err) => Self::Utf8(err),
            EventStreamError::Parser(err) => Self::Parser(err.to_string()),
            EventStreamError::Transport(err) => Self::Transport(err),
        }
    }
}

impl From<ReqwestError> for Error {
    fn from(err: ReqwestError) -> Self {
        Self::Transport(err)
    }
}
