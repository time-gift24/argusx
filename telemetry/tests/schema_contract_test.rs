use telemetry::{
    EventPriority, TelemetryConfig, TelemetryError, TelemetryRecord,
};

#[test]
fn authoritative_billing_event_requires_dedupe_key() {
    let record = TelemetryRecord::builder("llm_response_completed", EventPriority::High)
        .session_id("s1")
        .turn_id("t1")
        .trace_id("trace-1")
        .span_id("span-1")
        .sequence_no(7)
        .input_tokens(10)
        .output_tokens(20)
        .total_tokens(30)
        .build();

    let err = record.validate().unwrap_err();
    assert!(matches!(err, TelemetryError::Validation(message) if message.contains("billing_dedupe_key")));
}

#[test]
fn default_config_matches_design_doc() {
    let config = TelemetryConfig::default();
    assert_eq!(config.high_priority_batch_size, 5);
    assert_eq!(config.low_priority_batch_size, 500);
    assert_eq!(config.high_priority_flush_interval_ms, 1_000);
    assert_eq!(config.low_priority_flush_interval_ms, 30_000);
}
