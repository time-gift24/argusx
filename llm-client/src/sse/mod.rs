//! SSE support built on the vendored `eventsource-stream` parser.

mod error;
mod event_source;

pub mod retry;

pub use error::{CannotCloneRequestError, Error};
pub use event_source::{Event, EventSource, ReadyState};
pub use eventsource_stream::Event as MessageEvent;
