use telemetry::{RecordingSink, TelemetryConfig, TelemetryLayer};
use tracing_subscriber::{Registry, layer::SubscriberExt};

#[test]
fn layer_maps_tracing_event_into_telemetry_record() {
    let sink = RecordingSink::default();
    let subscriber = Registry::default().with(TelemetryLayer::new(
        sink.clone(),
        TelemetryConfig::default(),
    ));

    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!(
            "turn_run",
            session_id = "s1",
            turn_id = "t1",
            trace_id = "trace-1",
            span_id = "span-1"
        );
        let _guard = span.enter();
        tracing::info!(
            event_name = "turn_finished",
            sequence_no = 3u32,
            event_priority = "high"
        );
    });

    let records = sink.take();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].event_name, "turn_finished");
    assert_eq!(records[0].turn_id, "t1");
    assert_eq!(records[0].sequence_no, 3);
}
