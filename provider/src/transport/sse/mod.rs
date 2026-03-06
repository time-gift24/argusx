mod error;
mod event_source;

pub use error::Error;
pub use event_source::{Event, EventSource};
pub use eventsource_stream::Event as MessageEvent;
