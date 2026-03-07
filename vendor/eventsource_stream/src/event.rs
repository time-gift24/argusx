#[cfg(not(feature = "std"))]
use alloc::string::String;

use core::time::Duration;

/// An Event
#[derive(Default, Debug, Clone)]
pub struct Event {
    /// The event name if given
    pub event: String,
    /// The event data
    pub data: String,
    /// The event id if given
    pub id: String,
    /// Retry duration if given
    pub retry: Option<Duration>,
    /// The raw SSE frame text that produced this event.
    pub raw: String,
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.event == other.event
            && self.data == other.data
            && self.id == other.id
            && self.retry == other.retry
    }
}

impl Eq for Event {}
