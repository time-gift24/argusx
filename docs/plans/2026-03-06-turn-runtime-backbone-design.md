# Turn Runtime Backbone Design

Date: 2026-03-06
Status: Approved
Scope: Architecture and contract design only

## 1. Goal

Introduce a new `turn` runtime backbone that owns a single agentic turn from initial model stream through tool execution, permission gating, cancellation, and final completion.

This round focuses on a single turn only. `Session` remains a persistence boundary and `Agent` remains a conceptual long-lived owner, but neither requires a concrete workspace crate in v1.

## 2. Current Workspace Context

Current Rust workspace state:

- [`core`](/Users/wanyaozhong/Projects/argusx/core/src/lib.rs) defines provider-facing stream contracts such as `ResponseEvent`, `ResponseStream`, and `ToolCall`.
- [`provider`](/Users/wanyaozhong/Projects/argusx/provider/src/client.rs) owns HTTP/SSE streaming and normalizes provider deltas into `core::ResponseEvent`.
- [`tool`](/Users/wanyaozhong/Projects/argusx/tool/src/lib.rs) owns builtin and MCP execution plus scheduling policy.
- `agent`, `agent-core`, `agent-turn`, `llm-client`, and `llm-provider` have been removed from the main branch and are no longer the implementation target.

Implication:

- the new backbone should be introduced as a fresh `turn` runtime crate aligned to `core`, `provider`, and `tool`
- the old `tests/agent-turn-cli` and `tests/agent-session-cli` fixtures were stale historical artifacts and have been removed rather than carried forward into the new architecture

## 3. Approved Backbone Decisions

1. `Agent` is the long-lived conceptual runtime owner.
2. This iteration implements only `Turn`; `Session` stays minimal and mostly conceptual.
3. `Turn` owns the entire agentic loop for one user request.
4. `Turn` must support:
   - LLM streaming
   - tool batch execution with per-tool realtime updates
   - same-turn permission pause and resume
   - user cancellation with best-effort draining
5. `VercelEvent` is a transport adapter, not the internal source of truth.
6. Internal runtime logic should emit `TurnEvent` first, then map to Vercel Data Stream Protocol.

## 4. Architecture Options

### Option 1: Monolithic async loop

One `Turn::run()` function owns streaming, tool collection, permissions, cancellation, and event fan-out.

Pros:

- lowest initial code volume
- easy to prototype quickly

Cons:

- state becomes implicit
- cancellation and permission recovery become branch-heavy
- hard to test race conditions

### Option 2: Explicit state machine

`Turn` is modeled as explicit runtime states with clear transition rules.

Pros:

- clear ownership of streaming vs tool execution vs permission waiting
- easier cancellation and timeout semantics
- best fit for Rust tests and invariants

Cons:

- more types and boilerplate up front

### Option 3: Actor/inbox runtime

`Turn` receives messages such as stream chunks, tool completions, permission decisions, and cancel requests.

Pros:

- naturally models external control signals
- good future fit for highly concurrent orchestration

Cons:

- too heavy for current workspace stage
- debugging is more complex than needed for v1

### Selected approach

Option 2 is approved.

## 5. Conceptual Ownership Model

Although no concrete `agent` crate exists in the workspace today, the ownership model is still:

- `Agent` conceptually owns long-lived shared resources such as model client factories, tool runtime, policy resolver, MCP pools, and session store
- `Session` conceptually owns transcript and audit persistence only
- `Turn` is the only component that runs the agentic loop

This lets the current implementation introduce a `turn` crate without prematurely rebuilding the removed `agent*` stack.

## 6. Turn State Machine

Approved top-level states:

- `Ready`
- `StreamingLlm`
- `WaitingTools`
- `WaitingForPermission`
- `Completed`
- `Cancelled`
- `Failed`

Rules:

- only one active LLM invocation may exist at a time
- one LLM step may emit zero or one tool batch
- tool calls in a batch are collected first, then scheduled
- each tool completion is forwarded to the frontend immediately
- `FinishStep` is emitted only after the full batch reaches terminal states
- `Cancelled` preserves already-produced text and completed tool results but never resumes LLM

Recommended execution shape:

```rust
loop {
    match state {
        TurnState::Ready(ctx) => start_llm_step(ctx),
        TurnState::StreamingLlm(step) => consume_llm_stream(step),
        TurnState::WaitingTools(batch) => wait_for_batch(batch),
        TurnState::WaitingForPermission(pause) => wait_for_permission(pause),
        TurnState::Completed(_) | TurnState::Cancelled(_) | TurnState::Failed(_) => break,
    }
}
```

## 7. Core Runtime Types

Recommended backbone types:

```rust
struct TurnDriver {
    turn_id: TurnId,
    session_id: SessionId,
    command_rx: tokio::sync::mpsc::Receiver<TurnCommand>,
    event_tx: tokio::sync::mpsc::Sender<TurnEvent>,
    state: TurnState,
    cancel_token: CancellationToken,
}

enum TurnState {
    Ready(TurnContext),
    StreamingLlm(ActiveLlmStep),
    WaitingTools(ToolBatch),
    WaitingForPermission(PermissionPause),
    Completed(TurnSummary),
    Cancelled(TurnSummary),
    Failed(TurnFailure),
}

struct ActiveLlmStep {
    step_index: u32,
    invocation_id: String,
    request: LlmRequestSnapshot,
    stream: ResponseStream,
    cancel: CancellationToken,
    tool_calls: Vec<ToolCall>,
    usage: Option<Usage>,
    finish_reason: Option<LlmFinishReason>,
}

struct ToolBatch {
    batch_id: String,
    step_index: u32,
    calls: Vec<ToolExecution>,
    waiting_permission: Vec<PermissionGate>,
    in_flight: usize,
}

struct PermissionPause {
    batch: ToolBatch,
    pending_requests: Vec<PendingPermission>,
}
```

Interpretation:

- `ActiveLlmStep` owns only the current model invocation and its local buffers
- `ToolBatch` owns execution state for the current step's tools
- `PermissionPause` is a recoverable batch-level suspension, not a failed turn

## 8. Internal Event Model

`Turn` should emit internal events before any transport-specific encoding:

```rust
enum TurnEvent {
    TurnStarted,
    LlmTextDelta { text: String },
    LlmReasoningDelta { text: String },
    ToolCallPrepared { call: ToolCall },
    ToolCallDispatched { call_id: String },
    ToolCallCompleted { call_id: String, result: ToolOutcome },
    ToolCallPermissionRequested { request: PermissionRequest },
    ToolCallPermissionResolved { request_id: String, decision: PermissionDecision },
    StepFinished { step_index: u32, reason: StepFinishReason },
    TurnFinished { reason: TurnFinishReason },
}
```

`ToolOutcome` must distinguish:

- `Success(Value)`
- `Failed { message, retryable }`
- `TimedOut`
- `Denied`
- `Cancelled`

This keeps tool-level failures from collapsing into one ambiguous string.

## 9. Vercel Transport Mapping

The desktop client is pinned to `ai@6` and expects the current UI Message Stream v1 transport, not the older prefix-coded data stream.

`TurnEvent` should therefore be adapted into SSE JSON chunks shaped like:

- `TurnStarted` -> `{"type":"start"}` followed by `{"type":"start-step"}`
- `LlmTextDelta` -> `text-start`, `text-delta`, and `text-end` around each active text part
- `LlmReasoningDelta` -> `reasoning-start`, `reasoning-delta`, and `reasoning-end`
- `ToolCallPrepared` -> `{"type":"tool-input-available", ...}`
- `ToolCallPermissionRequested` -> `{"type":"tool-approval-request", ...}`
- `ToolCallCompleted::Success` -> `{"type":"tool-output-available", ...}`
- `ToolCallCompleted::Denied` -> `{"type":"tool-output-denied", ...}`
- `ToolCallCompleted::Failed/TimedOut/Cancelled` -> `{"type":"tool-output-error", ...}`
- `StepFinished` -> `{"type":"finish-step"}`
- `TurnFinished` -> `{"type":"finish", "finishReason": ...}` followed by SSE `[DONE]`

Control-plane details that do not have a first-class UI chunk should travel as transient `data-*` chunks rather than as fake tool or error events.

Example:

```json
data: {"type":"tool-approval-request","approvalId":"perm_1","toolCallId":"call_123"}

data: {"type":"data-turn-control","data":{"kind":"permission-resolved","requestId":"perm_1","decision":"allow"},"transient":true}

data: [DONE]
```

## 10. Cancellation Semantics

User interruption is approved with these semantics:

- the turn is marked `cancelled`
- already-emitted text remains visible
- already-completed tool results remain visible
- the current LLM stream is best-effort cancelled
- in-flight tool execution is best-effort cancelled
- the runtime may drain late-arriving tail events from already-started work
- no new LLM invocation starts after cancellation

This is intentionally cooperative cancellation, not hard rollback.

## 11. Permission Semantics

When a tool needs user approval:

1. `ToolAuthorizer` returns `Ask`
2. the tool is not executed yet
3. `Turn` emits a permission request event immediately
4. `Turn` enters `WaitingForPermission`
5. already-running tools in the same batch may continue to completion
6. the turn resumes in the same batch after `Allow` or `Deny`

Result handling:

- `Allow` -> tool goes back to pending dispatch
- `Deny` -> tool completes immediately as `Denied`
- once the full batch reaches terminal states, LLM may resume

## 12. Hook Surfaces

Hooks are approved, but they must be separated by responsibility:

- `TurnObserver`
  - observe events only
  - logging, persistence, tracing, metrics, UI fan-out
- `ToolAuthorizer`
  - decides `Allow`, `Deny`, or `Ask`
  - only permission control lives here
- `TurnControl`
  - external input plane
  - at minimum supports `cancel()` and `resolve_permission()`

Do not collapse these into a single catch-all hook trait.

## 13. Session Boundary

`Session` stays intentionally small in this round:

- stores metadata
- stores transcript
- stores turn terminal status
- stores permission audit history
- stores partial assistant output and completed tool results for replay

`Session` does not own:

- active LLM streams
- active tool batches
- pending permission gates
- MCP pools
- turn orchestration logic

The recommended persistence model is append-only event recording through a `SessionRecorder` observer.

## 14. Tool Runtime Boundary

`tool` remains the executor layer, but the current interface is too thin for the new turn runtime:

- `ToolContext` currently carries only `session_id` and `turn_id`
- the runtime likely needs additional execution context such as cancellation and correlation metadata
- permission policy should remain outside `tool` for now, in `turn`

Therefore the implementation plan should add a minimal runtime-aware execution context while keeping `tool` focused on execution and scheduling.

## 15. Non-Goals for v1

This round explicitly does not include:

- automatic tool retries
- automatic LLM retry of previous steps
- speculative cross-batch execution
- session-level crash recovery of a half-finished turn
- nested permission suspension layers
- full desktop/Tauri integration

## 16. Test Matrix

Minimum required scenarios:

- text-only turn completes normally
- single builtin tool succeeds
- multiple tools complete out of order but stream per-tool results immediately
- permission request then allow resumes the same turn
- permission request then deny produces `Denied` and continues
- tool timeout does not fail the whole turn
- tool execution error does not fail the whole turn
- user cancels during LLM streaming
- user cancels during tool execution
- malformed provider stream fails the turn
- UI message stream adapter emits valid SSE chunks for `start`, tool approval, tool output, `finish-step`, `finish`, and `[DONE]`
- session replay can reconstruct partial output for cancelled turns
