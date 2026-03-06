//! SSE APIs re-exported from the vendored `eventsource_stream` crate.

pub use eventsource_stream::retry;
pub use eventsource_stream::{
    CannotCloneRequestError, Error, Event, EventSource, MessageEvent, ReadyState,
};
