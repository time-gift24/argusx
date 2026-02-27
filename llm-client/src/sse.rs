use bytes::Bytes;
use futures::Stream;
use std::time::Duration;
use tokio::time::timeout;
use tokio_stream::StreamExt;

/// SSE event types.
#[derive(Debug, Clone)]
pub enum SseEvent {
    /// Data event with JSON payload.
    Data(String),
    /// Stream completed.
    Done,
    /// Error event.
    Error(String),
}

/// Parse a single SSE line into an event.
pub fn parse_sse_line(line: &str) -> Option<SseEvent> {
    let line = line.trim();

    // Skip empty lines and comments
    if line.is_empty() || line.starts_with(':') {
        return None;
    }

    // Parse data lines - support both "data: " (with space) and "data:" (without space)
    if let Some(data) = line.strip_prefix("data:") {
        // Handle "data:" (no space) or "data: " (with space)
        let data = data.trim_start();
        if data == "[DONE]" {
            return Some(SseEvent::Done);
        }
        if !data.is_empty() {
            return Some(SseEvent::Data(data.to_string()));
        }
        return None;
    }

    // Skip other SSE fields (event:, id:, retry:)
    None
}

/// Parse a byte stream into SSE events.
pub fn parse_sse_stream<S>(
    stream: S,
) -> impl Stream<Item = SseEvent>
where
    S: Stream<Item = Bytes> + Unpin,
{
    use async_stream::stream;

    stream! {
        let mut buffer = String::new();
        let mut lines_stream = std::pin::pin!(stream);

        while let Some(bytes) = lines_stream.next().await {
            if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                buffer.push_str(&text);

                // Process complete lines
                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].to_string();
                    buffer = buffer[pos + 1..].to_string();

                    if let Some(event) = parse_sse_line(&line) {
                        yield event;
                    }
                }
            }
        }
    }
}

/// Wrap a stream with idle timeout detection.
pub fn with_idle_timeout<S>(
    stream: S,
    idle_timeout: Duration,
) -> impl Stream<Item = Result<SseEvent, crate::error::LlmError>>
where
    S: Stream<Item = SseEvent> + Unpin,
{
    use crate::error::LlmError;

    async_stream::try_stream! {
        let mut stream = std::pin::pin!(stream);

        loop {
            match timeout(idle_timeout, stream.next()).await {
                Ok(Some(event)) => yield event,
                Ok(None) => break, // Stream ended
                Err(_) => {
                    yield Err(LlmError::StreamIdleTimeout)?;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream::{self, StreamExt};

    #[tokio::test]
    async fn parse_valid_sse_events() {
        let input = "data: {\"text\":\"hello\"}\n\ndata: [DONE]\n\n";
        let bytes_vec: Vec<Bytes> = vec![Bytes::from(input)];
        let bytes_stream = stream::iter(bytes_vec);

        let events: Vec<_> = parse_sse_stream(bytes_stream)
            .collect()
            .await;

        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], SseEvent::Data(ref s) if s == "{\"text\":\"hello\"}"));
        assert!(matches!(events[1], SseEvent::Done));
    }

    #[test]
    fn parse_single_line_event() {
        let line = "data: {\"content\":\"test\"}";
        let event = parse_sse_line(line);
        assert!(matches!(event, Some(SseEvent::Data(ref s)) if s == "{\"content\":\"test\"}"));
    }

    #[test]
    fn parse_done_event() {
        let line = "data: [DONE]";
        let event = parse_sse_line(line);
        assert!(matches!(event, Some(SseEvent::Done)));
    }

    #[test]
    fn ignore_non_data_lines() {
        assert!(parse_sse_line(": comment").is_none());
        assert!(parse_sse_line("").is_none());
        assert!(parse_sse_line("event: foo").is_none());
    }

    #[test]
    fn parse_data_without_space() {
        // Test "data:" without space
        let line = "data:{\"content\":\"test\"}";
        let event = parse_sse_line(line);
        assert!(matches!(event, Some(SseEvent::Data(ref s)) if s == "{\"content\":\"test\"}"));
    }

    #[test]
    fn parse_data_without_space_done() {
        // Test "data:[DONE]" without space
        let line = "data:[DONE]";
        let event = parse_sse_line(line);
        assert!(matches!(event, Some(SseEvent::Done)));
    }
}
