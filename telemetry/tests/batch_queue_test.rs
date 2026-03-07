use telemetry::{BatchEnqueueResult, BatchQueue, EventPriority, TelemetryConfig, TelemetryRecord};

#[tokio::test]
async fn low_priority_events_are_dropped_when_queue_is_full() {
    let mut config = TelemetryConfig::default();
    config.max_in_memory_events = 1;
    let mut queue = BatchQueue::new(config);

    let first = TelemetryRecord::builder("step_started", EventPriority::Low).build();
    let second = TelemetryRecord::builder("step_finished", EventPriority::Low).build();

    assert!(matches!(queue.enqueue(first), BatchEnqueueResult::Queued));
    assert!(matches!(queue.enqueue(second), BatchEnqueueResult::DroppedLowPriority));
}

#[tokio::test]
async fn high_priority_batch_requests_flush_at_batch_size() {
    let mut config = TelemetryConfig::default();
    config.high_priority_batch_size = 2;
    let mut queue = BatchQueue::new(config);

    assert!(matches!(
        queue.enqueue(TelemetryRecord::builder("turn_finished", EventPriority::High).build()),
        BatchEnqueueResult::Queued
    ));
    assert!(matches!(
        queue.enqueue(TelemetryRecord::builder("llm_response_completed", EventPriority::High).billing_dedupe_key("key-1").build()),
        BatchEnqueueResult::FlushRequired
    ));
}
