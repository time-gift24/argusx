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
    timestamp DateTime64(3),
    trace_id String,
    span_id String,
    parent_span_id Nullable(String),

    -- Event Classification
    level Enum8('trace'=1, 'debug'=2, 'info'=3, 'warn'=4, 'error'=5),
    target String,
    event_name String,

    -- Business Fields
    user_id String,
    session_id String,
    model_name LowCardinality(String),
    provider LowCardinality(String),

    -- Token Usage (Billing)
    input_tokens UInt64,
    output_tokens UInt64,
    total_tokens UInt64,

    -- Tool Statistics
    tool_name LowCardinality(String),
    tool_outcome LowCardinality(String),
    tool_duration_ms UInt64,

    -- Error Info
    error_code Nullable(String),
    error_message Nullable(String),

    -- Sensitive Data (feature gated)
    request_preview Nullable(String),
    response_preview Nullable(String),

    -- Extension
    attributes String
)
ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(timestamp)
ORDER BY (timestamp, trace_id, span_id)
TTL timestamp + INTERVAL 90 DAY;
```

### Materialized Views

```sql
-- Daily Usage Stats
CREATE MATERIALIZED VIEW telemetry_daily_stats
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMM(date)
ORDER BY (date, user_id, model_name)
AS SELECT
    toDate(timestamp) AS date,
    user_id,
    model_name,
    provider,
    count() AS request_count,
    sum(input_tokens) AS total_input_tokens,
    sum(output_tokens) AS total_output_tokens,
    sum(tool_duration_ms) AS total_tool_time_ms
FROM telemetry_logs
WHERE event_name IN ('turn_finished', 'llm_response_completed')
GROUP BY date, user_id, model_name, provider;

-- Tool Usage Stats
CREATE MATERIALIZED VIEW telemetry_tool_stats
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMM(date)
ORDER BY (date, tool_name)
AS SELECT
    toDate(timestamp) AS date,
    tool_name,
    count() AS call_count,
    sum(if(tool_outcome = 'success', 1, 0)) AS success_count,
    sum(if(tool_outcome = 'failed', 1, 0)) AS failed_count,
    avg(tool_duration_ms) AS avg_duration_ms
FROM telemetry_logs
WHERE event_name = 'tool_completed'
GROUP BY date, tool_name;
```

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

## Structured Field Naming

```rust
pub mod telemetry_fields {
    // Identity
    pub const TRACE_ID: &str = "trace_id";
    pub const SESSION_ID: &str = "session_id";
    pub const USER_ID: &str = "user_id";

    // Event
    pub const EVENT: &str = "event";

    // Model
    pub const MODEL: &str = "model";
    pub const PROVIDER: &str = "provider";

    // Tokens
    pub const INPUT_TOKENS: &str = "input_tokens";
    pub const OUTPUT_TOKENS: &str = "output_tokens";
    pub const TOTAL_TOKENS: &str = "total_tokens";

    // Tool
    pub const TOOL_NAME: &str = "tool_name";
    pub const TOOL_OUTCOME: &str = "tool_outcome";
    pub const DURATION_MS: &str = "duration_ms";

    // Error
    pub const ERROR_CODE: &str = "error_code";
    pub const ERROR_MESSAGE: &str = "error_message";
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
```

- `full-logging` ON: Record request/response previews (alpha stage)
- `full-logging` OFF: Only record metrics and metadata (beta/production)

## Priority-Based Reporting

| Priority | Events | Behavior |
|----------|--------|----------|
| High | Errors, billing events (`turn_finished`, `llm_response_completed`) | Send immediately or batch size = 5 |
| Low | Regular info/debug events | Batch send (500 entries or 30s interval) |

## Error Handling & Degradation

### Degradation Policy

```rust
pub enum DegradationPolicy {
    Strict,           // Panic on write failure
    DropOnFailure,    // Drop logs, continue application (default for alpha)
    BufferOnFailure { max_buffer_size: usize },
}
```

### Graceful Shutdown

- Send shutdown signal
- Flush remaining queues (max 10s timeout)
- Report unflushed entries on timeout

## Configuration Defaults

```rust
TelemetryConfig {
    clickhouse_url: "http://localhost:8123",
    user: "default",
    password: "",
    database: "argusx",
    high_priority_batch_size: 5,
    low_priority_batch_size: 500,
    low_priority_flush_interval_ms: 30_000,
    user_id: None,
}
```

## Data Retention

- TTL: 90 days
- Partition: Daily (`toYYYYMMDD(timestamp)`)
- Materialized views: Monthly partition for aggregated stats

## Implementation Scope

1. Create `telemetry` crate
2. Add instrumentation to turn/provider/tool/core modules
3. Create ClickHouse tables and materialized views
4. Add configuration support
5. Integration tests

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
