# Turn Shared-State Refactor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace stale/cloned turn runtime snapshots with shared read-only data and single-owner mutable state while preserving tool ordering guarantees.

**Architecture:** `TurnDriver` keeps mutable step state in `TurnState`; transcript/model/event payloads use `Arc`-backed immutable snapshots. Tool execution remains concurrent, but transcript replay stays aligned with the original tool-call array order.

**Tech Stack:** Rust 2024 workspace, `tokio`, `async_trait`, `Arc`, unit/integration tests in `turn/tests`

---

### Task 1: Add red tests for the new public shapes

**Files:**
- Modify: `turn/tests/transcript_turn_test.rs`
- Modify: `turn/tests/tool_batch_turn_test.rs`
- Modify: `turn/tests/vercel_adapter_test.rs`

**Step 1: Write the failing tests**

- Add assertions that `LlmStepRequest.messages` is `Arc<[TurnMessage]>` and still preserves prior-step tool-result ordering.
- Add assertions that `TurnEvent::LlmTextDelta` and `TurnEvent::LlmReasoningDelta` carry `Arc<str>`.
- Add a regression test where tool completions happen in reverse completion order while the second-step transcript still reflects original tool-call order.

**Step 2: Run test to verify it fails**

Run: `cargo test -p turn transcript_turn_test tool_batch_turn_test vercel_adapter_test`

Expected: compile or assertion failures because the public types still use owned `Vec`/`String`.

**Step 3: Commit**

```bash
git add turn/tests/transcript_turn_test.rs turn/tests/tool_batch_turn_test.rs turn/tests/vercel_adapter_test.rs
git commit -m "test(turn): capture shared-state refactor expectations"
```

### Task 2: Refactor transcript and request snapshot types

**Files:**
- Modify: `turn/src/transcript.rs`
- Modify: `turn/src/model.rs`
- Modify: `turn/src/context.rs`
- Modify: `turn/tests/support/fake_model.rs`
- Modify: `turn/tests/transcript_turn_test.rs`

**Step 1: Write the minimal implementation**

- Convert immutable text/ID fields in `TurnMessage` to `Arc<str>`.
- Represent assistant tool-call payloads as `Arc<[Arc<ToolCall>]>`.
- Expose transcript snapshots as `Arc<[Arc<TurnMessage>]>`.
- Change `LlmStepRequest` to use shared snapshot fields.

**Step 2: Run focused tests**

Run: `cargo test -p turn transcript_turn_test`

Expected: transcript/request tests pass.

**Step 3: Commit**

```bash
git add turn/src/transcript.rs turn/src/model.rs turn/src/context.rs turn/tests/support/fake_model.rs turn/tests/transcript_turn_test.rs
git commit -m "refactor(turn): share transcript snapshots across llm steps"
```

### Task 3: Refactor events and runtime state ownership

**Files:**
- Modify: `turn/src/event.rs`
- Modify: `turn/src/state.rs`
- Modify: `turn/src/driver.rs`
- Modify: `turn/src/handle.rs`
- Modify: `turn/src/vercel.rs`
- Modify: `turn/tests/text_only_turn_test.rs`
- Modify: `turn/tests/permission_turn_test.rs`
- Modify: `turn/tests/tool_batch_turn_test.rs`
- Modify: `turn/tests/vercel_adapter_test.rs`

**Step 1: Write the minimal implementation**

- Change `TurnEvent` immutable text/ID payloads to `Arc<str>`.
- Make `TurnState::Ready` payload-free.
- Keep `ActiveLlmStep` and tool-batch state as the mutable truth inside `self.state`, eliminating the stale `active_step.clone()` pattern.
- Update `vercel` event mapping and tests to consume the new shared types.

**Step 2: Run focused tests**

Run: `cargo test -p turn text_only_turn_test permission_turn_test tool_batch_turn_test vercel_adapter_test`

Expected: event/state tests pass.

**Step 3: Commit**

```bash
git add turn/src/event.rs turn/src/state.rs turn/src/driver.rs turn/src/handle.rs turn/src/vercel.rs turn/tests/text_only_turn_test.rs turn/tests/permission_turn_test.rs turn/tests/tool_batch_turn_test.rs turn/tests/vercel_adapter_test.rs
git commit -m "refactor(turn): make runtime state the single source of truth"
```

### Task 4: Verify cancellation, timeout, and tracing regressions

**Files:**
- Modify: `turn/tests/cancel_turn_test.rs`
- Modify: `turn/tests/timeout_turn_test.rs`
- Modify: `turn/tests/tracing_turn_test.rs`
- Modify: `turn/tests/compile_smoke_test.rs`

**Step 1: Update tests as needed**

- Adapt payload assertions to `Arc<str>` and updated `TurnState`.
- Keep cancellation and timeout semantics unchanged.

**Step 2: Run focused tests**

Run: `cargo test -p turn cancel_turn_test timeout_turn_test tracing_turn_test compile_smoke_test`

Expected: all pass.

**Step 3: Commit**

```bash
git add turn/tests/cancel_turn_test.rs turn/tests/timeout_turn_test.rs turn/tests/tracing_turn_test.rs turn/tests/compile_smoke_test.rs
git commit -m "test(turn): cover shared-state runtime regressions"
```

### Task 5: Full verification

**Files:**
- Modify: none

**Step 1: Run the full crate test suite**

Run: `cargo test -p turn`

Expected: all `turn` unit and integration tests pass.

**Step 2: Run linting if time allows**

Run: `cargo clippy -p turn --tests -- -D warnings`

Expected: no warnings in touched code.

**Step 3: Commit**

```bash
git add -A
git commit -m "refactor(turn): share immutable snapshots and remove stale state copies"
```
