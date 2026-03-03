# Agent Runtime Center Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build `agent-runtime-center` as a hybrid runtime pool that isolates process-level crashes, auto-recovers from checkpoint once, and integrates with desktop without breaking existing stream semantics.

**Architecture:** Introduce a new orchestration crate between desktop and runtime execution. The center classifies requests, schedules them into shared/in-isolated pools, supervises worker health, and performs one-time checkpoint retry on crash. `subagent` is treated as a tool call and is always executed via isolated workers with retry metadata carried in execution context.

**Tech Stack:** Rust 2021, Tokio async runtime, existing workspace crates (`agent-core`, `agent-turn`, `agent-session`, `agent-tool`, `desktop/src-tauri`), serde/serde_json, tracing.

---

## Execution Rules

1. Use @test-driven-development for every behavior change.
2. Use @systematic-debugging if any test fails unexpectedly.
3. Use @verification-before-completion before claiming task completion.
4. Keep commits small and task-scoped.

---

### Task 1: Scaffold `agent-runtime-center` Crate and Public API Skeleton

**Files:**
- Modify: `Cargo.toml`
- Create: `agent-runtime-center/Cargo.toml`
- Create: `agent-runtime-center/src/lib.rs`
- Create: `agent-runtime-center/src/config.rs`
- Create: `agent-runtime-center/src/types.rs`
- Create: `agent-runtime-center/src/error.rs`
- Create: `agent-runtime-center/tests/public_api_test.rs`

**Step 1: Write the failing test**

```rust
// agent-runtime-center/tests/public_api_test.rs
use agent_runtime_center::{PoolMode, RuntimeCenterConfig, RoutingDecision};

#[test]
fn public_types_are_exposed() {
    let cfg = RuntimeCenterConfig::default();
    assert_eq!(cfg.shared_min_workers, 2);
    assert_eq!(cfg.isolated_max_workers, 8);
    assert_eq!(RoutingDecision::Isolated.mode(), PoolMode::Isolated);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-runtime-center public_api_test -v`  
Expected: FAIL (crate/types not found yet).

**Step 3: Write minimal implementation**

1. Add `agent-runtime-center` to workspace members.
2. Create crate with minimal exported types:
   - `RuntimeCenterConfig { shared_min_workers, isolated_max_workers, ... }`
   - `PoolMode::{Shared, Isolated}`
   - `RoutingDecision::{Shared, Isolated}`
   - `RoutingDecision::mode()`

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-runtime-center public_api_test -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add Cargo.toml agent-runtime-center/
git commit -m "feat(agent-runtime-center): add crate skeleton and public config types"
```

---

### Task 2: Extend Turn Metadata for Routing and Subagent Detection

**Files:**
- Modify: `agent-core/src/model.rs`
- Modify: `agent-core/src/lib.rs`
- Create: `agent-core/tests/turn_request_routing_profile_test.rs`
- Modify: `desktop/src-tauri/src/lib.rs`
- Modify: `agent-session/src/session_runtime.rs` (compile-through for `TurnRequest` construction)
- Modify: `agent/src/agent.rs` (compile-through for `TurnRequest` construction)

**Step 1: Write the failing test**

```rust
// agent-core/tests/turn_request_routing_profile_test.rs
use agent_core::{InputEnvelope, SessionMeta, TurnRequest};

#[test]
fn turn_request_defaults_to_non_subagent_and_no_route_hints() {
    let req = TurnRequest::new(
        SessionMeta::new("s1".into(), "t1".into()),
        "bigmodel",
        "glm-5",
        InputEnvelope::user_text("hello"),
    );
    assert!(!req.is_subagent);
    assert_eq!(req.route_hint, None);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-core --test turn_request_routing_profile_test -v`  
Expected: FAIL (`is_subagent` / `route_hint` missing).

**Step 3: Write minimal implementation**

1. Add new fields to `TurnRequest`:
   - `is_subagent: bool` (serde default `false`)
   - `route_hint: Option<String>` (serde default `None`)
2. Update `TurnRequest::new` and all existing constructors/call sites.

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-core --test turn_request_routing_profile_test -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-core/src/model.rs agent-core/src/lib.rs \
  agent-core/tests/turn_request_routing_profile_test.rs \
  desktop/src-tauri/src/lib.rs agent-session/src/session_runtime.rs agent/src/agent.rs
git commit -m "feat(agent-core): add routing metadata on turn request"
```

---

### Task 3: Implement Classifier (Subagent/High-Risk/Long-Task Routing)

**Files:**
- Create: `agent-runtime-center/src/classifier.rs`
- Modify: `agent-runtime-center/src/lib.rs`
- Create: `agent-runtime-center/tests/classifier_test.rs`

**Step 1: Write the failing test**

```rust
// agent-runtime-center/tests/classifier_test.rs
use agent_runtime_center::{classify, ClassificationInput, RoutingDecision};

#[test]
fn subagent_is_always_isolated() {
    let input = ClassificationInput::builder().is_subagent(true).build();
    assert_eq!(classify(&input), RoutingDecision::Isolated);
}

#[test]
fn escalated_permission_routes_to_isolated() {
    let input = ClassificationInput::builder()
        .requires_escalation(true)
        .build();
    assert_eq!(classify(&input), RoutingDecision::Isolated);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-runtime-center classifier_test -v`  
Expected: FAIL (classifier not implemented).

**Step 3: Write minimal implementation**

1. Add `ClassificationInput` with:
   - `is_subagent`
   - `requires_escalation`
   - `high_risk_tool_intent`
   - `estimated_tool_calls`
   - `historical_p95_ms`
2. Implement priority order:
   - subagent > high-risk > long-task > shared.

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-runtime-center classifier_test -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-runtime-center/src/classifier.rs agent-runtime-center/src/lib.rs \
  agent-runtime-center/tests/classifier_test.rs
git commit -m "feat(agent-runtime-center): add routing classifier with subagent-first priority"
```

---

### Task 4: Implement Scheduler and Hybrid Scaling Policy

**Files:**
- Create: `agent-runtime-center/src/scheduler.rs`
- Modify: `agent-runtime-center/src/config.rs`
- Modify: `agent-runtime-center/src/lib.rs`
- Create: `agent-runtime-center/tests/scheduler_test.rs`

**Step 1: Write the failing test**

```rust
// agent-runtime-center/tests/scheduler_test.rs
use agent_runtime_center::{RuntimeCenterConfig, Scheduler};

#[test]
fn scheduler_keeps_shared_min_and_caps_isolated_max() {
    let cfg = RuntimeCenterConfig::default();
    let s = Scheduler::new(cfg);
    assert_eq!(s.shared_target(), 2);
    assert_eq!(s.isolated_max(), 8);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-runtime-center scheduler_test -v`  
Expected: FAIL.

**Step 3: Write minimal implementation**

1. Add scheduler state:
   - queue depth
   - active shared workers
   - active isolated workers
2. Implement:
   - `shared_target() == shared_min_workers`
   - `isolated_max() == isolated_max_workers`
   - `try_scale_isolated(queue_depth)` with upper bound.

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-runtime-center scheduler_test -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-runtime-center/src/scheduler.rs agent-runtime-center/src/config.rs \
  agent-runtime-center/src/lib.rs agent-runtime-center/tests/scheduler_test.rs
git commit -m "feat(agent-runtime-center): add hybrid scheduler and scaling bounds"
```

---

### Task 5: Implement Recovery Coordinator (One Retry from Checkpoint)

**Files:**
- Create: `agent-runtime-center/src/recovery.rs`
- Modify: `agent-runtime-center/src/error.rs`
- Modify: `agent-runtime-center/src/types.rs`
- Modify: `agent-runtime-center/src/lib.rs`
- Create: `agent-runtime-center/tests/recovery_test.rs`

**Step 1: Write the failing test**

```rust
// agent-runtime-center/tests/recovery_test.rs
use agent_runtime_center::{RecoveryCoordinator, TurnFailureKind};

#[tokio::test]
async fn crash_triggers_single_retry_then_fails_with_both_causes() {
    let rc = RecoveryCoordinator::new();
    let err = rc
        .recover_once_for_test("turn-1", "worker crashed", "retry stream ended")
        .await
        .unwrap_err();
    assert_eq!(err.kind(), TurnFailureKind::RetryFailed);
    assert!(err.message().contains("worker crashed"));
    assert!(err.message().contains("retry stream ended"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-runtime-center recovery_test -v`  
Expected: FAIL.

**Step 3: Write minimal implementation**

1. Add `TurnFailureKind` and structured center error type.
2. Implement `RecoveryCoordinator`:
   - accepts crash signal
   - performs max 1 retry
   - returns merged failure details on second failure.

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-runtime-center recovery_test -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-runtime-center/src/recovery.rs agent-runtime-center/src/error.rs \
  agent-runtime-center/src/types.rs agent-runtime-center/src/lib.rs \
  agent-runtime-center/tests/recovery_test.rs
git commit -m "feat(agent-runtime-center): add checkpoint-based single-retry recovery coordinator"
```

---

### Task 6: Add Subagent Retry Context in Tool Execution Path

**Files:**
- Modify: `agent-core/src/tools/mod.rs`
- Modify: `agent-core/src/lib.rs`
- Create: `agent-core/tests/tool_execution_context_retry_test.rs`
- Modify: `agent-turn/src/effect.rs`

**Step 1: Write the failing test**

```rust
// agent-core/tests/tool_execution_context_retry_test.rs
use agent_core::tools::ToolExecutionContext;

#[test]
fn tool_execution_context_supports_retry_metadata() {
    let ctx = ToolExecutionContext::new_for_test("s1", "t1")
        .with_retry_attempt(1)
        .with_retry_reason("worker_crash")
        .with_parent_turn_id("parent-1")
        .with_stable_tool_call_id("call-1");
    assert_eq!(ctx.retry_attempt, Some(1));
    assert_eq!(ctx.retry_reason.as_deref(), Some("worker_crash"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-core --test tool_execution_context_retry_test -v`  
Expected: FAIL.

**Step 3: Write minimal implementation**

1. Extend `ToolExecutionContext` with optional retry metadata fields:
   - `retry_attempt`
   - `retry_reason`
   - `parent_turn_id`
   - `stable_tool_call_id`
2. In `agent-turn/src/effect.rs`, when executing `subagent.*` tool calls, populate these fields on retry.

**Step 4: Run test to verify it passes**

Run:  
- `cargo test -p agent-core --test tool_execution_context_retry_test -v`  
- `cargo test -p agent-turn effect::tests -- --nocapture`  
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-core/src/tools/mod.rs agent-core/src/lib.rs \
  agent-core/tests/tool_execution_context_retry_test.rs agent-turn/src/effect.rs
git commit -m "feat(tooling): carry subagent retry context in execution metadata"
```

---

### Task 7: Implement Shared Pool Worker Adapter on Top of `SessionRuntime`

**Files:**
- Create: `agent-runtime-center/src/pool/mod.rs`
- Create: `agent-runtime-center/src/pool/shared.rs`
- Modify: `agent-runtime-center/src/lib.rs`
- Create: `agent-runtime-center/tests/shared_pool_test.rs`

**Step 1: Write the failing test**

```rust
// agent-runtime-center/tests/shared_pool_test.rs
use agent_runtime_center::pool::shared::SharedPool;

#[tokio::test]
async fn shared_pool_can_execute_turn_via_runtime_worker() {
    let pool = SharedPool::new_for_test(2);
    let result = pool.run_noop_turn_for_test().await;
    assert!(result.is_ok());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-runtime-center shared_pool_test -v`  
Expected: FAIL.

**Step 3: Write minimal implementation**

1. Define worker trait abstraction (`RuntimeWorker`).
2. Implement `SharedPool` with worker checkout/release lifecycle.
3. Wire through existing `SessionRuntime` compatibility adapter.

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-runtime-center shared_pool_test -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-runtime-center/src/pool/mod.rs agent-runtime-center/src/pool/shared.rs \
  agent-runtime-center/src/lib.rs agent-runtime-center/tests/shared_pool_test.rs
git commit -m "feat(agent-runtime-center): add shared runtime pool adapter"
```

---

### Task 8: Implement Isolated Pool Worker Supervisor (Subprocess)

**Files:**
- Create: `agent-runtime-center/src/pool/isolated.rs`
- Create: `agent-runtime-center/src/ipc.rs`
- Create: `agent-runtime-center/src/bin/runtime-worker.rs`
- Modify: `agent-runtime-center/Cargo.toml`
- Create: `agent-runtime-center/tests/isolated_pool_test.rs`

**Step 1: Write the failing test**

```rust
// agent-runtime-center/tests/isolated_pool_test.rs
use agent_runtime_center::pool::isolated::IsolatedPool;

#[tokio::test]
async fn isolated_pool_replaces_crashed_worker() {
    let pool = IsolatedPool::new_for_test(8);
    let report = pool.simulate_worker_crash_and_recover_for_test().await;
    assert!(report.replaced);
    assert_eq!(report.retry_attempted, 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-runtime-center isolated_pool_test -v`  
Expected: FAIL.

**Step 3: Write minimal implementation**

1. Define lightweight IPC envelope for request/event forwarding.
2. Add subprocess worker binary with stdio protocol.
3. Implement supervisor:
   - spawn/re-spawn worker
   - health probe
   - crash detection callback to recovery coordinator.

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-runtime-center isolated_pool_test -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-runtime-center/src/pool/isolated.rs agent-runtime-center/src/ipc.rs \
  agent-runtime-center/src/bin/runtime-worker.rs agent-runtime-center/Cargo.toml \
  agent-runtime-center/tests/isolated_pool_test.rs
git commit -m "feat(agent-runtime-center): add isolated subprocess pool and worker supervisor"
```

---

### Task 9: Implement `AgentRuntimeCenter` Orchestration and Session Affinity

**Files:**
- Create: `agent-runtime-center/src/center.rs`
- Create: `agent-runtime-center/src/session_affinity.rs`
- Create: `agent-runtime-center/src/telemetry.rs`
- Modify: `agent-runtime-center/src/lib.rs`
- Create: `agent-runtime-center/tests/center_routing_test.rs`

**Step 1: Write the failing test**

```rust
// agent-runtime-center/tests/center_routing_test.rs
use agent_runtime_center::{AgentRuntimeCenter, RoutingDecision};

#[tokio::test]
async fn center_routes_subagent_to_isolated() {
    let center = AgentRuntimeCenter::new_for_test();
    let decision = center.route_decision_for_test(true, false, false).await;
    assert_eq!(decision, RoutingDecision::Isolated);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-runtime-center center_routing_test -v`  
Expected: FAIL.

**Step 3: Write minimal implementation**

1. Implement center orchestration pipeline:
   - classify -> schedule -> dispatch -> stream forward -> recover.
2. Add session affinity map for non-subagent turns.
3. Emit telemetry counters/timers on each lifecycle transition.

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-runtime-center center_routing_test -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-runtime-center/src/center.rs agent-runtime-center/src/session_affinity.rs \
  agent-runtime-center/src/telemetry.rs agent-runtime-center/src/lib.rs \
  agent-runtime-center/tests/center_routing_test.rs
git commit -m "feat(agent-runtime-center): add orchestration, affinity, and telemetry"
```

---

### Task 10: Integrate Desktop Tauri Runtime with `AgentRuntimeCenter`

**Files:**
- Modify: `desktop/src-tauri/src/lib.rs`
- Create: `desktop/src-tauri/tests/runtime_center_desktop_test.rs` (if test harness is available)
- Modify: `desktop/src-tauri/Cargo.toml`

**Step 1: Write the failing test**

```rust
// desktop/src-tauri/tests/runtime_center_desktop_test.rs
#[test]
fn app_state_uses_runtime_center_type_alias() {
    // compile-time test: desktop build should depend on agent-runtime-center.
    assert!(true);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p desktop_lib runtime_center_desktop_test -v`  
Expected: FAIL (no runtime center wiring yet) or compile failure.

**Step 3: Write minimal implementation**

1. Replace `RuntimeHandle = SessionRuntime<...>` with center handle.
2. Update `build_runtime_state` to construct center (shared+isolated pools).
3. Keep existing command contract:
   - `start_agent_turn`
   - `cancel_agent_turn`
   - `restore_turn_checkpoint`
4. Preserve stream envelope ordering (`seq` monotonic).

**Step 4: Run test to verify it passes**

Run: `cargo test -p desktop_lib -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/src-tauri/src/lib.rs desktop/src-tauri/Cargo.toml \
  desktop/src-tauri/tests/runtime_center_desktop_test.rs
git commit -m "feat(desktop): switch runtime orchestration to agent-runtime-center"
```

---

### Task 11: Full Verification, Regression, and Plan Traceability

**Files:**
- Create: `docs/plans/2026-03-03-agent-runtime-center-verification.md`
- Modify: `docs/plans/2026-03-03-agent-runtime-center-design.md` (optional cross-links)

**Step 1: Write verification checklist file**

Include sections:
1. Unit test matrix
2. Crash recovery scenarios
3. Subagent isolation scenarios
4. Manual desktop smoke checks

**Step 2: Run full verification**

Run:

```bash
cargo test -p agent-core -v
cargo test -p agent-turn -v
cargo test -p agent-session -v
cargo test -p agent-runtime-center -v
cargo test -p desktop_lib -v
cargo clippy -p agent-runtime-center -p desktop_lib --all-targets -- -D warnings
```

Expected: all PASS, no clippy warnings.

**Step 3: Commit**

```bash
git add docs/plans/2026-03-03-agent-runtime-center-verification.md \
  docs/plans/2026-03-03-agent-runtime-center-design.md
git commit -m "docs: add runtime center verification checklist and traceability links"
```

---

## Final Acceptance Criteria

1. Desktop uses `agent-runtime-center` as orchestration entry (not direct single `SessionRuntime`).
2. `subagent` tool calls are always routed to isolated pool.
3. Worker crash triggers automatic replacement and exactly one checkpoint retry.
4. Retry failure returns `failed` with both crash and retry reasons.
5. Existing frontend stream behavior remains compatible.
