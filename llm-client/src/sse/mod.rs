//! SSE support backed by in-tree `eventsource-stream` and `reqwest-eventsource` inspired implementations.

mod error;
mod event_source;
mod event_stream;
mod message_event;
mod parser;
mod traits;
mod utf8_stream;

pub mod retry;

pub use error::{CannotCloneRequestError, Error};
pub use event_source::{Event, EventSource, ReadyState};
pub use message_event::MessageEvent;

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use futures::stream::{self, StreamExt};

    #[tokio::test]
    async fn parse_multiline_data_event() {
        let input = "data: line1\ndata: line2\n\n";
        let events = crate::sse::traits::Eventsource::eventsource(stream::iter(vec![Ok::<_, ()>(
            Bytes::from(input),
        )]))
        .collect::<Vec<_>>()
        .await;

        assert_eq!(events.len(), 1);
        let event = events[0].as_ref().expect("ok");
        assert_eq!(event.event, "message");
        assert_eq!(event.data, "line1\nline2");
    }

    #[tokio::test]
    async fn parse_stream_supports_error_event_type() {
        let input = "event: error\ndata: upstream failure\n\n";
        let events = crate::sse::traits::Eventsource::eventsource(stream::iter(vec![Ok::<_, ()>(
            Bytes::from(input),
        )]))
        .collect::<Vec<_>>()
        .await;

        assert_eq!(events.len(), 1);
        let event = events[0].as_ref().expect("ok");
        assert_eq!(event.event, "error");
        assert_eq!(event.data, "upstream failure");
    }

    #[tokio::test]
    async fn parse_stream_flushes_trailing_event_without_blank_line() {
        let input = "data: tail event\n";
        let events = crate::sse::traits::Eventsource::eventsource(stream::iter(vec![Ok::<_, ()>(
            Bytes::from(input),
        )]))
        .collect::<Vec<_>>()
        .await;

        assert_eq!(events.len(), 1);
        let event = events[0].as_ref().expect("ok");
        assert_eq!(event.data, "tail event");
    }

    #[tokio::test]
    async fn parse_stream_supports_crlf() {
        let input = "data: hello\r\n\r\ndata: [DONE]\r\n\r\n";
        let events = crate::sse::traits::Eventsource::eventsource(stream::iter(vec![Ok::<_, ()>(
            Bytes::from(input),
        )]))
        .collect::<Vec<_>>()
        .await;

        assert_eq!(events.len(), 2);
        let first = events[0].as_ref().expect("ok");
        assert_eq!(first.data, "hello");
        let second = events[1].as_ref().expect("ok");
        assert_eq!(second.data, "[DONE]");
    }

    #[tokio::test]
    async fn parse_stream_handles_utf8_split_across_chunks() {
        let payload = "data: {\"content\":\"你\"}\n\n".as_bytes().to_vec();
        let split_at = payload
            .iter()
            .position(|byte| *byte == 0xE4)
            .expect("contains multibyte utf8")
            + 1;

        let first = Bytes::copy_from_slice(&payload[..split_at]);
        let second = Bytes::copy_from_slice(&payload[split_at..]);

        let events = crate::sse::traits::Eventsource::eventsource(stream::iter(vec![
            Ok::<_, ()>(first),
            Ok::<_, ()>(second),
        ]))
        .collect::<Vec<_>>()
        .await;

        assert_eq!(events.len(), 1);
        let event = events[0].as_ref().expect("ok");
        assert_eq!(event.data, "{\"content\":\"你\"}");
    }
}
