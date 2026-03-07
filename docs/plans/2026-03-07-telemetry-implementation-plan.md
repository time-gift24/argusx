# Telemetry System Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a production-safe telemetry pipeline that captures turn, provider, and tool events through `tracing`, batches them, and writes them to ClickHouse without breaking the desktop runtime.

**Architecture:** Add a new `telemetry` crate that owns the event schema, batching, writer, filtering, and `tracing_subscriber::Layer`. Existing crates (`provider`, `tool`, `turn`) only emit structured tracing events; the desktop runtime initializes the subscriber once and shuts it down gracefully. Keep `core` unchanged in v1 and emit `error_occurred` at crate boundaries instead of pushing telemetry concerns into shared domain types.

**Tech Stack:** Rust 2024, `tracing`, `tracing-subscriber`, `tokio`, `reqwest`, `serde`, `serde_json`, `uuid`, ClickHouse HTTP insert API, `wiremock`

---

## Repo Notes

- Workspace members today: `core`, `desktop/src-tauri`, `provider`, `tool`, `turn`, `vendor/eventsource_stream`
- There is no existing global tracing initialization; `desktop/src-tauri/src/lib.rs` is the runtime entry point
- Existing integration test style already lives in `provider/tests`, `tool/tests`, and `turn/tests`
- `config/` and `sql/` do not exist yet; create them in this work
- Do not change `core/src/lib.rs` in v1 unless implementation proves it is strictly necessary

### Task 1: Scaffold the `telemetry` crate and workspace wiring

**Files:**
- Create: `telemetry/Cargo.toml`
- Create: `telemetry/src/lib.rs`
- Create: `telemetry/src/error.rs`
- Create: `telemetry/tests/compile_smoke_test.rs`
- Modify: `Cargo.toml`

**Step 1: Write the failing smoke test**

```rust
use telemetry::{TelemetryConfig, TelemetryError};

#[test]
fn telemetry_crate_exports_config_and_error_types() {
    let config = TelemetryConfig::default();
    assert_eq!(config.high_priority_batch_size, 5);

    let err = TelemetryError::Validation("boom".into());
    assert!(err.to_string().contains("boom"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p telemetry --test compile_smoke_test telemetry_crate_exports_config_and_error_types -- --exact`
Expected: FAIL with `package ID specification 'telemetry' did not match any packages` or unresolved imports.

**Step 3: Write minimal implementation**

```toml
# Cargo.toml
[workspace]
members = ["core", "desktop/src-tauri", "provider", "tool", "turn", "telemetry", "vendor/eventsource_stream"]
```

```toml
# telemetry/Cargo.toml
[package]
name = "telemetry"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true, features = ["sync", "time", "rt", "macros"] }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
reqwest = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["rt-multi-thread", "macros", "time"] }
wiremock = { workspace = true }
```

```rust
// telemetry/src/lib.rs
mod error;
pub mod config;

pub use config::TelemetryConfig;
pub use error::TelemetryError;
```

```rust
// telemetry/src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("telemetry validation error: {0}")]
    Validation(String),
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p telemetry --test compile_smoke_test telemetry_crate_exports_config_and_error_types -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add Cargo.toml telemetry/Cargo.toml telemetry/src/lib.rs telemetry/src/error.rs telemetry/tests/compile_smoke_test.rs
git commit -m "feat: scaffold telemetry crate"
```

### Task 2: Define config and schema contracts first

**Files:**
- Create: `telemetry/src/config.rs`
- Create: `telemetry/src/schema.rs`
- Create: `telemetry/tests/schema_contract_test.rs`
- Modify: `telemetry/src/lib.rs`
- Modify: `telemetry/src/error.rs`

**Step 1: Write the failing contract tests**

```rust
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
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p telemetry --test schema_contract_test -q`
Expected: FAIL with missing `TelemetryRecord`, `EventPriority`, or validation behavior.

**Step 3: Write minimal implementation**

```rust
// telemetry/src/config.rs
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub clickhouse_url: String,
    pub database: String,
    pub table: String,
    pub high_priority_batch_size: usize,
    pub low_priority_batch_size: usize,
    pub high_priority_flush_interval_ms: u64,
    pub low_priority_flush_interval_ms: u64,
    pub max_in_memory_events: usize,
    pub max_retry_backoff_ms: u64,
    pub full_logging: bool,
    pub delta_events: bool,
}
```

```rust
// telemetry/src/schema.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPriority {
    High,
    Low,
}

#[derive(Debug, Clone)]
pub struct TelemetryRecord {
    pub schema_version: u16,
    pub event_name: String,
    pub event_priority: EventPriority,
    pub session_id: String,
    pub turn_id: String,
    pub trace_id: String,
    pub span_id: String,
    pub sequence_no: u32,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub billing_dedupe_key: Option<String>,
    pub attributes_json: serde_json::Value,
}

impl TelemetryRecord {
    pub fn validate(&self) -> Result<(), TelemetryError> {
        if self.event_name == "llm_response_completed" && self.billing_dedupe_key.is_none() {
            return Err(TelemetryError::Validation(
                "billing_dedupe_key is required for llm_response_completed".into(),
            ));
        }
        Ok(())
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p telemetry --test schema_contract_test -q`
Expected: PASS

**Step 5: Commit**

```bash
git add telemetry/src/config.rs telemetry/src/schema.rs telemetry/src/lib.rs telemetry/src/error.rs telemetry/tests/schema_contract_test.rs
git commit -m "feat: define telemetry schema contract"
```

### Task 3: Implement bounded priority batching before the writer

**Files:**
- Create: `telemetry/src/batch.rs`
- Create: `telemetry/tests/batch_queue_test.rs`
- Modify: `telemetry/src/lib.rs`
- Modify: `telemetry/src/config.rs`

**Step 1: Write the failing queue tests**

```rust
use telemetry::{BatchEnqueueResult, BatchQueue, EventPriority, TelemetryConfig, TelemetryRecord};

#[tokio::test]
async fn low_priority_events_are_dropped_when_queue_is_full() {
    let mut config = TelemetryConfig::default();
    config.max_in_memory_events = 1;
    let mut queue = BatchQueue::new(config);

    let first = TelemetryRecord::test("step_started", EventPriority::Low);
    let second = TelemetryRecord::test("step_finished", EventPriority::Low);

    assert!(matches!(queue.enqueue(first), BatchEnqueueResult::Queued));
    assert!(matches!(queue.enqueue(second), BatchEnqueueResult::DroppedLowPriority));
}

#[tokio::test]
async fn high_priority_batch_requests_flush_at_batch_size() {
    let mut config = TelemetryConfig::default();
    config.high_priority_batch_size = 2;
    let mut queue = BatchQueue::new(config);

    assert!(matches!(
        queue.enqueue(TelemetryRecord::test("turn_finished", EventPriority::High)),
        BatchEnqueueResult::Queued
    ));
    assert!(matches!(
        queue.enqueue(TelemetryRecord::test("llm_response_completed", EventPriority::High)),
        BatchEnqueueResult::FlushRequired
    ));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p telemetry --test batch_queue_test -q`
Expected: FAIL with missing queue types.

**Step 3: Write minimal implementation**

```rust
// telemetry/src/batch.rs
pub enum BatchEnqueueResult {
    Queued,
    FlushRequired,
    DroppedLowPriority,
}

pub struct BatchQueue {
    config: TelemetryConfig,
    high: Vec<TelemetryRecord>,
    low: Vec<TelemetryRecord>,
}

impl BatchQueue {
    pub fn enqueue(&mut self, record: TelemetryRecord) -> BatchEnqueueResult {
        // keep queues bounded and make drop semantics explicit
    }

    pub fn drain_high(&mut self) -> Vec<TelemetryRecord> {
        std::mem::take(&mut self.high)
    }

    pub fn drain_low(&mut self) -> Vec<TelemetryRecord> {
        std::mem::take(&mut self.low)
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p telemetry --test batch_queue_test -q`
Expected: PASS

**Step 5: Commit**

```bash
git add telemetry/src/batch.rs telemetry/src/lib.rs telemetry/src/config.rs telemetry/tests/batch_queue_test.rs
git commit -m "feat: add telemetry priority batching"
```

### Task 4: Implement the ClickHouse HTTP writer with retry boundaries

**Files:**
- Create: `telemetry/src/writer.rs`
- Create: `telemetry/tests/clickhouse_writer_test.rs`
- Modify: `telemetry/src/error.rs`
- Modify: `telemetry/src/lib.rs`

**Step 1: Write the failing writer tests**

```rust
use telemetry::{ClickHouseWriter, EventPriority, TelemetryConfig, TelemetryRecord};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn writer_posts_json_each_row_to_clickhouse() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let mut config = TelemetryConfig::default();
    config.clickhouse_url = server.uri();
    let writer = ClickHouseWriter::new(config).unwrap();

    writer.write_batch(vec![TelemetryRecord::test("turn_finished", EventPriority::High)]).await.unwrap();
}

#[tokio::test]
async fn writer_does_not_retry_validation_errors() {
    let writer = ClickHouseWriter::new(TelemetryConfig::default()).unwrap();
    let mut invalid = TelemetryRecord::test("llm_response_completed", EventPriority::High);
    invalid.billing_dedupe_key = None;

    let err = writer.write_batch(vec![invalid]).await.unwrap_err();
    assert!(err.to_string().contains("billing_dedupe_key"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p telemetry --test clickhouse_writer_test -q`
Expected: FAIL with missing `ClickHouseWriter`.

**Step 3: Write minimal implementation**

```rust
// telemetry/src/writer.rs
pub struct ClickHouseWriter {
    client: reqwest::Client,
    config: TelemetryConfig,
}

impl ClickHouseWriter {
    pub fn new(config: TelemetryConfig) -> Result<Self, TelemetryError> {
        Ok(Self {
            client: reqwest::Client::new(),
            config,
        })
    }

    pub async fn write_batch(&self, records: Vec<TelemetryRecord>) -> Result<(), TelemetryError> {
        for record in &records {
            record.validate()?;
        }

        let body = records
            .into_iter()
            .map(|record| serde_json::to_string(&record))
            .collect::<Result<Vec<_>, _>>()?
            .join("\n");

        let query = format!(
            "INSERT INTO {}.{} FORMAT JSONEachRow",
            self.config.database, self.config.table
        );

        self.client
            .post(&self.config.clickhouse_url)
            .query(&[("query", query)])
            .body(body)
            .send()
            .await?;

        Ok(())
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p telemetry --test clickhouse_writer_test -q`
Expected: PASS

**Step 5: Commit**

```bash
git add telemetry/src/writer.rs telemetry/src/error.rs telemetry/src/lib.rs telemetry/tests/clickhouse_writer_test.rs
git commit -m "feat: add clickhouse telemetry writer"
```

### Task 5: Build the sensitive filter and `tracing` layer mapping

**Files:**
- Create: `telemetry/src/sensitive.rs`
- Create: `telemetry/src/layer.rs`
- Create: `telemetry/tests/sensitive_filter_test.rs`
- Create: `telemetry/tests/layer_event_mapping_test.rs`
- Modify: `telemetry/src/lib.rs`

**Step 1: Write the failing layer tests**

```rust
use telemetry::{RecordingSink, TelemetryConfig, TelemetryLayer};
use tracing_subscriber::{layer::SubscriberExt, Registry};

#[test]
fn sensitive_filter_redacts_auth_headers() {
    let preview = telemetry::redact_preview(r#"{"authorization":"Bearer secret","prompt":"hello"}"#, 256);
    assert!(preview.contains("[REDACTED]"));
    assert!(!preview.contains("secret"));
}

#[test]
fn layer_maps_tracing_event_into_telemetry_record() {
    let sink = RecordingSink::default();
    let subscriber = Registry::default().with(TelemetryLayer::new(sink.clone(), TelemetryConfig::default()));

    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!("turn_run", session_id = "s1", turn_id = "t1", trace_id = "trace-1", span_id = "span-1");
        let _guard = span.enter();
        tracing::info!(event_name = "turn_finished", sequence_no = 3u32, event_priority = "high");
    });

    let records = sink.take();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].event_name, "turn_finished");
    assert_eq!(records[0].turn_id, "t1");
    assert_eq!(records[0].sequence_no, 3);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p telemetry --test sensitive_filter_test --test layer_event_mapping_test -q`
Expected: FAIL with missing layer/filter types.

**Step 3: Write minimal implementation**

```rust
// telemetry/src/sensitive.rs
pub fn redact_preview(raw: &str, limit: usize) -> String {
    let truncated = raw.chars().take(limit).collect::<String>();
    truncated
        .replace("authorization", "[REDACTED_KEY]")
        .replace("Bearer ", "[REDACTED] ")
}
```

```rust
// telemetry/src/layer.rs
pub trait TelemetrySink: Send + Sync + 'static {
    fn try_send(&self, record: TelemetryRecord);
}

pub struct TelemetryLayer<S> {
    sink: S,
    config: TelemetryConfig,
}

impl<S> TelemetryLayer<S> {
    pub fn new(sink: S, config: TelemetryConfig) -> Self {
        Self { sink, config }
    }
}

impl<S, T> tracing_subscriber::Layer<T> for TelemetryLayer<S>
where
    S: TelemetrySink,
    T: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, T>) {
        // visit fields, map span metadata, redact previews, and forward to sink
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p telemetry --test sensitive_filter_test --test layer_event_mapping_test -q`
Expected: PASS

**Step 5: Commit**

```bash
git add telemetry/src/sensitive.rs telemetry/src/layer.rs telemetry/src/lib.rs telemetry/tests/sensitive_filter_test.rs telemetry/tests/layer_event_mapping_test.rs
git commit -m "feat: add telemetry tracing layer"
```

### Task 6: Add runtime facade, desktop initialization, example config, and ClickHouse schema

**Files:**
- Modify: `desktop/src-tauri/Cargo.toml`
- Modify: `desktop/src-tauri/src/lib.rs`
- Modify: `telemetry/src/lib.rs`
- Create: `config/telemetry.toml`
- Create: `sql/schema.sql`
- Create: `telemetry/tests/config_parse_test.rs`

**Step 1: Write the failing config/runtime tests**

```rust
use telemetry::TelemetryConfig;

#[test]
fn example_config_parses_into_runtime_config() {
    let raw = include_str!("../../config/telemetry.toml");
    let config: TelemetryConfig = toml::from_str(raw).unwrap();
    assert_eq!(config.database, "argusx");
    assert_eq!(config.table, "telemetry_logs");
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p telemetry --test config_parse_test -q`
Expected: FAIL because config file or `Deserialize` support is missing.

**Step 3: Write minimal implementation**

```toml
# desktop/src-tauri/Cargo.toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
telemetry = { path = "../../telemetry" }
```

```rust
// telemetry/src/lib.rs
pub struct TelemetryRuntime {
    shutdown: tokio::sync::oneshot::Sender<()>,
}

pub fn init(config: TelemetryConfig) -> Result<TelemetryRuntime, TelemetryError> {
    // build queue + writer task, register subscriber layer, return shutdown handle
}
```

```rust
// desktop/src-tauri/src/lib.rs
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = telemetry::TelemetryConfig::from_path("config/telemetry.toml")
        .unwrap_or_else(|_| telemetry::TelemetryConfig::default());
    let runtime = telemetry::init(config)?;

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())?;

    runtime.shutdown(std::time::Duration::from_secs(10))?;
    Ok(())
}
```

```toml
# config/telemetry.toml
enabled = true
clickhouse_url = "http://localhost:8123"
database = "argusx"
table = "telemetry_logs"
high_priority_batch_size = 5
low_priority_batch_size = 500
high_priority_flush_interval_ms = 1000
low_priority_flush_interval_ms = 30000
max_in_memory_events = 10000
max_retry_backoff_ms = 30000
full_logging = false
delta_events = false
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p telemetry --test config_parse_test -q`
Expected: PASS

**Step 5: Commit**

```bash
git add desktop/src-tauri/Cargo.toml desktop/src-tauri/src/lib.rs telemetry/src/lib.rs config/telemetry.toml sql/schema.sql telemetry/tests/config_parse_test.rs
git commit -m "feat: initialize telemetry runtime in desktop app"
```

### Task 7: Instrument provider request and completion events

**Files:**
- Modify: `provider/Cargo.toml`
- Modify: `provider/src/client.rs`
- Create: `provider/tests/provider_telemetry_test.rs`

**Step 1: Write the failing provider telemetry test**

```rust
use provider::{Dialect, ProviderClient, ProviderConfig, Request};
use telemetry::{RecordingSink, TelemetryConfig, TelemetryLayer};
use tracing_subscriber::{layer::SubscriberExt, Registry};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn provider_emits_request_and_completion_events() {
    let server = MockServer::start().await;
    let body = concat!(
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",",
        "\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hi\"}}]}\n\n",
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",",
        "\"choices\":[{\"index\":0,\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":5,\"completion_tokens\":7,\"total_tokens\":12}}\n\n",
        "data: [DONE]\n\n"
    );

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
        .mount(&server)
        .await;

    let sink = RecordingSink::default();
    let subscriber = Registry::default().with(TelemetryLayer::new(sink.clone(), TelemetryConfig::default()));

    tracing::subscriber::with_default(subscriber, || async {
        let client = ProviderClient::new(ProviderConfig::new(Dialect::Openai, server.uri(), "test-key")).unwrap();
        let stream = client.stream(Request::default()).unwrap();
        let _: Vec<_> = futures::StreamExt::collect(stream).await;
    }).await;

    let records = sink.take();
    assert!(records.iter().any(|record| record.event_name == "llm_request"));
    assert!(records.iter().any(|record| record.event_name == "llm_response_completed" && record.total_tokens == Some(12)));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p provider --test provider_telemetry_test provider_emits_request_and_completion_events -- --exact`
Expected: FAIL because provider emits no tracing telemetry events yet.

**Step 3: Write minimal implementation**

```rust
// provider/src/client.rs
#[tracing::instrument(
    name = "provider.stream",
    skip(self, request),
    fields(
        event_name = "llm_request",
        provider = ?self.config.dialect,
        model_name = tracing::field::Empty
    )
)]
pub fn stream(&self, request: Request) -> Result<ResponseStream, Error> {
    // keep request normalization
}

// inside spawned producer future
let producer = tokio::spawn(async move {
    tracing::info!(event_name = "llm_request");
    // when final usage is known:
    tracing::info!(
        event_name = "llm_response_completed",
        event_priority = "high",
        input_tokens = usage.input_tokens,
        output_tokens = usage.output_tokens,
        total_tokens = usage.total_tokens,
        billing_dedupe_key = billing_key
    );
}.instrument(tracing::Span::current()));
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p provider --test provider_telemetry_test provider_emits_request_and_completion_events -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add provider/Cargo.toml provider/src/client.rs provider/tests/provider_telemetry_test.rs
git commit -m "feat: instrument provider telemetry events"
```

### Task 8: Instrument tool execution lifecycle

**Files:**
- Modify: `tool/Cargo.toml`
- Modify: `tool/src/scheduler.rs`
- Create: `tool/tests/telemetry_scheduler_test.rs`

**Step 1: Write the failing tool telemetry test**

```rust
use std::sync::Arc;
use argus_core::{Builtin, BuiltinToolCall};
use tokio_util::sync::CancellationToken;
use telemetry::{RecordingSink, TelemetryConfig, TelemetryLayer};
use tool::{scheduler::{BuiltinRegistration, EffectiveToolPolicy, ToolScheduler}, ToolContext};
use tracing_subscriber::{layer::SubscriberExt, Registry};

#[tokio::test]
async fn scheduler_emits_tool_started_and_completed() {
    let sink = RecordingSink::default();
    let subscriber = Registry::default().with(TelemetryLayer::new(sink.clone(), TelemetryConfig::default()));

    tracing::subscriber::with_default(subscriber, || async {
        let scheduler = ToolScheduler::new([BuiltinRegistration::new(
            Builtin::Read,
            Arc::new(super::SlowTool::default()),
            EffectiveToolPolicy { allow_parallel: true, max_concurrency: 1 },
        )]).unwrap();

        let _ = scheduler.execute_builtin(
            BuiltinToolCall {
                sequence: 0,
                call_id: "call-1".into(),
                builtin: Builtin::Read,
                arguments_json: "{}".into(),
            },
            ToolContext::new("s1", "t1", CancellationToken::new()),
        ).await;
    }).await;

    let records = sink.take();
    assert!(records.iter().any(|record| record.event_name == "tool_started"));
    assert!(records.iter().any(|record| record.event_name == "tool_completed"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p tool --test telemetry_scheduler_test scheduler_emits_tool_started_and_completed -- --exact`
Expected: FAIL because scheduler emits no tracing events.

**Step 3: Write minimal implementation**

```rust
// tool/src/scheduler.rs
pub async fn execute_builtin(
    &self,
    call: BuiltinToolCall,
    ctx: ToolContext,
) -> Result<ToolResult, ToolError> {
    let start = std::time::Instant::now();
    tracing::info!(
        event_name = "tool_started",
        tool_name = builtin_name.as_str(),
        session_id = ctx.session_id.as_str(),
        turn_id = ctx.turn_id.as_str()
    );

    let result = tool.execute(ctx, args).await;

    tracing::info!(
        event_name = "tool_completed",
        tool_name = builtin_name.as_str(),
        tool_outcome = if result.is_ok() { "success" } else { "failed" },
        tool_duration_ms = start.elapsed().as_millis() as u64
    );

    result
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p tool --test telemetry_scheduler_test scheduler_emits_tool_started_and_completed -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add tool/Cargo.toml tool/src/scheduler.rs tool/tests/telemetry_scheduler_test.rs
git commit -m "feat: instrument tool scheduler telemetry"
```

### Task 9: Instrument turn lifecycle and per-turn ordering

**Files:**
- Modify: `turn/Cargo.toml`
- Modify: `turn/src/driver.rs`
- Create: `turn/tests/telemetry_turn_test.rs`

**Step 1: Write the failing turn telemetry test**

```rust
mod support;

use std::sync::Arc;
use telemetry::{RecordingSink, TelemetryConfig, TelemetryLayer};
use tracing_subscriber::{layer::SubscriberExt, Registry};
use turn::{TurnContext, TurnDriver};

#[tokio::test]
async fn turn_driver_emits_correlated_turn_events_with_monotonic_sequence_numbers() {
    let sink = RecordingSink::default();
    let subscriber = Registry::default().with(TelemetryLayer::new(sink.clone(), TelemetryConfig::default()));

    tracing::subscriber::with_default(subscriber, || async {
        let context = TurnContext {
            session_id: "session-1".into(),
            turn_id: "turn-1".into(),
            user_message: "hello".into(),
        };

        let (handle, task) = TurnDriver::spawn(
            context,
            Arc::new(support::text_only_model(["hel", "lo"])),
            Arc::new(support::FakeToolRunner::default()),
            Arc::new(support::FakeAuthorizer::default()),
            Arc::new(support::FakeObserver),
        );

        while handle.next_event().await.is_some() {}
        task.await.unwrap().unwrap();
    }).await;

    let records = sink.take();
    let turn_records: Vec<_> = records.into_iter().filter(|record| record.turn_id == "turn-1").collect();
    assert!(turn_records.iter().any(|record| record.event_name == "turn_started"));
    assert!(turn_records.iter().any(|record| record.event_name == "turn_finished"));
    assert!(turn_records.windows(2).all(|pair| pair[0].sequence_no < pair[1].sequence_no));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p turn --test telemetry_turn_test turn_driver_emits_correlated_turn_events_with_monotonic_sequence_numbers -- --exact`
Expected: FAIL because turn driver emits no tracing telemetry events or sequence numbers.

**Step 3: Write minimal implementation**

```rust
// turn/src/driver.rs
pub struct TurnDriver {
    // existing fields
    next_sequence_no: u32,
}

async fn run(mut self) -> Result<(), TurnError> {
    tracing::info!(
        event_name = "turn_started",
        event_priority = "low",
        session_id = self.context.session_id.as_str(),
        turn_id = self.context.turn_id.as_str(),
        sequence_no = self.next_sequence()
    );
    // existing run loop
}

async fn emit(&self, event: TurnEvent) -> Result<(), TurnError> {
    match &event {
        TurnEvent::StepFinished { step_index, .. } => tracing::info!(
            event_name = "step_finished",
            step_index = *step_index,
            sequence_no = self.next_sequence()
        ),
        TurnEvent::TurnFinished { .. } => tracing::info!(
            event_name = "turn_finished",
            event_priority = "high",
            sequence_no = self.next_sequence()
        ),
        _ => {}
    }

    self.observer.on_event(&event).await?;
    self.event_tx.send(event).await.map_err(|_| TurnError::Runtime("turn event receiver dropped".into()))
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p turn --test telemetry_turn_test turn_driver_emits_correlated_turn_events_with_monotonic_sequence_numbers -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add turn/Cargo.toml turn/src/driver.rs turn/tests/telemetry_turn_test.rs
git commit -m "feat: instrument turn lifecycle telemetry"
```

### Task 10: Finish failure injection, graceful shutdown, and verification matrix

**Files:**
- Create: `telemetry/tests/runtime_integration_test.rs`
- Modify: `telemetry/src/lib.rs`
- Modify: `telemetry/src/batch.rs`
- Modify: `telemetry/src/writer.rs`

**Step 1: Write the failing runtime tests**

```rust
use std::time::Duration;
use telemetry::{DegradationPolicy, TelemetryConfig, TelemetryRuntime};

#[tokio::test]
async fn shutdown_flushes_high_priority_records_before_exit() {
    let runtime = TelemetryRuntime::for_test(TelemetryConfig::default());
    runtime.record_test_high_priority("turn_finished");
    runtime.shutdown(Duration::from_secs(1)).unwrap();
    assert_eq!(runtime.test_sink().records().len(), 1);
}

#[tokio::test]
async fn drop_on_failure_counts_dropped_events_instead_of_panicking() {
    let mut config = TelemetryConfig::default();
    config.degradation_policy = DegradationPolicy::DropOnFailure;
    let runtime = TelemetryRuntime::with_failing_writer_for_test(config);
    runtime.record_test_low_priority("step_finished");
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(runtime.metrics().events_dropped_total(), 1);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p telemetry --test runtime_integration_test -q`
Expected: FAIL because runtime shutdown, failing writer hooks, or metrics are incomplete.

**Step 3: Write minimal implementation**

```rust
// telemetry/src/lib.rs
pub enum DegradationPolicy {
    Strict,
    DropOnFailure,
    BufferOnFailure { max_buffer_size: usize },
}

impl TelemetryRuntime {
    pub fn shutdown(self, timeout: Duration) -> Result<(), TelemetryError> {
        // signal background task, flush queues, and return timeout error when necessary
    }
}
```

```rust
// telemetry/src/writer.rs
fn is_retryable(status: reqwest::StatusCode) -> bool {
    status.is_server_error() || status.as_u16() == 429
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p telemetry --test runtime_integration_test -q`
Expected: PASS

**Step 5: Run final verification matrix**

Run: `cargo test -p telemetry`
Expected: PASS

Run: `cargo test -p provider --test provider_telemetry_test`
Expected: PASS

Run: `cargo test -p tool --test telemetry_scheduler_test`
Expected: PASS

Run: `cargo test -p turn --test telemetry_turn_test`
Expected: PASS

Run: `cargo test -p desktop --lib`
Expected: PASS

**Step 6: Commit**

```bash
git add telemetry/src/lib.rs telemetry/src/batch.rs telemetry/src/writer.rs telemetry/tests/runtime_integration_test.rs
git commit -m "feat: finalize telemetry runtime behavior"
```

## Constraints to Keep During Implementation

1. Do not add telemetry-specific fields into `argus_core`; keep v1 integration at crate boundaries.
2. Keep the writer on raw `reqwest` plus ClickHouse HTTP API; do not add a separate ClickHouse client crate unless raw HTTP proves insufficient.
3. Do not persist full prompts or responses unless `full_logging = true`.
4. Every billing-authoritative `llm_response_completed` event must set `billing_dedupe_key`.
5. Keep `delta-events` off by default and avoid emitting `llm_delta` in tests unless a test explicitly covers that feature flag.

## Final Review Checklist

1. `telemetry_logs` schema in `sql/schema.sql` matches the design doc field names exactly.
2. No materialized view double-counts both `turn_finished` and `llm_response_completed` for token totals.
3. Provider tasks spawned with `tokio::spawn` are instrumented with the current span so trace context survives async boundaries.
4. Queue saturation always increments a counter before dropping low-priority events.
5. Desktop shutdown flushes telemetry before returning from `run()`.

Plan complete and saved to `docs/plans/2026-03-07-telemetry-implementation-plan.md`. Two execution options:

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open a new session inside a dedicated git worktree and execute this plan there so implementation changes stay isolated from the current workspace

Which approach?
