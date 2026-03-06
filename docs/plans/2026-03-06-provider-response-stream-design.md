# Provider ResponseStream Design

Date: 2026-03-06
Status: Approved for planning
Scope: New end-to-end provider streaming path; no implementation in this document

## 1. Goal

Introduce a unified streaming surface for provider-driven model responses:

- `core` owns the stream contract and event protocol.
- `provider` owns HTTP request execution, SSE transport, dialect mapping, and stream lifecycle.
- `llm-client` and the current `agent-turn` path are legacy and will be retired after the new `turn` path is in place.

The immediate deliverable is an opaque `core::ResponseStream` returned by `provider`, with the first implementation backed by `tokio::spawn` plus a bounded `mpsc` channel.

## 2. Context

Current workspace state:

- `core` already defines `ResponseEvent`, `ToolCall`, and `ResponseContract`.
- `provider` currently maps raw SSE payloads into `Vec<ResponseEvent>` via dialect-specific mappers.
- `llm-client` currently owns HTTP + SSE transport, but that crate is being phased out.

The gap is a canonical end-to-end streaming API that lets future `turn` code consume one provider-owned stream instead of coordinating transport and mapper layers separately.

## 3. Locked Decisions

1. Ownership split
- `core` defines protocol-only types: `ResponseEvent`, `ResponseContract`, and `ResponseStream`.
- `provider` defines the concrete streaming implementation and returns `ResponseStream`.

2. Stream item type
- `ResponseStream` implements `Stream<Item = ResponseEvent>`.
- We do not use `Stream<Item = Result<ResponseEvent, _>>`.

Reason:
- `ResponseEvent::Error(_)` is already the stream-level terminal error event.
- Adding `Result` would create two parallel error channels with overlapping meaning.

3. Construction strategy
- `ResponseStream` is an opaque public type.
- The first implementation uses `tokio::spawn` + bounded `tokio::sync::mpsc`.
- Internal representation must remain swappable so a future direct-stream implementation can replace the channel path without changing call sites.

4. Runtime ownership
- `provider` owns HTTP request execution, SSE decoding, mapper invocation, contract enforcement, backpressure, and cancellation.
- Consumers only poll `ResponseStream`.

5. Migration direction
- New code must integrate with `provider` + `core`.
- `llm-client` and the existing `agent-turn` path are migration targets, not long-term dependencies.

## 4. Core Contract

### 4.1 `ResponseEvent`

`ResponseEvent` remains the canonical stream protocol. Its terminal rule is unchanged:

- exactly one terminal event: `Done(_)` or `Error(_)`
- no events after terminal

`core::Error` remains a generic protocol-level terminal payload. It is intentionally not expanded with provider-specific error categories in this phase.

### 4.2 `ResponseStream`

`core` adds a new opaque stream type:

```rust
pub struct ResponseStream { /* private */ }
```

Required behavior:

- implements `futures::Stream<Item = ResponseEvent>`
- drops cleanly and triggers cancellation of the background producer task
- does not expose the underlying `mpsc::Receiver`
- is constructible only through controlled APIs used by `provider`

The internal representation for the first iteration is expected to contain:

- a bounded `mpsc::Receiver<ResponseEvent>`
- a cancellation or abort handle for the spawned producer task

The internal shape is not part of the public contract.

### 4.3 `ResponseContract`

`ResponseContract` remains the single source of truth for terminal-event exclusivity. Provider code must pass every emitted event through the contract before sending it to the stream channel.

## 5. Provider API

`provider` becomes the end-to-end streaming boundary.

Recommended external shape:

```rust
pub struct ProviderClient {
    http: reqwest::Client,
    config: ProviderConfig,
}

impl ProviderClient {
    pub fn stream(&self, request: Request) -> Result<ResponseStream, provider::Error>;
}
```

Supporting config continues to carry dialect selection:

```rust
pub enum Dialect {
    Openai,
    Zai,
}
```

Notes:

- The exact request model can be finalized during implementation, but the ownership boundary is fixed: request construction happens in `provider`, not in `llm-client`.
- Existing mapper APIs (`feed`, `on_done`) remain valid internal building blocks.

## 6. Provider Runtime Data Flow

The first implementation uses one spawned task per streaming request.

### 6.1 Startup path

`ProviderClient::stream()` performs synchronous setup only:

- validate configuration
- build request
- create bounded channel
- create cancellation/abort handle
- spawn background task
- return `ResponseStream`

If startup fails before the task is returned, `stream()` returns `Err(provider::Error)`.

### 6.2 Background task pipeline

Inside the spawned task:

1. send HTTP request with `Accept: text/event-stream`
2. validate status and content type
3. decode SSE messages using provider-owned transport code built on `vendor/eventsource_stream`
4. feed each payload into the dialect mapper
5. validate every emitted `ResponseEvent` with `ResponseContract`
6. forward accepted events into the channel

### 6.3 `[DONE]` handling

- On `[DONE]`, the task must call `mapper.on_done()`.
- The resulting events are forwarded in order.
- The final emitted event must be `Done(_)`.

### 6.4 Abnormal termination

If the runtime sees any post-start failure, it emits a single terminal `ResponseEvent::Error(core::Error)` and stops:

- HTTP streaming failure
- SSE parse failure
- JSON decode failure
- mapper protocol failure
- EOF before `[DONE]`
- explicit cancellation

Channel closure after a terminal event is normal and does not require an additional signal.

### 6.5 Cancellation and backpressure

- Use a bounded channel, not unbounded.
- Slow consumers must naturally apply backpressure to the producer task.
- Dropping `ResponseStream` must cancel the producer task promptly.
- The producer loop should observe cancellation alongside the next SSE event, for example with `tokio::select!`.

## 7. Error Model

Provider runtime classification is provider-specific, not part of `core`.

Recommended provider-side runtime error type:

```rust
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

Design intent:

- `provider::Error` covers startup failures and top-level API construction failures.
- `provider::StreamError` classifies post-start runtime failures inside the producer task.
- Runtime failures are converted into `ResponseEvent::Error(core::Error)` before they leave the provider boundary.

This keeps `core` transport-agnostic while still giving provider internals a stable error taxonomy.

## 8. Transport Placement

SSE transport moves into `provider`.

Recommended module shape:

- `provider/src/transport/sse/mod.rs`
- `provider/src/transport/sse/error.rs`
- `provider/src/transport/sse/event_source.rs`

Implementation should reuse the existing design from `llm-client/src/sse`, but the new stream path must not depend on `llm-client` at runtime.

## 9. Testing Strategy

### 9.1 `core`

- `ResponseStream` yields queued events in order
- dropping `ResponseStream` triggers producer cancellation
- `ResponseContract` continues enforcing terminal exclusivity

### 9.2 `provider` mapper tests

Keep and extend current dialect replay tests:

- OpenAI content/reasoning/tool assembly
- Z.AI content/reasoning/tool assembly
- MCP upgrade behavior
- terminal usage emission

### 9.3 `provider` stream integration tests

Add end-to-end tests using mocked SSE responses:

- successful `[DONE]` flow
- tool-call flush on terminal
- runtime parse/protocol failures become `ResponseEvent::Error`
- EOF without `[DONE]` becomes terminal error
- dropped consumer cancels background task

## 10. Migration Plan

1. Add `ResponseStream` to `core`.
2. Add provider-owned SSE transport and end-to-end `stream()` API.
3. Keep current mapper tests and APIs while the new stream path is added.
4. Wire the future `turn` path directly to `provider`.
5. Remove `llm-client` and old `agent-turn` integrations once the new path is proven.

## 11. Out Of Scope

- Reworking the `ResponseEvent` shape beyond adding `ResponseStream`
- Changing tool-call protocol semantics
- Introducing retries or reconnection policies beyond the minimum required for correctness
- Designing the replacement `turn` runtime in this document
