use crate::dialect::zai::schema::stream::{ZaiStreamChunk, ZaiStreamEvent};
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

pub fn parse_payload(payload: &str) -> Result<ZaiStreamEvent, Error> {
    let payload = payload.trim();
    if payload.is_empty() {
        return Err(Error::EmptyPayload);
    }

    if payload == "[DONE]" {
        return Ok(ZaiStreamEvent::Done);
    }

    let chunk_err = match serde_json::from_str::<ZaiStreamChunk>(payload) {
        Ok(chunk) => return Ok(ZaiStreamEvent::Chunk(chunk)),
        Err(err) => err,
    };

    let looks_like_json = payload
        .chars()
        .next()
        .map(|c| matches!(c, '{' | '[' | '"'))
        .unwrap_or(false);

    if !looks_like_json {
        return Ok(ZaiStreamEvent::Error(payload.to_string()));
    }

    let parsed: Value = match serde_json::from_str(payload) {
        Ok(value) => value,
        Err(_) => return Err(Error::Parse(chunk_err)),
    };

    if let Value::Object(mut map) = parsed
        && let Some(error_value) = map.remove("error")
    {
        if let Some(raw) = error_value.as_str() {
            return Ok(ZaiStreamEvent::Error(raw.to_string()));
        }
        if let Some(message) = error_value
            .as_object()
            .and_then(|obj| obj.get("message"))
            .and_then(Value::as_str)
        {
            return Ok(ZaiStreamEvent::Error(message.to_string()));
        }
        return Ok(ZaiStreamEvent::Error(error_value.to_string()));
    }

    Err(Error::Parse(chunk_err))
}

pub fn parse_sse_line(line: &str) -> Result<Option<ZaiStreamEvent>, Error> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with(':') {
        return Ok(None);
    }

    if let Some(event) = trimmed.strip_prefix("event:") {
        if event.trim().eq_ignore_ascii_case("open") {
            return Ok(Some(ZaiStreamEvent::Open));
        }
        return Ok(None);
    }

    if let Some(payload) = trimmed.strip_prefix("data:") {
        return Ok(Some(parse_payload(payload)?));
    }

    Ok(None)
}

pub fn parse_chunk(raw: &str) -> Result<ZaiStreamChunk, Error> {
    match parse_payload(raw)? {
        ZaiStreamEvent::Chunk(chunk) => Ok(chunk),
        ZaiStreamEvent::Done => Err(Error::UnexpectedEvent("done")),
        ZaiStreamEvent::Error(_) => Err(Error::UnexpectedEvent("error")),
        ZaiStreamEvent::Open => Err(Error::UnexpectedEvent("open")),
    }
}
