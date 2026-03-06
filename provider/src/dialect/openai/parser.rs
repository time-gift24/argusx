use crate::dialect::openai::schema::common::{StreamError, StreamErrorStructured};
use crate::dialect::openai::schema::stream::{
    ChatCompletionsStreamChunk, ChatCompletionsStreamEvent,
};
use serde_json::Value;
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

pub fn parse_payload(payload: &str) -> Result<ChatCompletionsStreamEvent, Error> {
    let payload = payload.trim();
    if payload.is_empty() {
        return Err(Error::EmptyPayload);
    }

    if payload == "[DONE]" {
        return Ok(ChatCompletionsStreamEvent::Done);
    }

    let chunk_err = match serde_json::from_str::<ChatCompletionsStreamChunk>(payload) {
        Ok(chunk) => return Ok(ChatCompletionsStreamEvent::Chunk(chunk)),
        Err(err) => err,
    };

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

    let parsed: Value = match serde_json::from_str(payload) {
        Ok(value) => value,
        Err(_) => return Err(Error::Parse(chunk_err)),
    };

    match parsed {
        Value::Object(mut map) => {
            if let Some(error_value) = map.remove("error") {
                if let Ok(structured) =
                    serde_json::from_value::<StreamErrorStructured>(error_value.clone())
                {
                    return Ok(ChatCompletionsStreamEvent::Error(StreamError::Structured(
                        structured,
                    )));
                }

                if let Some(raw) = error_value.as_str() {
                    return Ok(ChatCompletionsStreamEvent::Error(StreamError::Raw(
                        raw.to_string(),
                    )));
                }

                return Ok(ChatCompletionsStreamEvent::Error(StreamError::Raw(
                    error_value.to_string(),
                )));
            }
        }
        Value::String(raw) => {
            return Ok(ChatCompletionsStreamEvent::Error(StreamError::Raw(raw)));
        }
        _ => {}
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
