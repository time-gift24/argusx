use eventsource_stream::EventStreamError;
use reqwest::Error as ReqwestError;
use reqwest::StatusCode;
use std::string::FromUtf8Error;
use thiserror::Error;

#[derive(Debug, Error)]
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
