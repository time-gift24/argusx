use telemetry::{EventPriority, TelemetryConfig, TelemetryError, TelemetryRecord};

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
    assert!(
        matches!(err, TelemetryError::Validation(message) if message.contains("billing_dedupe_key"))
    );
}

#[test]
fn default_config_matches_design_doc() {
    let config = TelemetryConfig::default();
    assert_eq!(config.high_priority_batch_size, 5);
    assert_eq!(config.low_priority_batch_size, 500);
    assert_eq!(config.high_priority_flush_interval_ms, 1_000);
    assert_eq!(config.low_priority_flush_interval_ms, 30_000);
}

#[test]
fn record_builder_exposes_design_required_columns() {
    let record = TelemetryRecord::builder("turn_finished", EventPriority::High)
        .sequence_no(7)
        .build();

    assert!(record.ingest_id.is_none());
    assert!(record.parent_span_id.is_none());
    assert!(record.step_index.is_none());
    assert_eq!(record.level, "info");
    assert_eq!(record.target, "");
    assert!(record.occurred_at.timestamp_millis() > 0);
    assert_eq!(record.attributes_json, serde_json::json!({}));
}

#[test]
fn sql_schema_matches_design_columns() {
    let schema = include_str!("../../sql/schema.sql");

    for required in [
        "ingest_id UUID",
        "occurred_at DateTime64(3)",
        "parent_span_id Nullable(String)",
        "step_index Nullable(UInt32)",
        "level Enum8('trace'=1, 'debug'=2, 'info'=3, 'warn'=4, 'error'=5)",
        "target LowCardinality(String)",
        "model_name Nullable(LowCardinality(String))",
        "provider Nullable(LowCardinality(String))",
        "tool_name Nullable(LowCardinality(String))",
        "tool_outcome Nullable(LowCardinality(String))",
        "tool_duration_ms Nullable(UInt64)",
        "error_code Nullable(String)",
        "error_message Nullable(String)",
        "request_preview Nullable(String)",
        "response_preview Nullable(String)",
    ] {
        assert!(
            schema.contains(required),
            "schema should contain required column: {required}"
        );
    }
}
