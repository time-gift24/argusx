# Core + Provider-OpenAI Streaming Design (MVP)

Date: 2026-03-06
Status: Approved for planning
Scope phase: Foundation only (no migration of existing runtime path)

## 1. Context And Goal

We will fully rewrite the lower-level streaming path in a new architecture, but this phase only builds the minimum viable foundation in parallel without touching existing `llm-client/llm-provider/agent-turn` behavior.

Long-term target is destructive replacement (A). Current phase keeps old path intact and validates new protocol and adapter quality first.

## 2. Architecture

### 2.1 New crates

1. `core`
- Defines the stable streaming event contract.
- Owns protocol/state-machine constraints.
- No provider-specific parsing logic.

2. `provider-openai`
- Depends on vendored event stream source and OpenAI Chat Completions interface.
- Converts Chat Completions SSE chunks into `core::ResponseEvent`.
- Handles stream transport, chunk parsing, tool-call assembly, and terminal event emission.

### 2.2 Current-phase compatibility policy

- Do not modify current production path in this phase.
- New crates are parallel path for verification.
- Migration/replacement happens in later review rounds.

## 3. Event Contract

```rust
pub enum ResponseEvent {
    Created(Meta),

    ContentDelta(Arc<str>),
    ReasoningDelta(Arc<str>),
    ToolDelta(Arc<str>),

    ContentDone(String),
    ReasoningDone(String),
    ToolDone(ToolCall),

    Done(Option<Usage>),
    Error(Error),
}
```

### 3.1 ToolCall

`ToolDone` contains structured tool calls; `ToolDelta` is raw streaming text for real-time display only.

```rust
pub enum ToolCall {
    FunctionCall {
        call_id: String,
        name: String,
        arguments_json: String,
    },
    Mcp {
        server: String,
        method: String,
        payload_json: String,
        call_id: Option<String>,
    },
}
```

Notes:
- MVP implements `FunctionCall` assembly from OpenAI chunks.
- `Mcp` variant is reserved for future providers.

### 3.2 Meta / Usage

- `Created(Meta)` replaces the previous single `Model(String)`-style metadata event.
- Terminal event is `Done(Option<Usage>)`.
- If usage appears before `[DONE]`, emit `Done(Some(usage))`.
- If stream ends without usage, emit `Done(None)`.

## 4. Stream Semantics And State Machine

### 4.1 Strong contract

- `Delta*` can appear multiple times in order.
- `ContentDone` at most once.
- `ReasoningDone` at most once.
- `ToolDone` at most once per tool call.
- Exactly one terminal event: `Done(_)` or `Error(_)`.
- No event is allowed after terminal event.

### 4.2 Adapter mapping rules (OpenAI Chat Completions SSE)

- `delta.content` -> `ContentDelta(Arc<str>)`
- `delta.reasoning_content` -> `ReasoningDelta(Arc<str>)`
- `delta.tool_calls[*].function.arguments` -> append internal buffer and emit `ToolDelta(Arc<str>)`
- `finish_reason == "tool_calls"` -> flush buffered tool calls as `ToolDone(ToolCall::FunctionCall{...})`
- `finish_reason == "stop"` -> emit `ContentDone` / `ReasoningDone` if corresponding buffers are non-empty
- `[DONE]` -> emit `Done(Option<Usage>)`
- Transport/parse/protocol errors -> emit `Error(Error)` and terminate

## 5. Error Model (MVP)

- `Error::Transport`: network/SSE I/O failures
- `Error::Parse`: malformed chunk payload
- `Error::Protocol`: illegal ordering, illegal terminal behavior, invalid tool assembly state

`Error` is terminal and exclusive with `Done`.

## 6. Testing Strategy (MVP)

1. `core` unit tests
- state-machine invariants
- terminal exclusivity
- done-event cardinality

2. `provider-openai` unit tests
- chunk-to-event mapping
- tool-call argument assembly
- usage-carrying terminal event behavior

3. fixture-based integration tests
- replay SSE fixture stream
- assert ordered event sequence and terminal behavior

## 7. Test Fixture For Immediate Use

Primary fixture file:
- `docs/plans/testdata/2026-03-06-openai-chat-completions-sse.txt`

This fixture contains a full example stream including:
- reasoning deltas
- content deltas
- function tool-call argument deltas
- usage in the penultimate event
- terminal `[DONE]`

## 8. Milestones

1. Foundation (current)
- Finalize design and fixture data
- Freeze event contract for MVP

2. Build MVP (next)
- Implement `core`
- Implement `provider-openai`
- Add fixture replay tests

3. Migration (later review)
- Replace old path incrementally after quality review

## 9. Out Of Scope For This Phase

- Modifying existing `agent-turn` integration path
- Multi-provider abstraction
- Runtime optimization passes beyond correctness
- MCP parser implementation (only enum shape is reserved)
