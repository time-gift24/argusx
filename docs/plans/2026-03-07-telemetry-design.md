# Telemetry System Design

Date: 2026-03-07
Status: Approved

## Overview

Design a comprehensive telemetry system for ArgusX using `tracing` as the unified logging infrastructure, with ClickHouse as the storage backend for log collection and usage analytics.

## Goals

1. **Debug & Diagnostics** - Help developers and users troubleshoot issues
2. **Usage Billing** - Track token consumption for billing purposes
3. **Product Analytics** - Understand feature usage patterns
4. **Security Audit** - Record sensitive operations for compliance

## Non-Goals

1. Replace application metrics/traces already exported through other observability pipelines
2. Build a general-purpose BI platform inside the application runtime
3. Guarantee exactly-once delivery to ClickHouse under process crash or network partition

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Application Layer                         │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐            │
│  │  Turn   │  │Provider │  │  Tool   │  │  Core   │            │
│  │ Driver  │  │ Client  │  │Scheduler│  │  Types  │            │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘            │
│       │            │            │            │                  │
│       └────────────┴────────────┴────────────┘                  │
│                          │                                       │
│                          ▼                                       │
│              ┌───────────────────────┐                          │
│              │   #[instrument]       │  ← Instrumentation Layer  │
│              │   tracing::info!()    │                          │
│              └───────────┬───────────┘                          │
└──────────────────────────┼──────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Telemetry Crate (New)                         │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                TelemetryLayer (tracing::Layer)              ││
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ ││
│  │  │ SpanParser  │  │EventParser  │  │ SensitiveFilter     │││
│  │  └─────────────┘  └─────────────┘  │ (feature gated)     │ ││
│  │                                    └─────────────────────┘ ││
│  └──────────────────────────┬──────────────────────────────────┘│
│                              │                                   │
│  ┌───────────────────────────▼──────────────────────────────────┐│
│  │                   BatchQueue                                 ││
│  │  ┌─────────────┐  ┌─────────────┐                           ││
│  │  │ HighPriority│  │ LowPriority │  ← Priority Queues        ││
│  │  │ (real-time) │  │ (batched)   │                           ││
│  │  └─────────────┘  └─────────────┘                           ││
│  └──────────────────────────┬──────────────────────────────────┘│
│                              │                                   │
│  ┌───────────────────────────▼──────────────────────────────────┐│
│  │                ClickHouseWriter                              ││
│  │  Async batch writes, auto-retry, backpressure               ││
│  └──────────────────────────┬──────────────────────────────────┘│
└──────────────────────────────┼───────────────────────────────────┘
                               │
                               ▼
                    ┌─────────────────────┐
                    │    ClickHouse       │
                    │  (Single Instance)  │
                    └─────────────────────┘
```

## ClickHouse Schema

### Main Log Table

```sql
CREATE TABLE telemetry_logs (
    ingest_id UUID,
    schema_version UInt16,
    occurred_at DateTime64(3),
    ingested_at DateTime64(3) DEFAULT now64(3),

    -- Correlation
    trace_id String,
    span_id String,
    parent_span_id Nullable(String),
    session_id String,
    turn_id String,
    step_index Nullable(UInt32),
    sequence_no UInt32,

    -- Event Classification
    level Enum8('trace'=1, 'debug'=2, 'info'=3, 'warn'=4, 'error'=5),
    target LowCardinality(String),
    event_name LowCardinality(String),
    event_priority Enum8('high'=1, 'low'=2),

    -- Business Fields
    user_id Nullable(String),
    model_name Nullable(LowCardinality(String)),
    provider Nullable(LowCardinality(String)),

    -- Token Usage (Billing)
    input_tokens Nullable(UInt64),
    output_tokens Nullable(UInt64),
    total_tokens Nullable(UInt64),
    billing_dedupe_key Nullable(String),

    -- Tool Statistics
    tool_name Nullable(LowCardinality(String)),
    tool_outcome Nullable(LowCardinality(String)),
    tool_duration_ms Nullable(UInt64),

    -- Error Info
    error_code Nullable(String),
    error_message Nullable(String),

    -- Sensitive Data (feature gated)
    request_preview Nullable(String),
    response_preview Nullable(String),

    -- Extension
    attributes_json String
)
ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(occurred_at)
ORDER BY (session_id, turn_id, occurred_at, sequence_no, trace_id, span_id)
TTL occurred_at + INTERVAL 90 DAY;
```

### Schema Notes

- `ingest_id` is generated by the telemetry writer and is unique per emitted record
- `schema_version` starts at `1` and is incremented only on breaking schema changes
- `sequence_no` is monotonic within a single `turn_id` and is used to recover event order inside one turn
- `billing_dedupe_key` is required for all billing-authoritative events and is stable across retries
- `attributes_json` stores a canonical JSON object for non-indexed fields; reserved fields must not be duplicated inside this blob
- `request_preview` and `response_preview` are capped previews, not full payload storage

### Materialized Views

```sql
-- Daily Token Usage (authoritative for billing analytics)
CREATE MATERIALIZED VIEW telemetry_daily_usage_stats
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMM(date)
ORDER BY (date, user_id, model_name)
AS SELECT
    toDate(occurred_at) AS date,
    user_id,
    model_name,
    provider,
    count() AS completed_response_count,
    sum(input_tokens) AS total_input_tokens,
    sum(output_tokens) AS total_output_tokens,
    sum(total_tokens) AS total_tokens
FROM telemetry_logs
WHERE event_name = 'llm_response_completed'
GROUP BY date, user_id, model_name, provider;

-- Daily Turn Completion Stats
CREATE MATERIALIZED VIEW telemetry_daily_turn_stats
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMM(date)
ORDER BY (date, user_id)
AS SELECT
    toDate(occurred_at) AS date,
    user_id,
    count() AS completed_turn_count
FROM telemetry_logs
WHERE event_name = 'turn_finished'
GROUP BY date, user_id;

-- Tool Usage Stats
CREATE MATERIALIZED VIEW telemetry_tool_stats
ENGINE = AggregatingMergeTree()
PARTITION BY toYYYYMM(date)
ORDER BY (date, tool_name)
AS SELECT
    toDate(occurred_at) AS date,
    tool_name,
    countState() AS call_count_state,
    sumState(if(tool_outcome = 'success', 1, 0)) AS success_count_state,
    sumState(if(tool_outcome = 'failed', 1, 0)) AS failed_count_state,
    avgState(tool_duration_ms) AS avg_duration_ms_state
FROM telemetry_logs
WHERE event_name = 'tool_completed'
GROUP BY date, tool_name;
```

To query `telemetry_tool_stats`, use `countMerge`, `sumMerge`, and `avgMerge` in downstream SQL.

## Instrumentation Points

| Module | Location | Type | Event Names |
|--------|----------|------|-------------|
| turn | `TurnDriver::run()` | `#[instrument]` | `turn_started`, `turn_finished` |
| turn | step loop | `info!` | `step_started`, `step_finished` |
| provider | `ProviderClient::stream()` | `#[instrument]` | `llm_request`, `llm_response_completed` |
| provider | stream response | `debug!` | `llm_delta` |
| tool | `ToolScheduler::execute()` | `#[instrument]` | `tool_started`, `tool_completed` |
| tool | permission request | `info!` | `tool_permission_requested` |
| core | error conversion | `error!` | `error_occurred` |

## Event Contract

### Reserved Event Names

| Event | Purpose | Priority | Billing Authority |
|-------|---------|----------|-------------------|
| `turn_started` | Turn lifecycle start | Low | No |
| `step_started` | Turn internal step start | Low | No |
| `step_finished` | Turn internal step end | Low | No |
| `llm_request` | Provider request issued | Low | No |
| `llm_response_completed` | Provider response completed with final token usage | High | Yes |
| `tool_started` | Tool execution started | Low | No |
| `tool_completed` | Tool execution finished | Low | No |
| `tool_permission_requested` | Permission gate reached | High | No |
| `turn_finished` | Turn lifecycle completed | High | No |
| `error_occurred` | User-visible or operator-actionable error | High | No |

### Required Fields by Event Class

| Event Class | Required Fields |
|------------|-----------------|
| All events | `ingest_id`, `schema_version`, `occurred_at`, `trace_id`, `span_id`, `session_id`, `turn_id`, `event_name`, `level`, `target`, `sequence_no` |
| LLM completion | `provider`, `model_name`, `input_tokens`, `output_tokens`, `total_tokens`, `billing_dedupe_key` |
| Tool completion | `tool_name`, `tool_outcome`, `tool_duration_ms` |
| Error | `error_code`, `error_message` |

### Field Naming Rules

1. Use `event_name` for the canonical event discriminator. `event` must not be used as a competing synonym.
2. Use snake_case for all structured fields and event names.
3. Fields promoted into top-level columns must be removed from `attributes_json`.
4. Event producers may add optional fields only through `attributes_json` unless the schema is revised.

## Context Propagation & Ordering

1. `session_id` is created when a user session begins and is reused across turns.
2. `turn_id` is created at `TurnDriver::run()` entry and must be attached to every nested provider/tool event.
3. `trace_id` and `span_id` come from `tracing` spans; spawned tasks must inherit the current span using `Instrument` or equivalent helpers.
4. `sequence_no` is allocated by the turn driver and increments for every emitted event in the same turn.
5. Cross-thread or async child work that cannot preserve strict ordering must still preserve `turn_id` and `trace_id`; ordering is only guaranteed within a turn, not globally.

## Delivery Semantics

1. Delivery to ClickHouse is at-least-once.
2. Billing and audit consumers must deduplicate by `billing_dedupe_key` or `ingest_id`, never by timestamp alone.
3. Retries reuse the same `billing_dedupe_key` for the same provider completion and create a new `ingest_id` for each physical insert attempt.
4. `llm_delta` is diagnostic-only and is disabled by default in production deployments because of storage amplification.

## Structured Field Naming

```rust
pub mod telemetry_fields {
    // Schema
    pub const SCHEMA_VERSION: &str = "schema_version";

    // Identity
    pub const TRACE_ID: &str = "trace_id";
    pub const SESSION_ID: &str = "session_id";
    pub const USER_ID: &str = "user_id";
    pub const TURN_ID: &str = "turn_id";
    pub const STEP_INDEX: &str = "step_index";
    pub const SEQUENCE_NO: &str = "sequence_no";

    // Event
    pub const EVENT_NAME: &str = "event_name";

    // Model
    pub const MODEL: &str = "model";
    pub const PROVIDER: &str = "provider";

    // Tokens
    pub const INPUT_TOKENS: &str = "input_tokens";
    pub const OUTPUT_TOKENS: &str = "output_tokens";
    pub const TOTAL_TOKENS: &str = "total_tokens";
    pub const BILLING_DEDUPE_KEY: &str = "billing_dedupe_key";

    // Tool
    pub const TOOL_NAME: &str = "tool_name";
    pub const TOOL_OUTCOME: &str = "tool_outcome";
    pub const DURATION_MS: &str = "duration_ms";

    // Error
    pub const ERROR_CODE: &str = "error_code";
    pub const ERROR_MESSAGE: &str = "error_message";

    // Sensitive data
    pub const REQUEST_PREVIEW: &str = "request_preview";
    pub const RESPONSE_PREVIEW: &str = "response_preview";

    // Extension
    pub const ATTRIBUTES_JSON: &str = "attributes_json";
}
```

## Telemetry Crate Structure

```
telemetry/
├── Cargo.toml
├── src/
│   ├── lib.rs           # Public API
│   ├── layer.rs         # TelemetryLayer implementation
│   ├── writer.rs        # ClickHouse writer
│   ├── batch.rs         # Batch queue
│   ├── schema.rs        # Log data structures
│   ├── sensitive.rs     # Sensitive data handling
│   ├── error.rs         # Error types
│   └── config.rs        # Configuration
└── tests/
```

## Feature Flags

```toml
[features]
default = []
full-logging = []  # Enable sensitive data recording
delta-events = []  # Enable high-volume streaming delta diagnostics
```

- `full-logging` ON: Record request/response previews (alpha stage)
- `full-logging` OFF: Only record metrics and metadata (beta/production)
- `delta-events` ON: Record `llm_delta` events for local debugging or short-lived investigations
- `delta-events` OFF: Skip token-by-token streaming diagnostics in production by default

## Priority-Based Reporting

| Priority | Events | Behavior |
|----------|--------|----------|
| High | Errors, permission events, billing-authoritative events (`llm_response_completed`, `turn_finished`, `tool_permission_requested`, `error_occurred`) | Flush every `5` records or `1s`, whichever comes first |
| Low | Regular info/debug events | Batch send (`500` entries or `30s` interval) |

## Backpressure & Retry Policy

1. Use bounded in-memory queues for both priorities.
2. On low-priority queue saturation, drop new low-priority events and increment an internal `telemetry_events_dropped_total` counter.
3. On high-priority queue saturation, block the producer for up to `250ms`; if still full, downgrade to drop-and-count while preserving application progress.
4. Retry ClickHouse writes with exponential backoff capped at `30s` and jitter.
5. Do not retry validation or schema errors; only retry transport and transient server failures.
6. When a batch partially fails, split and retry only the failed slice so one malformed record does not starve the queue.

## Error Handling & Degradation

### Degradation Policy

```rust
pub enum DegradationPolicy {
    Strict,           // Fail initialization or return runtime errors in tests/dev
    DropOnFailure,    // Drop logs, continue application (default for alpha)
    BufferOnFailure { max_buffer_size: usize },
}
```

### Graceful Shutdown

- Send shutdown signal
- Flush remaining queues (max 10s timeout)
- Report unflushed entries on timeout
- Emit one final internal event summarizing dropped, retried, and flushed counts when possible

## Sensitive Data Governance

1. `request_preview` and `response_preview` must be truncated to a fixed byte limit, with UTF-8 safe boundary handling.
2. Secrets, API keys, auth headers, cookies, and provider credentials must always be redacted before preview generation.
3. User prompts and model outputs are opt-in through `full-logging`; production defaults keep them disabled.
4. Sensitive previews are retained for the same 90-day TTL only in alpha/internal environments unless a stricter environment policy overrides it.
5. Access to raw previews should be limited to operators with explicit debugging permissions; billing and analytics consumers should query aggregate views instead.

## Telemetry Self-Observability

The telemetry subsystem must export internal counters/gauges through the existing metrics path:

- `telemetry_batches_written_total`
- `telemetry_write_failures_total`
- `telemetry_retry_attempts_total`
- `telemetry_events_dropped_total`
- `telemetry_queue_depth{priority=...}`
- `telemetry_flush_latency_ms`
- `telemetry_end_to_end_lag_ms`

## Configuration Defaults

```rust
TelemetryConfig {
    clickhouse_url: "http://localhost:8123",
    user: "default",
    password: "",
    database: "argusx",
    high_priority_batch_size: 5,
    low_priority_batch_size: 500,
    high_priority_flush_interval_ms: 1_000,
    low_priority_flush_interval_ms: 30_000,
    max_in_memory_events: 10_000,
    max_retry_backoff_ms: 30_000,
    user_id: None,
}
```

## Data Retention

- TTL: 90 days
- Partition: Daily (`toYYYYMMDD(timestamp)`)
- Materialized views: Monthly partition for aggregated stats
- Aggregated usage views should be retained longer than raw events if finance or product reporting requires it

## Rollout & Compatibility

1. Phase 1: Enable local file/stdout verification with the same event contract before writing to ClickHouse.
2. Phase 2: Enable ClickHouse ingestion for high-priority events only.
3. Phase 3: Add low-priority diagnostic events and optional `delta-events`.
4. Schema changes must be backward compatible within the same `schema_version`; breaking changes require dual-write or migration steps.
5. Dashboards and billing jobs must explicitly pin the schema version they expect.

## Implementation Scope

1. Create `telemetry` crate
2. Add instrumentation to turn/provider/tool/core modules
3. Create ClickHouse tables and materialized views
4. Add configuration support
5. Add internal telemetry metrics
6. Integration tests and failure-injection tests

## Acceptance Criteria

1. A completed turn emits a correlated event set sharing the same `session_id`, `turn_id`, and `trace_id`.
2. A successful provider completion produces exactly one authoritative billing event with a stable `billing_dedupe_key`.
3. ClickHouse unavailability does not crash the application under the default degradation policy.
4. Queue saturation is visible through internal metrics and does not silently drop high-priority events without a counter.
5. `full-logging` disabled means no prompt/response previews are persisted.
6. Materialized views return consistent daily totals without double-counting `turn_finished` and `llm_response_completed`.
7. Shutdown flush completes within `10s` or reports how many events were left behind.

## File Changes

```
telemetry/                          # New crate
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── layer.rs
│   ├── writer.rs
│   ├── batch.rs
│   ├── schema.rs
│   ├── sensitive.rs
│   ├── error.rs
│   └── config.rs

turn/src/driver.rs                  # Add #[instrument]
provider/src/client.rs              # Add #[instrument]
tool/src/scheduler.rs               # Add #[instrument]

Cargo.toml                          # Add telemetry member
config/telemetry.toml               # Telemetry config example
sql/schema.sql                      # ClickHouse schema
```
