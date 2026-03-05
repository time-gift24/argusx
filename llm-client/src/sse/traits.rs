// Adapted from eventsource-stream v0.2.3 (MIT OR Apache-2.0).

use crate::sse::event_stream::EventStream;
use futures::Stream;

/// Extension trait for turning a bytes stream into an SSE event stream.
pub trait Eventsource: Sized {
    fn eventsource(self) -> EventStream<Self>;
}

impl<S, B, E> Eventsource for S
where
    S: Stream<Item = Result<B, E>>,
    B: AsRef<[u8]>,
{
    fn eventsource(self) -> EventStream<Self> {
        EventStream::new(self)
    }
}
