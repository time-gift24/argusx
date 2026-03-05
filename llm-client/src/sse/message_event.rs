// Adapted from eventsource-stream v0.2.3 (MIT OR Apache-2.0).
// Local modifications:
// - Renamed exported type to `MessageEvent` for llm-client public API.

use core::time::Duration;

/// A parsed SSE message event.
#[derive(Default, Debug, Eq, PartialEq, Clone)]
pub struct MessageEvent {
    /// Event name, defaults to `message` when not provided.
    pub event: String,
    /// Event payload.
    pub data: String,
    /// Event ID.
    pub id: String,
    /// Server-provided reconnection delay.
    pub retry: Option<Duration>,
}
