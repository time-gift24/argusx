# Turn Shared-State Refactor Design

## Goal

Refactor the `turn` runtime so that:

- `TurnState` no longer stores stale cloned snapshots
- `LlmStepRequest.messages` is a shared read-only snapshot instead of a deep-copied `Vec`
- `TurnEvent` delta payloads use shared string storage
- tool results continue to stream in completion order but are always appended back into the transcript in original tool-call order

## Problems In The Current Design

### Redundant Context Ownership

`TurnDriver` owns `context`, while `TurnState::Ready` also stores a cloned `TurnContext`. The driver runtime does not actually dispatch on `TurnState`, so `Ready(TurnContext)` is duplicate data rather than the single source of truth.

### Deep-Copied Step Requests

Each model step builds `LlmStepRequest.messages` with `self.transcript.messages().to_vec()`. That creates an owned deep copy of the transcript for every step, even though model runners only need an immutable snapshot for that invocation.

### Event Payload Re-Allocation

`ResponseEvent::ContentDelta` and `ResponseEvent::ReasoningDelta` already arrive as `Arc<str>`, but `TurnEvent` converts them to `String`, paying an extra allocation and copy before they are sent through the event channel.

### Stale Streaming State

`run()` builds a local `active_step`, then stores `TurnState::StreamingLlm(active_step.clone())`. Subsequent tool-call accumulation only mutates the local variable, so `self.state` diverges immediately and stops being an accurate representation of the active step.

### Ordering Needs Two Different Guarantees

Tool execution is concurrent, so `ToolCallCompleted` events must continue to be emitted in completion order. However, transcript appends for the next LLM request must remain aligned with the original `tool_calls` array order from the triggering assistant step.

## Chosen Approach

Use single ownership for mutable runtime state and `Arc`-backed shared storage for read-only snapshots.

### Mutable Runtime State

- `TurnDriver` keeps `state` as the only mutable source of truth for the active step and tool batch.
- `TurnState::Ready` becomes a payload-free marker variant.
- `StreamingLlm` state is mutated in place instead of cloning a local `ActiveLlmStep`.

### Shared Read-Only Snapshots

- `LlmStepRequest.messages` becomes `Arc<[Arc<TurnMessage>]>` so snapshots share message ownership instead of deep-cloning message contents.
- `TurnMessage` string fields become `Arc<str>` where the content is immutable text or IDs.
- `TurnEvent` text deltas and stable string identifiers become `Arc<str>`.
- Tool-call collections reused across state/transcript/request boundaries become `Arc<[ToolCall]>`.

### Tool Ordering Contract

- `ToolCallCompleted` events are emitted as tasks finish.
- `execute_tool_batch()` returns outcomes in original call order.
- transcript replay for the next LLM step iterates the original tool-call array order, not completion order and not lexical `call_id` order.

## API Shape

### `TurnState`

```rust
enum TurnState {
    Ready,
    StreamingLlm(ActiveLlmStep),
    WaitingTools(ToolBatch),
    WaitingForPermission(PermissionPause),
    Completed(TurnSummary),
    Cancelled(TurnSummary),
    Failed(TurnFailure),
}
```

### `LlmStepRequest`

```rust
pub struct LlmStepRequest {
    pub session_id: String,
    pub turn_id: String,
    pub step_index: u32,
    pub messages: Arc<[Arc<TurnMessage>]>,
    pub allow_tools: bool,
}
```

### `TurnEvent`

```rust
pub enum TurnEvent {
    TurnStarted,
    LlmTextDelta { text: Arc<str> },
    LlmReasoningDelta { text: Arc<str> },
    ToolCallPrepared { call: Arc<ToolCall> },
    ToolCallCompleted { call_id: Arc<str>, result: ToolOutcome },
    ToolCallPermissionRequested { request: PermissionRequest },
    ToolCallPermissionResolved { request_id: Arc<str>, decision: PermissionDecision },
    StepFinished { step_index: u32, reason: StepFinishReason },
    TurnFinished { reason: TurnFinishReason },
}
```

## Data-Flow Changes

1. The user message is appended into `TurnTranscript` once.
2. Each step takes an immutable `Arc<[TurnMessage]>` snapshot from the transcript.
3. `StreamingLlm` owns the mutable in-progress `ActiveLlmStep`.
4. When tool calls are complete, the driver converts that step state into a shared `ToolBatch`.
5. Tool results are emitted as they complete, but transcript append order comes from the batch call array.

## Testing Strategy

Add and/or update tests that prove:

- `LlmStepRequest.messages` is shared immutable data rather than a deep-copied `Vec`
- `TurnEvent` deltas preserve `Arc<str>` payloads
- `TurnState::StreamingLlm` remains the source of truth while tool calls are collected
- tool completion events may be out of order, but transcript replay into the next step is in original tool-call order
- permission-paused batches preserve the same original order guarantee

## Risks

- Public API breakage in `turn` tests and future provider integrations
- Refactoring `TurnMessage` fields to `Arc<str>` requires broad test updates
- Overusing `Arc` in hot paths could add atomic overhead, but this is still cheaper and simpler than pervasive owned deep copies plus self-referential borrowing schemes

## Non-Goals

- Rewriting `ToolRunner` to accept borrowed tool calls
- Re-introducing a fully explicit state-machine loop structure
- Buffering assistant text into the transcript in this change set
