use crate::error::LlmError;
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
    let line = line.trim_end_matches('\r').trim();

    // Skip empty lines and comments
    if line.is_empty() || line.starts_with(':') {
        return None;
    }

    if let Some(data) = parse_data_field(line) {
        if data == "[DONE]" {
            Some(SseEvent::Done)
        } else {
            Some(SseEvent::Data(data))
        }
    } else {
        None
    }
}

/// Parse a byte stream into SSE events.
pub fn parse_sse_stream<S>(stream: S) -> impl Stream<Item = SseEvent>
where
    S: Stream<Item = Bytes> + Unpin,
{
    use async_stream::stream;

    stream! {
        let mut buffer = String::new();
        let mut event_name: Option<String> = None;
        let mut event_data: Vec<String> = Vec::new();
        let mut lines_stream = std::pin::pin!(stream);

        while let Some(bytes) = lines_stream.next().await {
            if let Ok(text) = std::str::from_utf8(&bytes) {
                buffer.push_str(text);
                while let Some(line) = pop_next_line(&mut buffer) {
                    if line.is_empty() {
                        if let Some(event) = build_event(event_name.take(), &mut event_data) {
                            yield event;
                        }
                        continue;
                    }

                    if line.starts_with(':') {
                        continue;
                    }

                    if let Some(name) = line.strip_prefix("event:") {
                        event_name = Some(name.trim_start().to_string());
                        continue;
                    }

                    if let Some(data) = parse_data_field(&line) {
                        event_data.push(data);
                    }
                }
            }
        }

        // Flush trailing event payload if connection closes without final blank line.
        if let Some(event) = build_event(event_name.take(), &mut event_data) {
            yield event;
        }
    }
}

/// Parse an SSE byte stream that can fail while reading from transport.
pub fn parse_sse_stream_result<S, E>(stream: S) -> impl Stream<Item = Result<SseEvent, LlmError>>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
    E: Into<LlmError>,
{
    async_stream::try_stream! {
        let mut buffer = String::new();
        let mut event_name: Option<String> = None;
        let mut event_data: Vec<String> = Vec::new();
        let mut lines_stream = std::pin::pin!(stream);

        while let Some(item) = lines_stream.next().await {
            let bytes = item.map_err(Into::into)?;
            let text = std::str::from_utf8(&bytes).map_err(|err| LlmError::ParseError {
                message: format!("invalid UTF-8 in SSE stream: {}", err),
            })?;
            buffer.push_str(text);

            while let Some(line) = pop_next_line(&mut buffer) {
                if line.is_empty() {
                    if let Some(event) = build_event(event_name.take(), &mut event_data) {
                        yield event;
                    }
                    continue;
                }

                if line.starts_with(':') {
                    continue;
                }

                if let Some(name) = line.strip_prefix("event:") {
                    event_name = Some(name.trim_start().to_string());
                    continue;
                }

                if let Some(data) = parse_data_field(&line) {
                    event_data.push(data);
                }
            }
        }

        if let Some(event) = build_event(event_name.take(), &mut event_data) {
            yield event;
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
    async_stream::try_stream! {
        let mut stream = std::pin::pin!(stream);

        loop {
            match timeout(idle_timeout, stream.next()).await {
                Ok(Some(event)) => yield event,
                Ok(None) => break, // Stream ended
                Err(_) => {
                    Err(LlmError::StreamIdleTimeout)?;
                }
            }
        }
    }
}

fn parse_data_field(line: &str) -> Option<String> {
    let data = line.strip_prefix("data:")?;
    if let Some(without_space) = data.strip_prefix(' ') {
        Some(without_space.to_string())
    } else {
        Some(data.to_string())
    }
}

fn pop_next_line(buffer: &mut String) -> Option<String> {
    let pos = buffer.find('\n')?;
    let mut line = buffer[..pos].to_string();
    if line.ends_with('\r') {
        line.pop();
    }
    *buffer = buffer[pos + 1..].to_string();
    Some(line)
}

fn build_event(event_name: Option<String>, data_lines: &mut Vec<String>) -> Option<SseEvent> {
    if data_lines.is_empty() {
        return None;
    }

    let data = data_lines.join("\n");
    data_lines.clear();

    if data == "[DONE]" {
        return Some(SseEvent::Done);
    }

    if event_name.as_deref() == Some("error") {
        return Some(SseEvent::Error(data));
    }

    Some(SseEvent::Data(data))
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

        let events: Vec<_> = parse_sse_stream(bytes_stream).collect().await;

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

    #[tokio::test]
    async fn parse_multiline_data_event() {
        let input = "data: line1\ndata: line2\n\n";
        let bytes_stream = stream::iter(vec![Bytes::from(input)]);

        let events: Vec<_> = parse_sse_stream(bytes_stream).collect().await;
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], SseEvent::Data(ref s) if s == "line1\nline2"));
    }

    #[tokio::test]
    async fn parse_error_event() {
        let input = "event: error\ndata: upstream failure\n\n";
        let bytes_stream = stream::iter(vec![Bytes::from(input)]);

        let events: Vec<_> = parse_sse_stream(bytes_stream).collect().await;
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], SseEvent::Error(ref s) if s == "upstream failure"));
    }

    #[tokio::test]
    async fn parse_stream_flushes_trailing_event_without_blank_line() {
        let input = "data: tail event\n";
        let bytes_stream = stream::iter(vec![Bytes::from(input)]);

        let events: Vec<_> = parse_sse_stream(bytes_stream).collect().await;
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], SseEvent::Data(ref s) if s == "tail event"));
    }

    #[tokio::test]
    async fn parse_stream_supports_crlf() {
        let input = "data: hello\r\n\r\ndata: [DONE]\r\n\r\n";
        let bytes_stream = stream::iter(vec![Bytes::from(input)]);

        let events: Vec<_> = parse_sse_stream(bytes_stream).collect().await;
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], SseEvent::Data(ref s) if s == "hello"));
        assert!(matches!(events[1], SseEvent::Done));
    }

    #[tokio::test]
    async fn parse_result_stream_propagates_transport_error() {
        let events = parse_sse_stream_result(stream::iter(vec![
            Ok::<Bytes, LlmError>(Bytes::from("data: hello\n\n")),
            Err(LlmError::NetworkError {
                message: "socket closed".to_string(),
            }),
        ]))
        .collect::<Vec<_>>()
        .await;

        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], Ok(SseEvent::Data(ref s)) if s == "hello"));
        assert!(matches!(events[1], Err(LlmError::NetworkError { .. })));
    }
}
