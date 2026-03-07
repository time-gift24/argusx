-- Telemetry Logs Schema for ClickHouse
-- Version: 1

CREATE TABLE IF NOT EXISTS argusx.telemetry_logs
(
    ingest_id UUID,
    schema_version UInt16,
    occurred_at DateTime64(3),
    ingested_at DateTime64(3) DEFAULT now64(3),

    trace_id String,
    span_id String,
    parent_span_id Nullable(String),
    session_id String,
    turn_id String,
    step_index Nullable(UInt32),
    sequence_no UInt32,

    level Enum8('trace'=1, 'debug'=2, 'info'=3, 'warn'=4, 'error'=5),
    target LowCardinality(String),
    event_name LowCardinality(String),
    event_priority Enum8('high' = 1, 'low' = 2),

    user_id Nullable(String),
    model_name Nullable(LowCardinality(String)),
    provider Nullable(LowCardinality(String)),

    input_tokens Nullable(UInt64),
    output_tokens Nullable(UInt64),
    total_tokens Nullable(UInt64),
    billing_dedupe_key Nullable(String),

    tool_name Nullable(LowCardinality(String)),
    tool_outcome Nullable(LowCardinality(String)),
    tool_duration_ms Nullable(UInt64),

    error_code Nullable(String),
    error_message Nullable(String),

    request_preview Nullable(String),
    response_preview Nullable(String),

    attributes_json String
)
ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(occurred_at)
ORDER BY (session_id, turn_id, occurred_at, sequence_no, trace_id, span_id)
TTL occurred_at + INTERVAL 90 DAY;
