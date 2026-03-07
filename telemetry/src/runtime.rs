use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc as std_mpsc,
    },
    time::{Duration, Instant},
};

use tracing_subscriber::layer::SubscriberExt;

use crate::{
    BatchEnqueueResult, BatchQueue, ClickHouseWriter, EventPriority, TelemetryConfig,
    TelemetryError, TelemetryLayer, TelemetryRecord, writer::BatchWriter,
};

/// Degradation policy for handling write failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DegradationPolicy {
    /// Strict mode: errors are propagated to the caller.
    Strict,
    /// Drop events on failure without retrying.
    #[default]
    DropOnFailure,
    /// Buffer events up to a maximum size on failure.
    BufferOnFailure { max_buffer_size: usize },
}

#[derive(Debug, Default)]
struct TelemetryMetricsInner {
    events_queued: AtomicU64,
    events_dropped: AtomicU64,
    events_written: AtomicU64,
    write_failures: AtomicU64,
}

/// Runtime metrics for telemetry operations.
#[derive(Debug, Default, Clone)]
pub struct TelemetryMetrics {
    events_queued: u64,
    events_dropped: u64,
    events_written: u64,
    write_failures: u64,
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

    pub fn write_failures_total(&self) -> u64 {
        self.write_failures
    }
}

/// The telemetry runtime that manages the background writer task.
pub struct TelemetryRuntime {
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    shutdown_complete_rx: std_mpsc::Receiver<Result<(), TelemetryError>>,
    metrics: Arc<TelemetryMetricsInner>,
}

impl TelemetryRuntime {
    pub fn metrics(&self) -> TelemetryMetrics {
        TelemetryMetrics {
            events_queued: self.metrics.events_queued.load(Ordering::Relaxed),
            events_dropped: self.metrics.events_dropped.load(Ordering::Relaxed),
            events_written: self.metrics.events_written.load(Ordering::Relaxed),
            write_failures: self.metrics.write_failures.load(Ordering::Relaxed),
        }
    }

    /// Shutdown the runtime, flushing any pending events.
    pub fn shutdown(mut self, timeout: Duration) -> Result<(), TelemetryError> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        match self.shutdown_complete_rx.recv_timeout(timeout) {
            Ok(result) => result,
            Err(std_mpsc::RecvTimeoutError::Timeout) => Err(TelemetryError::Shutdown(
                "timed out waiting for telemetry flush".into(),
            )),
            Err(std_mpsc::RecvTimeoutError::Disconnected) => Err(TelemetryError::Shutdown(
                "telemetry writer task exited without a shutdown result".into(),
            )),
        }
    }
}

struct RuntimeSink {
    queue: Arc<Mutex<BatchQueue>>,
    notify: Arc<tokio::sync::Notify>,
    metrics: Arc<TelemetryMetricsInner>,
}

impl RuntimeSink {
    fn new(
        queue: Arc<Mutex<BatchQueue>>,
        notify: Arc<tokio::sync::Notify>,
        metrics: Arc<TelemetryMetricsInner>,
    ) -> Self {
        Self {
            queue,
            notify,
            metrics,
        }
    }
}

impl crate::TelemetrySink for RuntimeSink {
    fn try_send(&self, record: TelemetryRecord) {
        let is_high_priority = matches!(record.event_priority, EventPriority::High);
        let deadline = is_high_priority.then(|| Instant::now() + Duration::from_millis(250));

        loop {
            let (enqueue_result, should_flush_low) = {
                let mut queue = self.queue.lock().unwrap();
                let enqueue_result = queue.enqueue(record.clone());
                let should_flush_low = queue.should_flush_low();
                (enqueue_result, should_flush_low)
            };

            match enqueue_result {
                BatchEnqueueResult::Queued | BatchEnqueueResult::FlushRequired => {
                    self.metrics.events_queued.fetch_add(1, Ordering::Relaxed);
                    if is_high_priority
                        || matches!(enqueue_result, BatchEnqueueResult::FlushRequired)
                        || should_flush_low
                    {
                        self.notify.notify_one();
                    }
                    return;
                }
                BatchEnqueueResult::DroppedLowPriority => {
                    self.metrics.events_dropped.fetch_add(1, Ordering::Relaxed);
                    return;
                }
                BatchEnqueueResult::DroppedHighPriority => {
                    if let Some(deadline) = deadline
                        && Instant::now() < deadline
                    {
                        std::thread::sleep(Duration::from_millis(10));
                        continue;
                    }

                    self.metrics.events_dropped.fetch_add(1, Ordering::Relaxed);
                    return;
                }
            }
        }
    }
}

pub fn init(config: TelemetryConfig) -> Result<TelemetryRuntime, TelemetryError> {
    let writer = Arc::new(ClickHouseWriter::new(config.clone())?);
    init_with_writer(config, writer)
}

fn init_with_writer(
    config: TelemetryConfig,
    writer: Arc<dyn BatchWriter>,
) -> Result<TelemetryRuntime, TelemetryError> {
    let queue = Arc::new(Mutex::new(BatchQueue::new(config.clone())));
    let notify = Arc::new(tokio::sync::Notify::new());
    let metrics = Arc::new(TelemetryMetricsInner::default());
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let (shutdown_complete_tx, shutdown_complete_rx) = std_mpsc::channel();

    let subscriber = tracing_subscriber::registry().with(TelemetryLayer::new(
        RuntimeSink::new(queue.clone(), notify.clone(), metrics.clone()),
        config.clone(),
    ));
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|err| TelemetryError::Initialization(err.to_string()))?;

    tokio::spawn(writer_task(
        queue,
        notify,
        writer,
        metrics.clone(),
        config,
        shutdown_rx,
        shutdown_complete_tx,
    ));

    Ok(TelemetryRuntime {
        shutdown_tx: Some(shutdown_tx),
        shutdown_complete_rx,
        metrics,
    })
}

async fn writer_task(
    queue: Arc<Mutex<BatchQueue>>,
    notify: Arc<tokio::sync::Notify>,
    writer: Arc<dyn BatchWriter>,
    metrics: Arc<TelemetryMetricsInner>,
    config: TelemetryConfig,
    mut shutdown_rx: tokio::sync::oneshot::Receiver<()>,
    shutdown_complete_tx: std_mpsc::Sender<Result<(), TelemetryError>>,
) {
    let mut high_interval = tokio::time::interval(Duration::from_millis(
        config.high_priority_flush_interval_ms,
    ));
    let mut low_interval =
        tokio::time::interval(Duration::from_millis(config.low_priority_flush_interval_ms));
    high_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    low_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let mut shutdown_result = Ok(());

    loop {
        tokio::select! {
            _ = &mut shutdown_rx => {
                if let Err(err) = flush_ready(&queue, &writer, &metrics, true).await {
                    shutdown_result = Err(err);
                }
                break;
            }
            _ = notify.notified() => {
                if let Err(err) = flush_ready(&queue, &writer, &metrics, false).await {
                    metrics.write_failures.fetch_add(1, Ordering::Relaxed);
                    eprintln!("telemetry write failed: {err}");
                }
            }
            _ = high_interval.tick() => {
                if let Err(err) = flush_priority(&queue, &writer, &metrics, EventPriority::High, true).await {
                    metrics.write_failures.fetch_add(1, Ordering::Relaxed);
                    eprintln!("telemetry high-priority flush failed: {err}");
                }
            }
            _ = low_interval.tick() => {
                if let Err(err) = flush_priority(&queue, &writer, &metrics, EventPriority::Low, true).await {
                    metrics.write_failures.fetch_add(1, Ordering::Relaxed);
                    eprintln!("telemetry low-priority flush failed: {err}");
                }
            }
        }
    }

    let _ = shutdown_complete_tx.send(shutdown_result);
}

async fn flush_ready(
    queue: &Arc<Mutex<BatchQueue>>,
    writer: &Arc<dyn BatchWriter>,
    metrics: &Arc<TelemetryMetricsInner>,
    force: bool,
) -> Result<(), TelemetryError> {
    flush_priority(queue, writer, metrics, EventPriority::High, force).await?;
    flush_priority(queue, writer, metrics, EventPriority::Low, force).await
}

async fn flush_priority(
    queue: &Arc<Mutex<BatchQueue>>,
    writer: &Arc<dyn BatchWriter>,
    metrics: &Arc<TelemetryMetricsInner>,
    priority: EventPriority,
    force: bool,
) -> Result<(), TelemetryError> {
    let batch = {
        let mut queue = queue.lock().unwrap();
        match priority {
            EventPriority::High if force || queue.should_flush_high() || queue.high_len() > 0 => {
                queue.drain_high()
            }
            EventPriority::Low if force || queue.should_flush_low() || queue.low_len() > 0 => {
                queue.drain_low()
            }
            _ => Vec::new(),
        }
    };

    if batch.is_empty() {
        return Ok(());
    }

    let batch_len = batch.len() as u64;
    writer.write_batch(batch).await?;
    metrics
        .events_written
        .fetch_add(batch_len, Ordering::Relaxed);
    Ok(())
}
