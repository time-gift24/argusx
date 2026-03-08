# Multi-Turn Runtime Boundary Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the `turn` crate accept prior conversation history and return a stable completed-turn artifact that a higher-level conversation manager can append to thread history.

**Architecture:** Keep `turn` focused on single-turn execution. Add a new recording-oriented API that can seed the initial transcript with prior history and return a `TurnOutcome` at the end of execution, while preserving the existing `TurnDriver::spawn()` compatibility path for current callers.

**Tech Stack:** Rust 2024 workspace, `turn` crate, `tokio`, `Arc`, integration tests under `turn/tests`

---

### Task 1: Add red tests for history seeding and completed-turn output

**Files:**
- Modify: `turn/tests/transcript_turn_test.rs`
- Modify: `turn/tests/text_only_turn_test.rs`
- Modify: `turn/tests/cancel_turn_test.rs`
- Modify: `turn/tests/support/fake_model.rs`

**Step 1: Write the failing tests**

- Add a test in `turn/tests/transcript_turn_test.rs` that starts a recording turn with pre-seeded history and asserts the first `LlmStepRequest` already contains the seeded assistant text and tool results before the new user message is appended.
- Add a test in `turn/tests/text_only_turn_test.rs` that asserts a completed text-only turn returns a `TurnOutcome::Completed` artifact whose transcript includes both the user message and the final assistant text.
- Add a test in `turn/tests/cancel_turn_test.rs` that asserts a cancelled recording turn returns `TurnOutcome::Cancelled` and does not pretend to produce a completed transcript artifact.
- Extend `turn/tests/support/fake_model.rs` only as needed to let tests inspect the first request and the final outcome.

Suggested test shape:

```rust
#[tokio::test]
async fn recording_turn_replays_seed_history_before_new_user_message() {
    let history = Arc::from([
        Arc::new(TurnMessage::AssistantText {
            content: "previous answer".into(),
        }),
    ]);

    let (handle, task) = TurnDriver::spawn_recording(
        context(),
        history,
        Arc::new(FakeModelRunner::default()),
        Arc::new(instant_tool_runner()),
        Arc::new(FakeAuthorizer::default()),
        Arc::new(FakeObserver),
    );

    collect_events(handle).await;
    let outcome = task.await.unwrap().unwrap();
    assert!(matches!(outcome, TurnOutcome::Completed(_)));
}
```

**Step 2: Run test to verify it fails**

Run:

- `cargo test -p turn transcript_turn_test -- --nocapture`
- `cargo test -p turn text_only_turn_test -- --nocapture`
- `cargo test -p turn cancel_turn_test -- --nocapture`

Expected:

- compile failures because `spawn_recording` / `TurnOutcome` do not exist yet, or assertion failures because assistant text is not captured in the completed artifact

**Step 3: Commit**

```bash
git add turn/tests/transcript_turn_test.rs \
  turn/tests/text_only_turn_test.rs \
  turn/tests/cancel_turn_test.rs \
  turn/tests/support/fake_model.rs
git commit -m "test(turn): capture multi-turn runtime boundary expectations"
```

### Task 2: Add public outcome types and transcript seeding helpers

**Files:**
- Create: `turn/src/outcome.rs`
- Modify: `turn/src/transcript.rs`
- Modify: `turn/src/lib.rs`

**Step 1: Write the minimal implementation**

Create `turn/src/outcome.rs` with the new public boundary types:

```rust
use crate::{TurnFailure, TurnFinishReason, TurnMessageSnapshot, TurnSummary};

#[derive(Debug, Clone, PartialEq)]
pub struct CompletedTurn {
    pub turn_id: String,
    pub transcript: TurnMessageSnapshot,
    pub assistant_text: Option<std::sync::Arc<str>>,
    pub finish_reason: TurnFinishReason,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TurnOutcome {
    Completed(CompletedTurn),
    Cancelled(TurnSummary),
    Failed(TurnFailure),
}
```

Extend `turn/src/transcript.rs` with helpers that let the driver initialize from prior history and snapshot the final transcript without rebuilding every message by hand:

```rust
impl TurnTranscript {
    pub fn from_snapshot(messages: TurnMessageSnapshot) -> Self {
        Self {
            messages: messages.iter().cloned().collect(),
        }
    }

    pub fn into_snapshot(self) -> TurnMessageSnapshot {
        std::sync::Arc::from(self.messages)
    }
}
```

Export the new types from `turn/src/lib.rs`.

**Step 2: Run focused tests**

Run: `cargo test -p turn transcript_turn_test text_only_turn_test cancel_turn_test -- --nocapture`

Expected:

- tests still fail, but only because the driver does not yet use the new helpers or return the new outcome type

**Step 3: Commit**

```bash
git add turn/src/outcome.rs turn/src/transcript.rs turn/src/lib.rs
git commit -m "feat(turn): add completed-turn boundary types"
```

### Task 3: Add a recording API without breaking existing callers

**Files:**
- Modify: `turn/src/driver.rs`
- Modify: `turn/src/lib.rs`
- Modify: `turn/tests/text_only_turn_test.rs`

**Step 1: Write the minimal implementation**

Add a new API alongside the existing `spawn()` / `spawn_with_options()` path:

```rust
pub fn spawn_recording(
    context: TurnContext,
    history: TurnMessageSnapshot,
    model: Arc<dyn ModelRunner>,
    tool_runner: Arc<dyn ToolRunner>,
    authorizer: Arc<dyn ToolAuthorizer>,
    observer: Arc<dyn TurnObserver>,
) -> (TurnHandle, JoinHandle<Result<TurnOutcome, TurnError>>)
```

Internally:

- keep the existing `spawn()` behavior intact for current callers
- move the actual driver task body to a path that returns `Result<TurnOutcome, TurnError>`
- let the old `spawn()` wrapper call the same internals and discard the final `TurnOutcome`

This keeps desktop single-turn code stable while opening a new API for multi-turn orchestration.

**Step 2: Run focused tests**

Run: `cargo test -p turn text_only_turn_test -- --nocapture`

Expected:

- the text-only recording test now compiles and gets a `TurnOutcome`, but transcript contents may still be incomplete until assistant text is recorded

**Step 3: Commit**

```bash
git add turn/src/driver.rs turn/src/lib.rs turn/tests/text_only_turn_test.rs
git commit -m "feat(turn): add recording spawn path for multi-turn callers"
```

### Task 4: Seed history and record final assistant text in the driver

**Files:**
- Modify: `turn/src/driver.rs`
- Modify: `turn/src/transcript.rs`
- Modify: `turn/tests/transcript_turn_test.rs`
- Modify: `turn/tests/text_only_turn_test.rs`
- Modify: `turn/tests/cancel_turn_test.rs`

**Step 1: Write the minimal implementation**

Update the driver so that:

- the transcript can start from a seeded history snapshot
- the new user message is appended after the seeded history
- `ResponseEvent::ContentDelta` accumulates assistant text in a local buffer
- `FinishReason::Stop` appends `TurnMessage::AssistantText` to the transcript
- the driver returns `TurnOutcome::Completed(CompletedTurn { ... })` on successful completion
- cancelled turns return `TurnOutcome::Cancelled(...)`
- failed turns return `TurnOutcome::Failed(...)`

Suggested implementation skeleton:

```rust
let mut assistant_text = String::new();

match event {
    ResponseEvent::ContentDelta(text) => {
        assistant_text.push_str(text.as_ref());
        self.emit(TurnEvent::LlmTextDelta { text }).await?;
    }
    ResponseEvent::Done { reason, .. } => break reason,
    // ...
}

if !assistant_text.is_empty() {
    self.transcript.push(TurnMessage::AssistantText {
        content: assistant_text.clone().into(),
    });
}

Ok(TurnOutcome::Completed(CompletedTurn {
    turn_id: self.context.turn_id.clone(),
    transcript: self.transcript.clone().into_snapshot(),
    assistant_text: Some(assistant_text.into()),
    finish_reason: TurnFinishReason::Completed,
}))
```

**Step 2: Run focused tests**

Run:

- `cargo test -p turn transcript_turn_test -- --nocapture`
- `cargo test -p turn text_only_turn_test -- --nocapture`
- `cargo test -p turn cancel_turn_test -- --nocapture`

Expected:

- seeded-history, completed-artifact, and cancelled-outcome tests pass

**Step 3: Commit**

```bash
git add turn/src/driver.rs \
  turn/src/transcript.rs \
  turn/tests/transcript_turn_test.rs \
  turn/tests/text_only_turn_test.rs \
  turn/tests/cancel_turn_test.rs
git commit -m "feat(turn): record completed turn artifacts for multi-turn history"
```

### Task 5: Full verification for the `turn` crate

**Files:**
- Modify: none

**Step 1: Run the full crate test suite**

Run: `cargo test -p turn`

Expected: all unit and integration tests pass

**Step 2: Run linting on touched code**

Run: `cargo clippy -p turn --tests -- -D warnings`

Expected: no warnings in touched code

**Step 3: Commit**

```bash
git add -A
git commit -m "feat(turn): establish multi-turn runtime history boundary"
```
