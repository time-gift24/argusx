# Provider ResponseStream Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add an opaque `core::ResponseStream` and a provider-owned end-to-end streaming API that executes HTTP requests, consumes SSE, maps payloads into `ResponseEvent`, and returns a single stream for the future `turn` path.

**Architecture:** Keep `core` protocol-only by adding `ResponseStream` next to `ResponseEvent` and `ResponseContract`. Build the first concrete implementation in `provider` with `tokio::spawn`, a bounded `mpsc` channel, provider-owned SSE transport code, and existing dialect mappers (`openai` and `zai`). Runtime failures stay inside the stream as `ResponseEvent::Error(core::Error)`; provider-specific runtime classification lives in provider-only error types.

**Tech Stack:** Rust workspace crates, `tokio`, `futures`, `reqwest`, vendored `eventsource-stream`, existing `provider` dialect mappers, `wiremock`, fixture replay tests, `cargo test`.

---

### Task 1: Add `core::ResponseStream` and stream contract tests

**Files:**
- Modify: `Cargo.toml`
- Modify: `core/Cargo.toml`
- Modify: `core/src/lib.rs`
- Create: `core/tests/response_stream_test.rs`
- Modify: `core/tests/contract_state_machine_test.rs`

**Step 1: Write the failing tests**

```rust
use argus_core::{Error, ResponseEvent, ResponseStream};
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio::task;

#[tokio::test]
async fn response_stream_yields_events_in_order() {
    let (tx, rx) = mpsc::channel(4);
    let producer = task::spawn(async move {
        tx.send(ResponseEvent::ContentDelta("hi".into())).await.unwrap();
        tx.send(ResponseEvent::Done(None)).await.unwrap();
    });

    let mut stream = ResponseStream::from_parts(rx, producer.abort_handle());
    assert!(matches!(stream.next().await, Some(ResponseEvent::ContentDelta(_))));
    assert!(matches!(stream.next().await, Some(ResponseEvent::Done(None))));
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn dropping_response_stream_aborts_producer() {
    let (_tx, rx) = mpsc::channel::<ResponseEvent>(1);
    let producer = task::spawn(async {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    });
    let handle = producer.abort_handle();
    let stream = ResponseStream::from_parts(rx, handle);
    drop(stream);
    tokio::task::yield_now().await;
    assert!(producer.is_finished());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p core --test response_stream_test -q`
Expected: FAIL with missing `ResponseStream` and missing `tokio`/`futures` dependencies.

**Step 3: Write minimal implementation**

```rust
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;
use tokio::task::AbortHandle;

pub struct ResponseStream {
    rx: mpsc::Receiver<ResponseEvent>,
    abort: Option<AbortHandle>,
}

impl ResponseStream {
    pub fn from_parts(rx: mpsc::Receiver<ResponseEvent>, abort: AbortHandle) -> Self {
        Self {
            rx,
            abort: Some(abort),
        }
    }
}

impl Stream for ResponseStream {
    type Item = ResponseEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

impl Drop for ResponseStream {
    fn drop(&mut self) {
        if let Some(abort) = self.abort.take() {
            abort.abort();
        }
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p core --test response_stream_test --test contract_state_machine_test -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add Cargo.toml core/Cargo.toml core/src/lib.rs core/tests/response_stream_test.rs core/tests/contract_state_machine_test.rs
git commit -m "feat(core): add opaque response stream type"
```

### Task 2: Add provider client skeleton and provider-specific error taxonomy

**Files:**
- Modify: `provider/Cargo.toml`
- Modify: `provider/src/lib.rs`
- Modify: `provider/src/error.rs`
- Create: `provider/src/client.rs`
- Create: `provider/src/request.rs`
- Create: `provider/tests/provider_client_smoke_test.rs`

**Step 1: Write the failing tests**

```rust
use provider::{Dialect, ProviderClient, ProviderConfig};

#[test]
fn provider_client_can_be_constructed() {
    let cfg = ProviderConfig {
        dialect: Dialect::Openai,
        base_url: "https://example.test".into(),
        api_key: "secret".into(),
        headers: Default::default(),
    };
    let _client = ProviderClient::new(cfg).unwrap();
}

#[test]
fn provider_error_kind_is_provider_specific() {
    let err = provider::StreamError {
        kind: provider::ErrorKind::Protocol,
        message: "bad chunk".into(),
    };
    assert!(matches!(err.kind, provider::ErrorKind::Protocol));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p provider --test provider_client_smoke_test -q`
Expected: FAIL with missing `ProviderClient`, `ProviderConfig`, `StreamError`, and `ErrorKind`.

**Step 3: Write minimal implementation**

```rust
pub struct ProviderConfig {
    pub dialect: Dialect,
    pub base_url: String,
    pub api_key: String,
    pub headers: std::collections::HashMap<String, String>,
}

pub struct ProviderClient {
    http: reqwest::Client,
    config: ProviderConfig,
}

pub type Request = crate::dialect::openai::schema::request::ChatCompletionsOptions;

impl ProviderClient {
    pub fn new(config: ProviderConfig) -> Result<Self, Error> {
        if config.base_url.trim().is_empty() {
            return Err(Error::Config("base_url is required".into()));
        }
        if config.api_key.trim().is_empty() {
            return Err(Error::Config("api_key is required".into()));
        }
        Ok(Self {
            http: reqwest::Client::new(),
            config,
        })
    }
}

pub struct StreamError {
    pub kind: ErrorKind,
    pub message: String,
}

pub enum ErrorKind {
    Transport,
    HttpStatus,
    Parse,
    Protocol,
    Cancelled,
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p provider --test provider_client_smoke_test -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add provider/Cargo.toml provider/src/lib.rs provider/src/error.rs provider/src/client.rs provider/src/request.rs provider/tests/provider_client_smoke_test.rs
git commit -m "feat(provider): add client skeleton and provider stream errors"
```

### Task 3: Move minimal SSE transport into `provider`

**Files:**
- Modify: `provider/Cargo.toml`
- Create: `provider/src/transport/mod.rs`
- Create: `provider/src/transport/sse/mod.rs`
- Create: `provider/src/transport/sse/error.rs`
- Create: `provider/src/transport/sse/event_source.rs`
- Modify: `provider/src/lib.rs`
- Create: `provider/tests/sse_transport_test.rs`
- Reference: `llm-client/src/sse/mod.rs`
- Reference: `llm-client/src/sse/error.rs`
- Reference: `llm-client/src/sse/event_source.rs`

**Step 1: Write the failing tests**

```rust
use futures::StreamExt;
use provider::transport::sse::{Event, EventSource};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn event_source_reads_message_and_done_boundary() {
    let server = MockServer::start().await;
    let body = "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"g\",\"choices\":[]}\n\n";

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body),
        )
        .mount(&server)
        .await;

    let response = reqwest::Client::new()
        .post(format!("{}/chat/completions", server.uri()))
        .send()
        .await
        .unwrap();

    let mut es = EventSource::from_response(response).unwrap();
    assert!(matches!(es.next().await, Some(Ok(Event::Open))));
    assert!(matches!(es.next().await, Some(Ok(Event::Message(_)))));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p provider --test sse_transport_test -q`
Expected: FAIL with missing transport module.

**Step 3: Implement minimal transport**
- Port the response-validation and message-stream path from `llm-client/src/sse`.
- Keep only the pieces needed for provider-owned, single-response streaming.
- Preserve support for `Event::Open` plus `Event::Message`.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p provider --test sse_transport_test -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add provider/Cargo.toml provider/src/lib.rs provider/src/transport provider/tests/sse_transport_test.rs
git commit -m "feat(provider): add provider-owned sse transport"
```

### Task 4: Implement `ProviderClient::stream()` for OpenAI dialect

**Files:**
- Modify: `provider/src/client.rs`
- Modify: `provider/src/error.rs`
- Modify: `provider/src/lib.rs`
- Modify: `provider/src/dialect/openai/mapper.rs`
- Create: `provider/tests/openai_stream_client_test.rs`
- Reference: `provider/src/dialect/openai/schema/request.rs`
- Reference: `provider/tests/fixtures/2026-03-06-openai-chat-completions-sse.txt`

**Step 1: Write the failing test**

```rust
use argus_core::ResponseEvent;
use futures::StreamExt;
use provider::{Dialect, ProviderClient, ProviderConfig, Request};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn openai_stream_returns_created_deltas_and_done() {
    let server = MockServer::start().await;
    let body = include_str!("fixtures/2026-03-06-openai-chat-completions-sse.txt");

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body),
        )
        .mount(&server)
        .await;

    let client = ProviderClient::new(ProviderConfig {
        dialect: Dialect::Openai,
        base_url: server.uri(),
        api_key: "test-key".into(),
        headers: Default::default(),
    })
    .unwrap();

    let request: Request = provider::dialect::openai::schema::request::ChatCompletionsOptions {
        model: "gpt-test".into(),
        stream: Some(true),
        ..Default::default()
    };

    let mut stream = client.stream(request).unwrap();
    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(event);
    }
    assert!(events.iter().any(|e| matches!(e, ResponseEvent::Created(_))));
    assert!(events.iter().any(|e| matches!(e, ResponseEvent::Done(Some(_)))));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p provider --test openai_stream_client_test -q`
Expected: FAIL with missing streaming API.

**Step 3: Implement minimal OpenAI stream path**
- Build the POST request from OpenAI chat-completions options.
- Spawn a background task.
- Use provider-owned SSE transport to read messages.
- Feed payloads into `dialect::openai::mapper::Mapper`.
- Pass emitted events through `ResponseContract`.
- Send events into the bounded channel and return `ResponseStream`.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p provider --test openai_stream_client_test --test openai_compat_replay_test -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add provider/src/client.rs provider/src/error.rs provider/src/lib.rs provider/src/dialect/openai/mapper.rs provider/tests/openai_stream_client_test.rs
git commit -m "feat(provider/openai): add end-to-end response stream client"
```

### Task 5: Extend end-to-end streaming to Z.AI dialect and shared dispatch

**Files:**
- Modify: `provider/src/client.rs`
- Modify: `provider/src/lib.rs`
- Modify: `provider/src/dialect/zai/mapper.rs`
- Create: `provider/tests/zai_stream_client_test.rs`
- Reference: `provider/tests/fixtures/2026-03-06-zai-chat-completions-sse.txt`

**Step 1: Write the failing test**

```rust
use argus_core::ResponseEvent;
use futures::StreamExt;
use provider::{Dialect, ProviderClient, ProviderConfig, Request};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn zai_stream_emits_mcp_and_done() {
    let server = MockServer::start().await;
    let body = include_str!("fixtures/2026-03-06-zai-chat-completions-sse.txt");

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body),
        )
        .mount(&server)
        .await;

    let client = ProviderClient::new(ProviderConfig {
        dialect: Dialect::Zai,
        base_url: server.uri(),
        api_key: "test-key".into(),
        headers: Default::default(),
    })
    .unwrap();

    let request: Request = provider::dialect::openai::schema::request::ChatCompletionsOptions {
        model: "glm-test".into(),
        stream: Some(true),
        ..Default::default()
    };

    let mut stream = client.stream(request).unwrap();
    let events: Vec<_> = stream.collect().await;
    assert!(events.iter().any(|e| matches!(e, ResponseEvent::ToolDone(_))));
    assert!(events.iter().any(|e| matches!(e, ResponseEvent::Done(Some(_)))));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p provider --test zai_stream_client_test -q`
Expected: FAIL with dialect dispatch limited to OpenAI only.

**Step 3: Implement minimal shared dispatch**
- Branch on `ProviderConfig.dialect`.
- Reuse the same spawned-task pattern and contract enforcement.
- Delegate payload mapping to the corresponding dialect mapper.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p provider --test zai_stream_client_test --test zai_fixture_replay_test -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add provider/src/client.rs provider/src/lib.rs provider/src/dialect/zai/mapper.rs provider/tests/zai_stream_client_test.rs
git commit -m "feat(provider/zai): add end-to-end response stream client"
```

### Task 6: Add terminal error, abnormal EOF, and cancellation coverage

**Files:**
- Modify: `core/tests/response_stream_test.rs`
- Modify: `provider/tests/openai_stream_client_test.rs`
- Create: `provider/tests/provider_stream_failure_test.rs`
- Modify: `provider/src/client.rs`
- Modify: `provider/src/error.rs`

**Step 1: Write the failing tests**

```rust
use argus_core::ResponseEvent;
use futures::StreamExt;

#[tokio::test]
async fn abnormal_eof_becomes_terminal_error_event() {
    // mock SSE body without [DONE]
    // assert final event is ResponseEvent::Error(_)
}

#[tokio::test]
async fn transport_parse_failure_becomes_terminal_error_event() {
    // mock malformed json event
    // assert final event is ResponseEvent::Error(_)
}

#[tokio::test]
async fn dropping_consumer_aborts_background_task() {
    // create stream, drop before completion, assert task abort path
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p provider --test provider_stream_failure_test -q`
Expected: FAIL with missing abnormal-termination handling.

**Step 3: Implement minimal fixes**
- Translate `StreamError` into `ResponseEvent::Error(core::Error)`.
- Emit exactly one terminal event on abnormal EOF and parser failures.
- Ensure drop/cancel path aborts the task and does not emit duplicate terminal events after cancellation.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p provider --test provider_stream_failure_test --test openai_stream_client_test -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add core/tests/response_stream_test.rs provider/src/client.rs provider/src/error.rs provider/tests/openai_stream_client_test.rs provider/tests/provider_stream_failure_test.rs
git commit -m "test(provider): cover response stream terminal failures and cancellation"
```

### Task 7: Run focused verification and capture migration notes

**Files:**
- Modify: `docs/plans/2026-03-06-provider-response-stream-design.md`
- Optional: `docs/plans/2026-03-06-provider-response-stream-implementation.md`

**Step 1: Run focused verification**

Run: `cargo test -p core -p provider -q`
Expected: PASS.

**Step 2: Run lint on touched crates**

Run: `cargo clippy -p core -p provider --all-targets -- -D warnings`
Expected: PASS.

**Step 3: Update docs only if implementation changed a locked design detail**

```markdown
- record any approved divergence from the design
- note any migration blocker for the future `turn` path
```

**Step 4: Commit**

```bash
git add docs/plans/2026-03-06-provider-response-stream-design.md docs/plans/2026-03-06-provider-response-stream-implementation.md
git commit -m "docs(plans): finalize provider response stream implementation notes"
```
