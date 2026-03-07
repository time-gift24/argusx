pub mod batch;
pub mod config;
mod error;
pub mod layer;
pub mod runtime;
pub mod schema;
pub mod sensitive;
pub mod writer;

pub use batch::{BatchEnqueueResult, BatchQueue};
pub use config::TelemetryConfig;
pub use error::TelemetryError;
pub use layer::{RecordingSink, TelemetryLayer, TelemetrySink};
pub use runtime::{DegradationPolicy, TelemetryMetrics, TelemetryRuntime, init};
pub use schema::{EventPriority, TelemetryRecord, TelemetryRecordBuilder};
pub use sensitive::redact_preview;
pub use writer::ClickHouseWriter;
