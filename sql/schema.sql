-- Telemetry Logs Schema for ClickHouse
-- Version: 1

CREATE TABLE IF NOT EXISTS argusx.telemetry_logs
(
    schema_version UInt16,
    event_name LowCardinality(String),
    event_priority Enum8('high' = 1, 'low' = 2),
    session_id String,
    turn_id String,
    trace_id String,
    span_id String,
    sequence_no UInt32,
    input_tokens Nullable(UInt64),
    output_tokens Nullable(UInt64),
    total_tokens Nullable(UInt64),
    billing_dedupe_key Nullable(String),
    attributes_json String,
    created_at DateTime64(3) DEFAULT now64(3)
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(created_at)
ORDER BY (session_id, turn_id, sequence_no, created_at)
SETTINGS index_granularity = 8192;
