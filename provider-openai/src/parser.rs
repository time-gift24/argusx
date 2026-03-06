use crate::schema::common::{StreamError, StreamErrorStructured};
use crate::schema::stream::{ChatCompletionsStreamChunk, ChatCompletionsStreamEvent};
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("empty payload")]
    EmptyPayload,
    #[error("parse error: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("unexpected event: {0}")]
    UnexpectedEvent(&'static str),
}

#[derive(Debug, Deserialize)]
struct ErrorEnvelope {
    error: StreamErrorStructured,
}

#[derive(Debug, Deserialize)]
struct ErrorStringEnvelope {
    error: String,
}

pub fn parse_payload(payload: &str) -> Result<ChatCompletionsStreamEvent, Error> {
    let payload = payload.trim();
    if payload.is_empty() {
        return Err(Error::EmptyPayload);
    }

    if payload == "[DONE]" {
        return Ok(ChatCompletionsStreamEvent::Done);
    }

    let chunk_attempt = serde_json::from_str::<ChatCompletionsStreamChunk>(payload);
    if let Ok(chunk) = chunk_attempt {
        return Ok(ChatCompletionsStreamEvent::Chunk(chunk));
    }

    let chunk_err = chunk_attempt.expect_err("already handled Ok above");

    let looks_like_json = payload
        .chars()
        .next()
        .map(|c| matches!(c, '{' | '[' | '"'))
        .unwrap_or(false);

    if !looks_like_json {
        return Ok(ChatCompletionsStreamEvent::Error(StreamError::Raw(
            payload.to_string(),
        )));
    }

    if let Ok(error) = serde_json::from_str::<ErrorEnvelope>(payload) {
        return Ok(ChatCompletionsStreamEvent::Error(StreamError::Structured(
            error.error,
        )));
    }

    if let Ok(error) = serde_json::from_str::<ErrorStringEnvelope>(payload) {
        return Ok(ChatCompletionsStreamEvent::Error(StreamError::Raw(
            error.error,
        )));
    }

    if let Ok(error) = serde_json::from_str::<String>(payload) {
        return Ok(ChatCompletionsStreamEvent::Error(StreamError::Raw(error)));
    }

    Err(Error::Parse(chunk_err))
}

pub fn parse_sse_line(line: &str) -> Result<Option<ChatCompletionsStreamEvent>, Error> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with(':') {
        return Ok(None);
    }

    if let Some(event) = trimmed.strip_prefix("event:") {
        if event.trim().eq_ignore_ascii_case("open") {
            return Ok(Some(ChatCompletionsStreamEvent::Open));
        }
        return Ok(None);
    }

    if let Some(payload) = trimmed.strip_prefix("data:") {
        return Ok(Some(parse_payload(payload)?));
    }

    Ok(None)
}

pub fn parse_chunk(raw: &str) -> Result<ChatCompletionsStreamChunk, Error> {
    match parse_payload(raw)? {
        ChatCompletionsStreamEvent::Chunk(chunk) => Ok(chunk),
        ChatCompletionsStreamEvent::Done => Err(Error::UnexpectedEvent("done")),
        ChatCompletionsStreamEvent::Error(_) => Err(Error::UnexpectedEvent("error")),
        ChatCompletionsStreamEvent::Open => Err(Error::UnexpectedEvent("open")),
    }
}
