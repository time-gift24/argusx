# Agent-Turn Enhancement Design

**Date:** 2026-03-03
**Status:** Approved
**Author:** Claude (with user requirements)

## Overview

This design enforces a hard responsibility boundary:

- **Backend is the single authority** for retry/error classification/timeout/checkpoint recovery.
- **Frontend is display-only** for turn progress, retry counters, and final error states.

The system will auto-retry transient failures up to 5 attempts, apply backoff when needed, and expose state through stream events that the frontend only renders.

## Summary of Requirements

1. **Single Authority Rule**: Retry decisions, backoff, timeout handling, and checkpoint restore must be backend-owned.
2. **Visual Feedback**: Show `Shimmer` text "Agent is analyzing..." while a turn is in progress.
3. **Retry Counter**: Show "Retrying... (N/5)" based only on backend stream events.
4. **Smart Retry**: Auto-retry transient errors (timeout/network/server unavailable); rate-limit errors use backoff.
5. **Permanent Errors**: No auto-retry for auth/invalid request/context overflow/quota and similar non-transient classes.
6. **Graceful Degradation**: After retry budget is exhausted, mark turn as failed and provide a manual retry action.
7. **Timeout Supervision**: Backend enforces multi-level timeout budget (10min turn, tool timeout default 5min with per-tool override, existing 120s LLM request timeout).

## Architecture

### Responsibility Boundary (Hard Rule)

**Backend owns:**
- error classification (`transient` vs `permanent`)
- retry eligibility and attempt counting
- backoff delay calculation
- checkpoint save/restore flow
- timeout supervision and cancellation
- authoritative turn status transitions (`started/streaming/done/failed/cancelled`)

**Frontend owns:**
- render-only projection of backend events
- user-initiated commands (`start`, `cancel`, `manual retry`)

**Frontend must not do:**
- `shouldRetry` decisions
- backoff calculation
- local retry loops
- error re-classification

### Error Categorization Hierarchy

```
Transient (retry-eligible)
├─ Timeout
├─ StreamIdleTimeout
├─ NetworkError
├─ RateLimit
└─ ServerError(5xx)

Permanent (no auto-retry)
├─ AuthError
├─ InvalidRequest
├─ ContextOverflow
├─ QuotaExceeded
└─ ParseError / protocol-invalid responses

Timeout supervision (backend-only)
├─ TurnSupervisor (10 min per turn)
├─ Tool execution timeout (default 5 min; optional per-tool timeout_ms)
└─ LLM request timeout (existing 120s)
```

### Component Layers

```
┌──────────────────────────────────────────────────────────┐
│ Frontend (Display Layer)                                │
│ Shimmer | RetryCounter | ErrorBanner | RetryButton      │
│ (no retry policy, no timeout logic, no classification)  │
└──────────────────────────────────────────────────────────┘
                          ↕ agent:stream
┌──────────────────────────────────────────────────────────┐
│ Tauri Backend (Command + Stream Bridge)                 │
│ start/cancel/retry_failed_turn + turn supervision hook  │
└──────────────────────────────────────────────────────────┘
                          ↕
┌──────────────────────────────────────────────────────────┐
│ Agent Runtime (Authoritative State Machine)             │
│ TransientError -> Retrying -> RetryTimerFired -> retry  │
│ FatalError/RetryExhausted -> TurnFailed                 │
└──────────────────────────────────────────────────────────┘
```

## Backend Modifications

### A. Retry Orchestration Remains in Agent Runtime (Single Source of Truth)

Use the existing reducer-driven flow in `agent-turn` as the only retry state machine:

```rust
// agent-turn/src/reducer.rs (existing flow, tuned to 5 retries)
RuntimeEvent::TransientError { .. } => {
    let attempt = state.retry_attempt + 1;
    let can_retry = attempt <= config.retry_policy.max_retries; // set to 5
    if can_retry {
        emit RunStreamEvent::Retrying { attempt, next_epoch, delay_ms };
        schedule Effect::ScheduleRetry { delay_ms, next_epoch };
    } else {
        fail_turn(...); // emits TurnFailed
    }
}
```

Implementation note:
- Tune `TurnEngineConfig.retry_policy.max_retries` from current default to **5**.
- Do not add a second retry orchestrator in frontend or outside this state machine.

### B. TurnSupervisor (Backend)

Location: `desktop/src-tauri/src/lib.rs` (or session runtime hook).

Responsibilities:
- start supervisor timer when a turn starts (10 min budget)
- if timed out, issue backend cancellation and finalize turn as failed
- ensure cleanup path clears active-turn markers

### C. Tool Execution Timeout (Backend)

Location: `agent-turn/src/effect.rs`.

Use per-tool timeout when provided, otherwise default to 5 minutes:

```rust
let timeout_ms = spec.execution_policy.timeout_ms.unwrap_or(300_000);
let result = tokio::time::timeout(
    Duration::from_millis(timeout_ms),
    tools.execute_tool(call.clone(), ctx),
).await;
```

Timeout outcome policy:
- timeout events are handled in backend and mapped to runtime error flow
- frontend only receives resulting run/ui events

### D. Error Classification API (Backend)

Location: `llm-client/src/error.rs` and adapter mapping in `agent-turn`.

Add classification helpers aligned to **existing enum names** (`AuthError`, `Timeout`, `RateLimit`, etc.), for example:

```rust
impl LlmError {
    pub fn is_transient(&self) -> bool { /* aligned to existing variants */ }
    pub fn requires_backoff(&self) -> bool { /* mainly RateLimit/503-like */ }
    pub fn is_permanent(&self) -> bool { /* auth/invalid/context/quota/... */ }
}
```

No introduction of non-existent variants in this design.

### E. Manual Retry Command (Backend)

Add `retry_failed_turn` Tauri command in `desktop/src-tauri/src/lib.rs`:
- restore checkpoint for target turn
- start a new turn from restored state
- return new `turn_id`

Manual retry is user-triggered; all policy stays backend-owned.

### F. Event Contract

Prefer existing event contract:
- `RunStreamEvent::Retrying`
- `RunStreamEvent::TransientError`
- `RunStreamEvent::TurnFailed`
- `UiThreadEvent::Error` / `UiThreadEvent::Warning`

Frontend must consume by `event.type`, not by transport `source`.

## Frontend Components (Display-Only)

### A. ThinkingIndicator
`desktop/components/features/chat/thinking-indicator.tsx` (new file)

Renders:
- shimmer while turn is active
- retry counter from backend-provided attempt/max values

### B. TurnErrorBanner
`desktop/components/features/chat/turn-error-banner.tsx` (new file)

Renders failed state and manual retry button.
Button calls backend command API; no local retry logic.

### C. AgentTurnCard View Model Extension
`desktop/components/features/chat/agent-turn-card.tsx`

```tsx
type RetryState = {
  isRetrying: boolean;
  attempt: number;
  maxAttempts: number;
  lastError?: string;
};
```

### D. Store Mapping (Read-Only Projection)
`desktop/lib/stores/chat-store.ts`

```typescript
applyAgentStreamEnvelope: (envelope) => {
  const eventType = String(envelope.event?.type ?? "");
  switch (eventType) {
    case "retrying":
      // update retryState for UI display only
      break;
    case "transient_error":
      // display warning/error context only
      break;
    case "turn_failed":
    case "error":
      // mark failed based on backend terminal event
      break;
  }
};
```

Rule: store only mirrors backend state; it does not run retry policy.

## Data Flow

### Normal Flow

```
User sends message
  ↓
Frontend calls start_agent_turn()
  ↓
Backend runtime executes turn and emits run/ui events
  ↓
Frontend renders shimmer/progress
  ↓
TurnDone -> frontend renders final output/checkpoint
```

### Transient Error + Auto-Retry (Backend Controlled)

```
Runtime gets transient error
  ↓
Reducer checks retry budget/backoff
  ↓
Emit Retrying(attempt N/5, delay)
  ↓
Backend schedules RetryTimerFired and re-runs model/tool path
  ↓
Frontend only renders retrying indicator from stream
```

### Retry Exhausted or Permanent Error

```
Runtime determines no retry (permanent or exhausted budget)
  ↓
Emit TurnFailed / Ui Error
  ↓
Frontend renders error banner + manual retry button
```

### Manual Retry Flow

```
User clicks "Retry from checkpoint"
  ↓
Frontend calls retry_failed_turn(sessionId, turnId)
  ↓
Backend restores checkpoint and creates new turn
  ↓
Frontend subscribes/render new turn stream
```

## Testing Strategy

### Backend Unit Tests

- classification correctness: `is_transient/is_permanent/requires_backoff`
- retry budget logic: max attempts = 5
- backoff logic and cap behavior
- tool timeout fallback and per-tool override handling

### Backend Integration Tests

- transient error triggers retry and emits `retrying`
- exhausted retries end with `turn_failed`
- permanent errors bypass retry
- turn supervisor timeout leads to deterministic terminal state and cleanup

### Frontend Tests

- `ThinkingIndicator` renders retry counter from incoming events
- `TurnErrorBanner` renders error and triggers backend command
- store projection updates state by `event.type`
- verify no frontend retry decision helpers exist (`shouldRetry`, `backoff`, timers)

### Manual Scenarios

| Scenario | Expected Result |
|----------|-----------------|
| Normal flow | Shimmer -> Done |
| Network timeout | Retrying (N/5) shown, then success/fail |
| 429 rate limit | Retrying with visible attempt increment |
| Auth failure | Immediate failed banner, no auto-retry |
| Tool timeout | Backend handles policy, frontend reflects event |
| Turn > 10 min | Supervisor terminal handling, no hanging UI |
| Retry exhausted | Failed banner + manual retry button |

## Files to Modify

### Backend (Rust)
- `agent-turn/src/state.rs` - retry policy max attempts update
- `agent-turn/src/reducer.rs` - authoritative retry/failure transitions
- `agent-turn/src/effect.rs` - tool timeout enforcement
- `agent-turn/src/adapters/bigmodel.rs` - error mapping alignment
- `llm-client/src/error.rs` - classification helpers aligned to existing variants
- `desktop/src-tauri/src/lib.rs` - `retry_failed_turn` + turn supervision wiring
- `agent-core/src/events.rs` (optional) - only if extra run events are strictly needed

### Frontend (TypeScript/React)
- `desktop/components/features/chat/thinking-indicator.tsx` - new
- `desktop/components/features/chat/turn-error-banner.tsx` - new
- `desktop/components/features/chat/agent-turn-card.tsx` - render enhancements
- `desktop/lib/stores/chat-store.ts` - display-only event projection
- `desktop/lib/api/chat.ts` - `retry_failed_turn` API

## Implementation Priority

1. Backend retry ownership hardening (single authority, max retries = 5)
2. Backend timeout supervision (turn + tool)
3. Backend manual retry command (`retry_failed_turn`)
4. Frontend display components and passive state projection
5. Test matrix (backend-first, then frontend display tests)

## Success Criteria

- ✅ Backend is the only layer deciding retry/backoff/failure/timeout behavior
- ✅ Frontend only renders stream state and triggers explicit commands
- ✅ Transient errors auto-retry up to 5 attempts
- ✅ Retry progress "Retrying... (N/5)" is visible in UI
- ✅ Permanent errors fail fast with user-friendly messaging
- ✅ Manual retry from checkpoint works after terminal failure
- ✅ No hanging turns after timeout supervision
