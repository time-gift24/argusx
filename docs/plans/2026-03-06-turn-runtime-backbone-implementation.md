# Turn Runtime Backbone Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a new `turn` runtime crate that orchestrates a single agentic turn across provider streaming, tool execution, permission pause/resume, cancellation, and Vercel-compatible event mapping.

**Architecture:** Extend the shared stream contract just enough to preserve model finish reasons, then introduce a dedicated `turn` crate with an explicit state machine and narrow control/event surfaces. Keep `Session` as a persistence observer and treat `tool` as the execution backend, adding only the minimal runtime context needed for cancellation-aware execution.

**Tech Stack:** Rust workspace crates, `tokio`, `tokio-util::sync::CancellationToken`, `serde`, `serde_json`, `async-trait`, `thiserror`, `provider`, `tool`, `cargo test`, `cargo clippy`

**Relevant Skills:** @test-driven-development, @rust-router, @m07-concurrency, @m06-error-handling, @m12-lifecycle

---

### Task 1: Extend provider/core terminal events with explicit finish reasons

**Files:**
- Modify: `Cargo.toml`
- Modify: `core/src/lib.rs`
- Modify: `core/tests/contract_state_machine_test.rs`
- Modify: `core/tests/response_event_shapes_test.rs`
- Create: `core/tests/finish_reason_shape_test.rs`
- Modify: `provider/src/dialect/openai/mapper.rs`
- Modify: `provider/src/dialect/zai/mapper.rs`
- Modify: `provider/src/bin/provider_cli.rs`
- Modify: `provider/tests/openai_mapper_behavior_test.rs`
- Modify: `provider/tests/openai_compat_replay_test.rs`
- Modify: `provider/tests/zai_mapper_test.rs`

**Step 1: Write the failing test**

```rust
use core::{FinishReason, ResponseEvent, Usage};

#[test]
fn done_event_preserves_finish_reason() {
    let event = ResponseEvent::Done {
        reason: FinishReason::ToolCalls,
        usage: Some(Usage::zero()),
    };

    match event {
        ResponseEvent::Done { reason, .. } => assert_eq!(reason, FinishReason::ToolCalls),
        _ => panic!("expected done event"),
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p core --test finish_reason_shape_test -q`  
Expected: FAIL with missing `FinishReason` or mismatched `ResponseEvent::Done` shape.

**Step 3: Write minimal implementation**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    Stop,
    ToolCalls,
    Length,
    Cancelled,
    Unknown,
}

pub enum ResponseEvent {
    // ...
    Done {
        reason: FinishReason,
        usage: Option<Usage>,
    },
    Error(Error),
}
```

Update both provider mappers so `finish_reason` from the upstream stream is preserved instead of collapsed away.

**Step 4: Run tests to verify it passes**

Run: `cargo test -p core -q`  
Expected: PASS.

Run: `cargo test -p provider -q`  
Expected: PASS.

**Step 5: Commit**

```bash
git add Cargo.toml core/src/lib.rs core/tests/contract_state_machine_test.rs core/tests/response_event_shapes_test.rs core/tests/finish_reason_shape_test.rs provider/src/dialect/openai/mapper.rs provider/src/dialect/zai/mapper.rs provider/src/bin/provider_cli.rs provider/tests/openai_mapper_behavior_test.rs provider/tests/openai_compat_replay_test.rs provider/tests/zai_mapper_test.rs
git commit -m "feat(core): preserve provider finish reasons"
```

### Task 2: Add runtime-aware execution context to `tool`

**Files:**
- Modify: `Cargo.toml`
- Modify: `tool/Cargo.toml`
- Modify: `tool/src/context.rs`
- Modify: `tool/src/trait_def.rs`
- Modify: `tool/src/scheduler.rs`
- Modify: `tool/src/builtin/shell.rs`
- Modify: `tool/src/mcp/process.rs`
- Modify: `tool/tests/runtime_integration_test.rs`
- Create: `tool/tests/tool_context_shape_test.rs`

**Step 1: Write the failing test**

```rust
use tokio_util::sync::CancellationToken;
use tool::ToolContext;

#[test]
fn tool_context_carries_runtime_cancellation() {
    let ctx = ToolContext::new("session-1", "turn-1", CancellationToken::new());
    assert_eq!(ctx.session_id, "session-1");
    assert!(!ctx.cancel_token.is_cancelled());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p tool --test tool_context_shape_test -q`  
Expected: FAIL because `ToolContext::new` or `cancel_token` does not exist.

**Step 3: Write minimal implementation**

```rust
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub session_id: String,
    pub turn_id: String,
    pub cancel_token: CancellationToken,
}

impl ToolContext {
    pub fn new(
        session_id: impl Into<String>,
        turn_id: impl Into<String>,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            turn_id: turn_id.into(),
            cancel_token,
        }
    }
}
```

Also:

- pass the new context through `ToolScheduler`
- set `kill_on_drop(true)` in `tool/src/builtin/shell.rs`
- confirm MCP stdio processes already respect drop semantics and keep that behavior covered by tests

**Step 4: Run tests to verify it passes**

Run: `cargo test -p tool -q`  
Expected: PASS.

**Step 5: Commit**

```bash
git add Cargo.toml tool/Cargo.toml tool/src/context.rs tool/src/trait_def.rs tool/src/scheduler.rs tool/src/builtin/shell.rs tool/src/mcp/process.rs tool/tests/runtime_integration_test.rs tool/tests/tool_context_shape_test.rs
git commit -m "feat(tool): add runtime-aware execution context"
```

### Task 3: Scaffold the new `turn` crate and public runtime primitives

**Files:**
- Modify: `Cargo.toml`
- Create: `turn/Cargo.toml`
- Create: `turn/src/lib.rs`
- Create: `turn/src/command.rs`
- Create: `turn/src/context.rs`
- Create: `turn/src/error.rs`
- Create: `turn/src/event.rs`
- Create: `turn/src/handle.rs`
- Create: `turn/src/state.rs`
- Create: `turn/src/summary.rs`
- Create: `turn/tests/compile_smoke_test.rs`

**Step 1: Write the failing test**

```rust
use turn::{TurnCommand, TurnEvent, TurnHandle, TurnState};

#[test]
fn turn_crate_exports_backbone_types() {
    let _ = std::mem::size_of::<TurnCommand>();
    let _ = std::mem::size_of::<TurnEvent>();
    let _ = std::mem::size_of::<TurnState>();
    let _ = std::mem::size_of::<TurnHandle>();
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p turn --test compile_smoke_test -q`  
Expected: FAIL because the crate does not exist yet.

**Step 3: Write minimal implementation**

```rust
pub enum TurnCommand {
    Cancel,
    ResolvePermission { request_id: String, decision: PermissionDecision },
}

pub enum TurnEvent {
    TurnStarted,
    TurnFinished { reason: TurnFinishReason },
}

pub enum TurnState {
    Ready,
    Completed,
    Cancelled,
    Failed,
}
```

Export only the stable backbone primitives first. Do not implement the full driver yet.

**Step 4: Run tests to verify it passes**

Run: `cargo test -p turn --test compile_smoke_test -q`  
Expected: PASS.

**Step 5: Commit**

```bash
git add Cargo.toml turn
git commit -m "feat(turn): scaffold runtime backbone crate"
```

### Task 4: Add model/tool/authorization traits and a fake test harness

**Files:**
- Create: `turn/src/model.rs`
- Create: `turn/src/tool_runner.rs`
- Create: `turn/src/permission.rs`
- Modify: `turn/src/lib.rs`
- Create: `turn/tests/support/mod.rs`
- Create: `turn/tests/support/fake_model.rs`
- Create: `turn/tests/support/fake_tool_runner.rs`
- Create: `turn/tests/support/fake_authorizer.rs`
- Create: `turn/tests/support/fake_observer.rs`
- Create: `turn/tests/runtime_harness_test.rs`

**Step 1: Write the failing test**

```rust
use turn::{ModelRunner, ToolRunner, ToolAuthorizer};

fn assert_model<T: ModelRunner>() {}
fn assert_tool<T: ToolRunner>() {}
fn assert_authorizer<T: ToolAuthorizer>() {}

#[test]
fn turn_runtime_traits_are_object_safe() {
    assert_model::<turn::tests_support::FakeModelRunner>();
    assert_tool::<turn::tests_support::FakeToolRunner>();
    assert_authorizer::<turn::tests_support::FakeAuthorizer>();
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p turn --test runtime_harness_test -q`  
Expected: FAIL because the traits and fake harness are missing.

**Step 3: Write minimal implementation**

```rust
#[async_trait]
pub trait ModelRunner: Send + Sync {
    async fn start(&self, request: LlmRequestSnapshot) -> Result<ResponseStream, TurnError>;
}

#[async_trait]
pub trait ToolRunner: Send + Sync {
    async fn execute(&self, call: ToolCall, ctx: ToolContext) -> Result<ToolResult, TurnError>;
}

#[async_trait]
pub trait ToolAuthorizer: Send + Sync {
    async fn authorize(&self, call: &ToolCall) -> AuthorizationDecision;
}
```

Build the fake harness now so the rest of the state-machine tasks can stay deterministic.

**Step 4: Run tests to verify it passes**

Run: `cargo test -p turn --test runtime_harness_test -q`  
Expected: PASS.

**Step 5: Commit**

```bash
git add turn/src/model.rs turn/src/tool_runner.rs turn/src/permission.rs turn/src/lib.rs turn/tests/support turn/tests/runtime_harness_test.rs
git commit -m "feat(turn): add runtime abstraction traits"
```

### Task 5: Implement text-only turn flow and event fan-out

**Files:**
- Create: `turn/src/driver.rs`
- Modify: `turn/src/handle.rs`
- Modify: `turn/src/state.rs`
- Modify: `turn/src/event.rs`
- Modify: `turn/src/context.rs`
- Modify: `turn/src/lib.rs`
- Create: `turn/tests/text_only_turn_test.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn text_only_turn_streams_text_and_completes() {
    let harness = support::text_only_harness(["hel", "lo"], "stop");
    let events = harness.run_to_end().await;

    assert!(events.iter().any(|e| matches!(e, TurnEvent::LlmTextDelta { text } if text == "hel")));
    assert!(events.iter().any(|e| matches!(e, TurnEvent::TurnFinished { reason: TurnFinishReason::Completed })));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p turn --test text_only_turn_test -q`  
Expected: FAIL because the driver does not yet process provider stream events.

**Step 3: Write minimal implementation**

```rust
match response_event {
    ResponseEvent::ContentDelta(text) => emit(TurnEvent::LlmTextDelta { text: text.to_string() }),
    ResponseEvent::ReasoningDelta(text) => emit(TurnEvent::LlmReasoningDelta { text: text.to_string() }),
    ResponseEvent::Done { reason, usage } => {
        self.finish_current_step(reason, usage).await?;
    }
    ResponseEvent::Error(err) => return Err(TurnError::Provider(err.message)),
    _ => {}
}
```

Implement only the no-tool happy path in this task.

**Step 4: Run tests to verify it passes**

Run: `cargo test -p turn --test text_only_turn_test -q`  
Expected: PASS.

**Step 5: Commit**

```bash
git add turn/src/driver.rs turn/src/handle.rs turn/src/state.rs turn/src/event.rs turn/src/context.rs turn/src/lib.rs turn/tests/text_only_turn_test.rs
git commit -m "feat(turn): support text-only turn execution"
```

### Task 6: Implement tool batch collection and realtime per-tool completion events

**Files:**
- Modify: `turn/src/driver.rs`
- Modify: `turn/src/event.rs`
- Modify: `turn/src/state.rs`
- Create: `turn/src/batch.rs`
- Create: `turn/tests/tool_batch_turn_test.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn tool_batch_emits_each_result_immediately_then_finishes_step_once() {
    let harness = support::parallel_tool_harness();
    let events = harness.run_to_end().await;

    let tool_results = events
        .iter()
        .filter(|event| matches!(event, TurnEvent::ToolCallCompleted { .. }))
        .count();
    let step_finishes = events
        .iter()
        .filter(|event| matches!(event, TurnEvent::StepFinished { .. }))
        .count();

    assert_eq!(tool_results, 2);
    assert_eq!(step_finishes, 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p turn --test tool_batch_turn_test -q`  
Expected: FAIL because tool calls are not yet batched and scheduled.

**Step 3: Write minimal implementation**

```rust
if !active_step.tool_calls.is_empty() {
    let batch = ToolBatch::from_calls(active_step.step_index, std::mem::take(&mut active_step.tool_calls));
    self.state = TurnState::WaitingTools(batch);
    self.dispatch_ready_tools().await?;
    return Ok(());
}
```

When each tool completes:

- emit `TurnEvent::ToolCallCompleted` immediately
- keep waiting for the rest of the batch
- emit a single `TurnEvent::StepFinished` only after the batch converges

**Step 4: Run tests to verify it passes**

Run: `cargo test -p turn --test tool_batch_turn_test -q`  
Expected: PASS.

**Step 5: Commit**

```bash
git add turn/src/driver.rs turn/src/event.rs turn/src/state.rs turn/src/batch.rs turn/tests/tool_batch_turn_test.rs
git commit -m "feat(turn): add tool batch execution loop"
```

### Task 7: Add permission pause/resume in the same turn

**Files:**
- Modify: `turn/src/command.rs`
- Modify: `turn/src/driver.rs`
- Modify: `turn/src/event.rs`
- Modify: `turn/src/permission.rs`
- Modify: `turn/src/state.rs`
- Create: `turn/tests/permission_turn_test.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn turn_waits_for_permission_and_resumes_after_allow() {
    let harness = support::permission_harness();
    let handle = harness.start().await;

    let request = harness.next_permission_request().await;
    handle.resolve_permission(request.request_id.clone(), PermissionDecision::Allow).await;

    let events = harness.collect_to_end().await;
    assert!(events.iter().any(|e| matches!(e, TurnEvent::ToolCallPermissionRequested { .. })));
    assert!(events.iter().any(|e| matches!(e, TurnEvent::ToolCallPermissionResolved { .. })));
    assert!(events.iter().any(|e| matches!(e, TurnEvent::TurnFinished { reason: TurnFinishReason::Completed })));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p turn --test permission_turn_test -q`  
Expected: FAIL because the driver cannot suspend and resume a batch.

**Step 3: Write minimal implementation**

```rust
match authorizer.authorize(&call).await {
    AuthorizationDecision::Allow => dispatch(call),
    AuthorizationDecision::Deny => complete_as_denied(call),
    AuthorizationDecision::Ask(request) => {
        emit(TurnEvent::ToolCallPermissionRequested { request: request.clone() });
        self.state = TurnState::WaitingForPermission(PermissionPause::new(batch, request));
    }
}
```

On `TurnCommand::ResolvePermission`:

- mark the request resolved
- convert `Allow` back into a pending dispatch
- convert `Deny` into `ToolOutcome::Denied`
- return to `WaitingTools` when all pending permissions are settled

**Step 4: Run tests to verify it passes**

Run: `cargo test -p turn --test permission_turn_test -q`  
Expected: PASS.

**Step 5: Commit**

```bash
git add turn/src/command.rs turn/src/driver.rs turn/src/event.rs turn/src/permission.rs turn/src/state.rs turn/tests/permission_turn_test.rs
git commit -m "feat(turn): add same-turn permission pause and resume"
```

### Task 8: Add cancellation, timeout handling, and Vercel stream adaptation

**Files:**
- Modify: `turn/Cargo.toml`
- Modify: `turn/src/driver.rs`
- Modify: `turn/src/error.rs`
- Modify: `turn/src/event.rs`
- Modify: `turn/src/handle.rs`
- Create: `turn/src/vercel.rs`
- Create: `turn/tests/cancel_turn_test.rs`
- Create: `turn/tests/timeout_turn_test.rs`
- Create: `turn/tests/vercel_adapter_test.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn cancelling_during_tool_execution_marks_turn_cancelled_and_keeps_completed_results() {
    let harness = support::slow_tool_harness();
    let handle = harness.start().await;

    harness.wait_until_tool_started().await;
    handle.cancel().await;

    let events = harness.collect_to_end().await;
    assert!(events.iter().any(|e| matches!(e, TurnEvent::TurnFinished { reason: TurnFinishReason::Cancelled })));
}
```

```rust
#[test]
fn vercel_adapter_maps_permission_and_tool_events() {
    let events = turn::vercel::map_events(vec![
        TurnEvent::ToolCallPermissionRequested { request: sample_request() },
        TurnEvent::ToolCallCompleted { call_id: "call_1".into(), result: sample_success() },
    ]);

    assert!(events.iter().any(|line| line.starts_with("1:")));
    assert!(events.iter().any(|line| line.starts_with("8:")));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p turn --test cancel_turn_test -q`  
Expected: FAIL because the driver does not yet support cancel semantics.

Run: `cargo test -p turn --test timeout_turn_test -q`  
Expected: FAIL because tool execution is not wrapped in timeouts.

Run: `cargo test -p turn --test vercel_adapter_test -q`  
Expected: FAIL because no Vercel adapter exists yet.

**Step 3: Write minimal implementation**

```rust
tokio::select! {
    _ = self.cancel_token.cancelled() => {
        self.stop_launching_new_work();
        self.state = TurnState::Cancelled(self.build_partial_summary());
    }
    result = tokio::time::timeout(timeout, tool_future) => {
        match result {
            Ok(result) => self.handle_tool_result(result).await?,
            Err(_) => self.complete_tool_as_timeout(call_id).await?,
        }
    }
}
```

In `turn/src/vercel.rs`, map:

- text deltas to `0:`
- reasoning deltas to `4:`
- permission/control events to `1:`
- tool calls to `7:`
- tool results to `8:`
- step finished to `e:`
- final turn finished to `d:`

**Step 4: Run tests to verify it passes**

Run: `cargo test -p turn -q`  
Expected: PASS.

Run: `cargo clippy -p turn --all-targets -- -D warnings`  
Expected: PASS.

**Step 5: Commit**

```bash
git add turn/Cargo.toml turn/src/driver.rs turn/src/error.rs turn/src/event.rs turn/src/handle.rs turn/src/vercel.rs turn/tests/cancel_turn_test.rs turn/tests/timeout_turn_test.rs turn/tests/vercel_adapter_test.rs
git commit -m "feat(turn): add cancellation and vercel stream adapter"
```

### Task 9: Remove stale runtime references to deleted `agent*` crates

**Files:**
- Modify: `tests/agent-turn-cli/Cargo.toml`
- Modify: `tests/agent-turn-cli/src/main.rs`
- Modify: `tests/agent-session-cli/Cargo.toml`
- Modify: `tests/agent-session-cli/src/main.rs`
- Modify: `tests/agent-session-cli/src/mock.rs`
- Optionally delete: `tests/agent-turn-cli/**`
- Optionally delete: `tests/agent-session-cli/**`
- Modify: `docs/plans/2026-03-06-turn-runtime-backbone-design.md`

**Step 1: Write the failing test**

```text
No code test for this cleanup step.
Use `cargo check --workspace` as the regression gate after removing or quarantining stale references.
```

**Step 2: Run verification to capture the current failure or confusion**

Run: `cargo check --workspace`  
Expected: PASS for workspace crates, but stale non-workspace manifests still point to deleted `agent*` crates and should be cleaned up or archived deliberately.

**Step 3: Write minimal implementation**

- either delete the stale `tests/agent-turn-cli` and `tests/agent-session-cli` trees
- or rewrite them as archived fixtures with a README explaining they are obsolete and not part of the workspace

Prefer deletion if they no longer provide value.

**Step 4: Run verification**

Run: `cargo check --workspace`  
Expected: PASS.

Run: `git grep -n \"agent-core\\|agent-turn\\|llm-client\\|llm-provider\"`  
Expected: only intentional historical references remain, or no matches.

**Step 5: Commit**

```bash
git add -A tests/agent-turn-cli tests/agent-session-cli docs/plans/2026-03-06-turn-runtime-backbone-design.md
git commit -m "chore: remove stale agent runtime references"
```
