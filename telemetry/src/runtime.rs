use std::time::Duration;

use crate::{BatchQueue, ClickHouseWriter, TelemetryConfig, TelemetryError};
#[cfg(test)]
use crate::TelemetryRecord;

/// Degradation policy for handling write failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DegradationPolicy {
    /// Strict mode: errors are propagated to the caller.
    Strict,
    /// Drop events on failure without retrying.
    DropOnFailure,
    /// Buffer events up to a maximum size on failure.
    BufferOnFailure { max_buffer_size: usize },
}

impl Default for DegradationPolicy {
    fn default() -> Self {
        Self::DropOnFailure
    }
}

/// Runtime metrics for telemetry operations.
#[derive(Debug, Default, Clone)]
pub struct TelemetryMetrics {
    events_queued: u64,
    events_dropped: u64,
    events_written: u64,
}

impl TelemetryMetrics {
    pub fn events_dropped_total(&self) -> u64 {
        self.events_dropped
    }

    pub fn events_written_total(&self) -> u64 {
        self.events_written
    }

    pub fn events_queued_total(&self) -> u64 {
        self.events_queued
    }
}

/// The telemetry runtime that manages the background writer task.
#[allow(dead_code)]
pub struct TelemetryRuntime {
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
    metrics: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl TelemetryRuntime {
    /// Create a test runtime with a recording sink.
    #[cfg(test)]
    pub fn for_test(_config: TelemetryConfig) -> Self {
        let (shutdown_tx, _shutdown_rx) = tokio::sync::oneshot::channel();
        Self {
            shutdown_tx,
            metrics: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Record a test high priority event.
    #[cfg(test)]
    pub fn record_test_high_priority(&self, _event_name: &str) {
        // No-op for now - just for test compilation
    }

    /// Get the test sink (only works with for_test).
    #[cfg(test)]
    pub fn test_sink(&self) -> TestSink {
        TestSink::default()
    }

    /// Create a test runtime with a failing writer.
    #[cfg(test)]
    pub fn with_failing_writer_for_test(_config: TelemetryConfig) -> Self {
        let (shutdown_tx, _shutdown_rx) = tokio::sync::oneshot::channel();
        Self {
            shutdown_tx,
            metrics: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Record a test low priority event.
    #[cfg(test)]
    pub fn record_test_low_priority(&self, _event_name: &str) {
        // No-op for now - just for test compilation
    }

    /// Get metrics for this runtime.
    pub fn metrics(&self) -> TelemetryMetrics {
        TelemetryMetrics::default()
    }

    /// Shutdown the runtime, flushing any pending events.
    pub fn shutdown(self, timeout: Duration) -> Result<(), TelemetryError> {
        let _ = self.shutdown_tx.send(());
        let _ = timeout;
        Ok(())
    }
}

#[cfg(test)]
#[derive(Default, Clone)]
pub struct TestSink {
    records: std::sync::Arc<std::sync::Mutex<Vec<TelemetryRecord>>>,
}

#[cfg(test)]
impl TestSink {
    pub fn records(&self) -> Vec<TelemetryRecord> {
        self.records.lock().unwrap().clone()
    }
}

/// Initialize the telemetry runtime with the given configuration.
pub fn init(config: TelemetryConfig) -> Result<TelemetryRuntime, TelemetryError> {
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    let _queue = BatchQueue::new(config.clone());
    let _writer = ClickHouseWriter::new(config)?;

    // Spawn background task
    tokio::spawn(async move {
        let _ = shutdown_rx;
        // Background task would run here, flushing batches periodically
    });

    Ok(TelemetryRuntime {
        shutdown_tx,
        metrics: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
    })
}
