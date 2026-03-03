# Agent Turn Backend-Owned Retry Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement backend-owned retry/timeout/error-classification for agent turns, while keeping frontend strictly display-only for retry/failure state.

**Architecture:** Keep retry orchestration in `agent-turn` reducer/effects as the single source of truth. Extend backend command surface for manual retry orchestration (`restore + rerun`) and emit stream events that frontend passively projects. Frontend may trigger commands but must never compute retry policy, backoff, or error classification.

**Tech Stack:** Rust workspace (`agent-turn`, `llm-client`, `agent-session`, `desktop`), Tauri commands/events, Next.js + Zustand + Vitest (`desktop`).

---

## Skills To Apply During Execution

- `@m07-concurrency` for timeout/supervision logic and async cancellation boundaries.
- `@m13-domain-error` for transient/permanent error boundaries and retry semantics.
- `@m15-anti-pattern` to prevent retry logic drifting into frontend.
- `@verification-before-completion` before any “done/passing” claim.

## Global Constraints

- DRY: reuse existing `reduce`/`RuntimeEvent`/`RunStreamEvent` pipeline; do not introduce a second retry state machine.
- YAGNI: do not add new event enums unless existing run/ui events are insufficient.
- TDD: every behavior change starts with a failing test.
- Frequent commits: one commit per task, staged by explicit file paths only.

---

### Task 1: Lock Backend Retry Defaults to 5 Attempts

**Files:**
- Modify: `agent-turn/src/state.rs`
- Modify: `agent-turn/src/reducer.rs`
- Test: `agent-turn/src/state.rs` (new unit test module)
- Test: `agent-turn/src/reducer.rs` (existing reducer tests)

**Step 1: Write the failing test (default retries must be 5)**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_policy_defaults_to_five_attempts() {
        let cfg = TurnEngineConfig::default();
        assert_eq!(cfg.retry_policy.max_retries, 5);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn retry_policy_defaults_to_five_attempts -- --exact`
Expected: FAIL with `left: 3, right: 5`.

**Step 3: Write minimal implementation**

```rust
impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 5,
            base_delay_ms: 200,
        }
    }
}
```

**Step 4: Update reducer boundary test for exhaustion at 5**

```rust
#[test]
fn transient_error_fails_when_exhausted() {
    let state = StateBuilder::new("s1", "t1")
        .with_lifecycle(Lifecycle::Active)
        .with_model_state(ModelState::Streaming)
        .with_retry_attempt(5)
        .build();

    let result = reduce(
        state,
        EventBuilder::transient_error("timeout").with_epoch(0).build(),
        &test_config(),
    );

    assert_eq!(result.state.lifecycle, Lifecycle::Failed);
}
```

**Step 5: Run targeted tests to verify pass**

Run: `cargo test -p agent-turn retry_policy_defaults_to_five_attempts transient_error_fails_when_exhausted`
Expected: PASS.

**Step 6: Commit**

```bash
git add agent-turn/src/state.rs agent-turn/src/reducer.rs
git commit -m "feat(agent-turn): set backend retry default to 5 attempts"
```

---

### Task 2: Add Explicit LLM Error Classification Helpers

**Files:**
- Modify: `llm-client/src/error.rs`
- Test: `llm-client/src/error.rs`

**Step 1: Write failing classification tests**

```rust
#[test]
fn classification_marks_auth_as_permanent() {
    let err = LlmError::AuthError { message: "bad key".into() };
    assert!(err.is_permanent());
    assert!(!err.is_transient());
}

#[test]
fn classification_marks_rate_limit_as_transient_with_backoff() {
    let err = LlmError::RateLimit {
        message: "busy".into(),
        retry_after: Some(Duration::from_secs(3)),
    };
    assert!(err.is_transient());
    assert!(err.requires_backoff());
}

#[test]
fn classification_marks_invalid_request_as_permanent() {
    let err = LlmError::InvalidRequest { message: "bad".into() };
    assert!(err.is_permanent());
    assert!(!err.requires_backoff());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p llm-client classification_marks_`
Expected: FAIL due to missing methods.

**Step 3: Write minimal implementation**

```rust
impl LlmError {
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::RateLimit { .. }
                | Self::ServerError { status: 500..=599, .. }
                | Self::NetworkError { .. }
                | Self::Timeout
                | Self::StreamIdleTimeout
        )
    }

    pub fn requires_backoff(&self) -> bool {
        matches!(self, Self::RateLimit { .. } | Self::ServerError { status: 503, .. })
    }

    pub fn is_permanent(&self) -> bool {
        matches!(
            self,
            Self::AuthError { .. }
                | Self::InvalidRequest { .. }
                | Self::ContextOverflow { .. }
                | Self::QuotaExceeded { .. }
                | Self::ParseError { .. }
        )
    }
}
```

**Step 4: Run tests to verify pass**

Run: `cargo test -p llm-client classification_marks_`
Expected: PASS.

**Step 5: Commit**

```bash
git add llm-client/src/error.rs
git commit -m "feat(llm-client): add explicit transient/permanent classification helpers"
```

---

### Task 3: Enforce Tool Timeout in Effect Executor and Route as Transient Error

**Files:**
- Modify: `agent-turn/src/effect.rs`
- Test: `agent-turn/src/effect.rs`

**Step 1: Write failing test for tool timeout -> transient retry signal**

```rust
#[tokio::test]
async fn tool_timeout_emits_transient_error() {
    // test tool spec should set timeout_ms = Some(5)
    // execute_tool sleeps 50ms
    // expect RuntimeEvent::TransientError is emitted
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn tool_timeout_emits_transient_error -- --exact`
Expected: FAIL (no transient error emitted yet).

**Step 3: Implement timeout-aware execution path**

```rust
let spec = tools.tool_spec(&call.tool_name).await;
let mode = spec
    .as_ref()
    .map(|s| s.execution_policy.parallel_mode.clone())
    .unwrap_or(ToolParallelMode::ParallelSafe);
let timeout_ms = spec
    .as_ref()
    .and_then(|s| s.execution_policy.timeout_ms)
    .unwrap_or(300_000);

let exec_future = async {
    if matches!(mode, ToolParallelMode::Exclusive) {
        // existing exclusive lock branch
    } else {
        tools.execute_tool(call.clone(), ctx).await
    }
};

match tokio::time::timeout(Duration::from_millis(timeout_ms), exec_future).await {
    Ok(result) => { /* existing Ok/Err handling */ }
    Err(_) => {
        let _ = tx.send(RuntimeEvent::TransientError {
            event_id: new_id(),
            epoch,
            message: format!("tool {} timed out after {}ms", call.tool_name, timeout_ms),
            retry_after_ms: None,
        });
        return;
    }
}
```

**Step 4: Run tests to verify pass + no regressions in executor tests**

Run: `cargo test -p agent-turn tool_timeout_emits_transient_error tool_execution_respects_parallel_limit exclusive_tools_are_serialized_even_with_parallel_slots`
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-turn/src/effect.rs
git commit -m "feat(agent-turn): enforce tool timeout and emit transient retry signal"
```

---

### Task 4: Add Backend `retry_failed_turn` Command (Restore + Rerun)

**Files:**
- Modify: `desktop/src-tauri/src/lib.rs`
- Test: `desktop/src-tauri/src/lib.rs`

**Step 1: Write failing test for retry request builder helper**

```rust
#[test]
fn build_retry_turn_request_uses_given_provider_model_and_input() {
    let req = build_retry_turn_request(
        "backend-session-1".into(),
        "bigmodel".into(),
        "glm-5".into(),
        "retry this".into(),
    );

    assert_eq!(req.provider, "bigmodel");
    assert_eq!(req.model, "glm-5");
    // verify initial input is user text
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p desktop build_retry_turn_request_uses_given_provider_model_and_input -- --exact`
Expected: FAIL (helper not implemented).

**Step 3: Implement payload, helper, and command**

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RetryFailedTurnPayload {
    session_id: String,
    turn_id: String,
    input: String,
    provider: ProviderId,
    model: String,
}

fn build_retry_turn_request(
    backend_session_id: String,
    provider: String,
    model: String,
    input: String,
) -> TurnRequest {
    TurnRequest {
        meta: SessionMeta::new(backend_session_id, new_id()),
        provider,
        model,
        initial_input: InputEnvelope::user_text(input),
        transcript: Vec::new(),
    }
}

#[tauri::command]
async fn retry_failed_turn(
    app: AppHandle,
    state: State<'_, AppState>,
    payload: RetryFailedTurnPayload,
) -> Result<StartAgentTurnResponse, String> {
    let backend_session_id = ensure_backend_session_id(&state, &payload.session_id).await?;
    state
        .runtime
        .restore_to_turn(&backend_session_id, &payload.turn_id)
        .await
        .map_err(|err| format!("failed to restore checkpoint {}: {err}", payload.turn_id))?;

    let request = build_retry_turn_request(
        backend_session_id,
        payload.provider.as_adapter_id().to_string(),
        payload.model,
        payload.input,
    );
    let turn_id = request.meta.turn_id.clone();
    let streams = state
        .runtime
        .run_turn(request)
        .await
        .map_err(|err| format!("failed to run retry turn: {err}"))?;

    spawn_stream_forwarders(app, payload.session_id, turn_id.clone(), streams.run, streams.ui);
    Ok(StartAgentTurnResponse { turn_id })
}
```

Also register command in `invoke_handler`.

**Step 4: Run tests to verify pass**

Run: `cargo test -p desktop build_retry_turn_request_uses_given_provider_model_and_input`
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop-tauri): add retry_failed_turn command for restore+rerun"
```

---

### Task 5: Add Frontend API Contract for `retry_failed_turn`

**Files:**
- Modify: `desktop/lib/api/chat.ts`
- Modify: `desktop/lib/api/chat.test.ts`

**Step 1: Write failing API payload test**

```ts
it("uses camelCase payload for retryFailedTurn", async () => {
  invokeMock.mockResolvedValueOnce({ turnId: "t2" });

  await retryFailedTurn({
    sessionId: "s1",
    turnId: "t1",
    input: "retry me",
    provider: "bigmodel",
    model: "glm-5",
  });

  expect(invokeMock).toHaveBeenCalledWith("retry_failed_turn", {
    payload: {
      sessionId: "s1",
      turnId: "t1",
      input: "retry me",
      provider: "bigmodel",
      model: "glm-5",
    },
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm --dir desktop test -- lib/api/chat.test.ts`
Expected: FAIL (`retryFailedTurn` not found / wrong payload).

**Step 3: Implement API types and function**

```ts
export interface RetryFailedTurnPayload {
  sessionId: string;
  turnId: string;
  input: string;
  provider: ProviderId;
  model: string;
}

export async function retryFailedTurn(
  payload: RetryFailedTurnPayload
): Promise<StartAgentTurnResponse> {
  return await invoke("retry_failed_turn", { payload });
}
```

**Step 4: Run test to verify pass**

Run: `pnpm --dir desktop test -- lib/api/chat.test.ts`
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/lib/api/chat.ts desktop/lib/api/chat.test.ts
git commit -m "feat(desktop-api): add retry_failed_turn client contract"
```

---

### Task 6: Keep Frontend Store Display-Only and Add Retry Projection

**Files:**
- Modify: `desktop/lib/stores/chat-store.ts`
- Test: `desktop/lib/stores/chat-store.retry.test.ts` (new)

**Step 1: Write failing store test for passive retry projection**

```ts
it("projects retrying event to retryState without computing policy", () => {
  const store = useChatStore.getState();
  store.ensureAgentTurn("s1", "t1");

  store.applyAgentStreamEnvelope({
    sessionId: "s1",
    turnId: "t1",
    source: "run",
    seq: 1,
    ts: Date.now(),
    event: { type: "retrying", attempt: 2, delay_ms: 400, next_epoch: 3 },
  });

  const turn = useChatStore.getState().turns["s1"][0];
  expect(turn.retryState?.attempt).toBe(2);
  expect(turn.retryState?.maxAttempts).toBe(5);
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm --dir desktop test -- lib/stores/chat-store.retry.test.ts`
Expected: FAIL (`retryState` absent / no retrying handler).

**Step 3: Implement minimal store shape + event mapping by `event.type` only**

```ts
type RetryState = {
  isRetrying: boolean;
  attempt: number;
  maxAttempts: number;
  lastError?: string;
};

interface AgentTurnVM {
  // existing fields...
  retryState?: RetryState;
}

if (eventType === "retrying") {
  turn.retryState = {
    isRetrying: true,
    attempt: Number(event.attempt ?? 1),
    maxAttempts: 5,
    lastError: turn.retryState?.lastError,
  };
}
if (eventType === "transient_error") {
  turn.retryState = {
    isRetrying: true,
    attempt: turn.retryState?.attempt ?? 1,
    maxAttempts: turn.retryState?.maxAttempts ?? 5,
    lastError: String(event.message ?? "transient error"),
  };
}
if (eventType === "turn_failed" || eventType === "error") {
  if (turn.retryState) {
    turn.retryState = { ...turn.retryState, isRetrying: false };
  }
}
```

No `shouldRetry`, no timer scheduling, no backoff computation in frontend.

**Step 4: Run tests to verify pass**

Run: `pnpm --dir desktop test -- lib/stores/chat-store.retry.test.ts`
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/lib/stores/chat-store.ts desktop/lib/stores/chat-store.retry.test.ts
git commit -m "feat(chat-store): project backend retry events in display-only mode"
```

---

### Task 7: Add Display Components and Wire Manual Retry Action

**Files:**
- Create: `desktop/components/features/chat/thinking-indicator.tsx`
- Create: `desktop/components/features/chat/turn-error-banner.tsx`
- Modify: `desktop/components/features/chat/agent-turn-card.tsx`
- Modify: `desktop/components/features/chat/chat-prompt-input.tsx`
- Modify: `desktop/lib/stores/chat-store.ts`
- Test: `desktop/components/features/chat/agent-turn-card.test.tsx` (new)

**Step 1: Write failing component test for retry UI**

```tsx
it("shows retry counter when turn.retryState.isRetrying is true", () => {
  render(<AgentTurnCard sessionId="s1" turn={turnWithRetryingState} />);
  expect(screen.getByText("Agent is analyzing...")).toBeInTheDocument();
  expect(screen.getByText("Retrying... (2/5)")).toBeInTheDocument();
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm --dir desktop test -- components/features/chat/agent-turn-card.test.tsx`
Expected: FAIL (components not found / text missing).

**Step 3: Implement `ThinkingIndicator` and `TurnErrorBanner`**

```tsx
export function ThinkingIndicator({ retryState }: { retryState?: RetryState }) {
  return (
    <div className="flex items-center gap-2 text-muted-foreground text-sm">
      <Shimmer>Agent is analyzing...</Shimmer>
      {retryState?.isRetrying ? (
        <span className="text-xs">Retrying... ({retryState.attempt}/{retryState.maxAttempts})</span>
      ) : null}
    </div>
  );
}
```

```tsx
export function TurnErrorBanner({ error, onRetry }: { error: string; onRetry: () => void }) {
  return (
    <Alert variant="destructive">
      <AlertDescription className="flex items-center justify-between">
        <span>{error}</span>
        <Button size="sm" variant="outline" onClick={onRetry}>Retry from checkpoint</Button>
      </AlertDescription>
    </Alert>
  );
}
```

**Step 4: Wire manual retry button to backend command (no local retry logic)**

```tsx
const handleRetry = async () => {
  const retryInput = findOriginalUserInput(turn.requestMessageId, sessionMessages);
  if (!retryInput || !selectedModel) return;
  const { turnId } = await retryFailedTurn({
    sessionId,
    turnId: turn.id,
    input: retryInput,
    provider: selectedModel.provider,
    model: selectedModel.model,
  });
  ensureAgentTurn(sessionId, turnId, turn.requestMessageId);
};
```

This is command orchestration only; retry policy remains backend-owned.

**Step 5: Run tests to verify pass**

Run: `pnpm --dir desktop test -- components/features/chat/agent-turn-card.test.tsx`
Expected: PASS.

**Step 6: Commit**

```bash
git add desktop/components/features/chat/thinking-indicator.tsx \
  desktop/components/features/chat/turn-error-banner.tsx \
  desktop/components/features/chat/agent-turn-card.tsx \
  desktop/components/features/chat/chat-prompt-input.tsx \
  desktop/components/features/chat/agent-turn-card.test.tsx \
  desktop/lib/stores/chat-store.ts
git commit -m "feat(chat-ui): add display-only retry indicator and manual retry banner"
```

---

### Task 8: End-to-End Verification and Final Integration Commit

**Files:**
- Modify (if needed): `docs/plans/2026-03-03-agent-turn-enhancement-design.md`

**Step 1: Run backend test suite for touched crates**

Run:

```bash
cargo test -p llm-client
cargo test -p agent-turn
cargo test -p desktop
```

Expected: PASS for all touched crates.

**Step 2: Run frontend targeted tests**

Run:

```bash
pnpm --dir desktop test -- lib/api/chat.test.ts
pnpm --dir desktop test -- lib/stores/chat-store.retry.test.ts
pnpm --dir desktop test -- components/features/chat/agent-turn-card.test.tsx
```

Expected: PASS.

**Step 3: Run frontend full test sweep**

Run: `pnpm --dir desktop test`
Expected: PASS.

**Step 4: Manual smoke verification**

1. Start a chat turn and verify “Agent is analyzing...” appears.
2. Force transient failure (network/rate-limit) and verify “Retrying... (N/5)” increments.
3. Force permanent failure and verify immediate error banner without auto-retry.
4. Click manual retry and verify backend starts a new turn stream.

Expected: behavior matches design boundaries.

**Step 5: Final commit**

```bash
git add agent-turn/src/state.rs agent-turn/src/reducer.rs agent-turn/src/effect.rs \
  llm-client/src/error.rs desktop/src-tauri/src/lib.rs \
  desktop/lib/api/chat.ts desktop/lib/api/chat.test.ts \
  desktop/lib/stores/chat-store.ts desktop/lib/stores/chat-store.retry.test.ts \
  desktop/components/features/chat/thinking-indicator.tsx \
  desktop/components/features/chat/turn-error-banner.tsx \
  desktop/components/features/chat/agent-turn-card.tsx \
  desktop/components/features/chat/agent-turn-card.test.tsx \
  desktop/components/features/chat/chat-prompt-input.tsx

git commit -m "feat(agent-turn): backend-owned retry flow with frontend display-only projection"
```

---

## Done Criteria Checklist

- [ ] Retry policy defaults to 5 in backend config.
- [ ] Frontend has no `shouldRetry`/backoff/timer retry decision logic.
- [ ] Tool timeout path emits backend retry signal (transient) and is tested.
- [ ] `retry_failed_turn` backend command is available and tested.
- [ ] Frontend consumes `event.type` for retrying/failed display state.
- [ ] Manual retry UI calls backend command only.
- [ ] All listed Rust + Vitest commands pass.

