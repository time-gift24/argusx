# Agent-Turn Event Bus Refactor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the monolithic `reduce`-centric turn loop with an event bus architecture (`DomainCommand -> DomainEvent -> OutputEvent`) and remove duplicated/noisy event pathways.

**Architecture:** Build a deterministic single-turn event bus with handler registry and projectors. Commands are normalized first, handlers emit domain facts, projectors mutate state and generate outputs, and dispatchers emit run/ui/effect signals. Runtime wiring switches in one cutover.

**Tech Stack:** Rust, Tokio, `mpsc` channels, existing `agent-core` event types (to be evolved), existing `agent-turn` runtime/effect layers.

---

### Task 1: Introduce New Event Type System and Bus Skeleton

**Files:**
- Create: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/command/mod.rs`
- Create: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/domain/mod.rs`
- Create: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/output/mod.rs`
- Create: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/bus/mod.rs`
- Modify: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/lib.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/bus/mod.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn bus_pump_preserves_fifo_order() {
    let mut bus = EventBus::new(BusConfig::default());
    bus.enqueue_command(DomainCommand::Noop { id: "c1".into() }).unwrap();
    bus.enqueue_command(DomainCommand::Noop { id: "c2".into() }).unwrap();

    let drained = bus.drain_commands_for_test();
    assert_eq!(drained.len(), 2);
    assert_eq!(drained[0].id(), "c1");
    assert_eq!(drained[1].id(), "c2");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn bus_pump_preserves_fifo_order -- --nocapture`
Expected: FAIL with unresolved types (`EventBus`, `DomainCommand`, `BusConfig`).

**Step 3: Write minimal implementation**

```rust
pub enum DomainCommand {
    Noop { id: String },
}

impl DomainCommand {
    pub fn id(&self) -> &str {
        match self {
            Self::Noop { id } => id,
        }
    }
}

pub struct BusConfig {
    pub command_capacity: usize,
}

impl Default for BusConfig {
    fn default() -> Self {
        Self { command_capacity: 1024 }
    }
}

pub struct EventBus {
    command_queue: std::collections::VecDeque<DomainCommand>,
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-turn bus_pump_preserves_fifo_order -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-turn/src/command/mod.rs agent-turn/src/domain/mod.rs agent-turn/src/output/mod.rs agent-turn/src/bus/mod.rs agent-turn/src/lib.rs
git commit -m "refactor(agent-turn): scaffold event bus and domain command/event/output types"
```

---

### Task 2: Add Command Normalizer (Dedup + Coalescing)

**Files:**
- Create: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/command/normalizer.rs`
- Modify: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/command/mod.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/command/normalizer.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn normalizer_drops_duplicate_event_ids() {
    let mut n = CommandNormalizer::default();
    let first = DomainCommand::from_runtime(RuntimeEvent::FatalError {
        event_id: "e1".into(),
        message: "x".into(),
    });
    let second = first.clone();

    assert!(n.normalize(first).is_some());
    assert!(n.normalize(second).is_none());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn normalizer_drops_duplicate_event_ids -- --nocapture`
Expected: FAIL with missing `CommandNormalizer` and `from_runtime` mapping.

**Step 3: Write minimal implementation**

```rust
#[derive(Default)]
pub struct CommandNormalizer {
    seen: std::collections::HashSet<String>,
}

impl CommandNormalizer {
    pub fn normalize(&mut self, cmd: DomainCommand) -> Option<DomainCommand> {
        let id = cmd.id().to_string();
        if !self.seen.insert(id) {
            return None;
        }
        Some(cmd)
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-turn normalizer_drops_duplicate_event_ids -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-turn/src/command/mod.rs agent-turn/src/command/normalizer.rs
git commit -m "refactor(agent-turn): add command normalizer with event-id dedup"
```

---

### Task 3: Implement Handler Registry and Core Domain Handlers (Model/Input)

**Files:**
- Create: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/handlers/mod.rs`
- Create: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/handlers/model.rs`
- Create: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/handlers/input.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/handlers/model.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/handlers/input.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn model_text_delta_command_emits_model_chunk_arrived_event() {
    let mut reg = HandlerRegistry::default();
    let cmd = DomainCommand::ModelTextDelta {
        id: "c1".into(),
        epoch: 0,
        delta: "hello".into(),
    };
    let out = reg.handle(cmd, &TurnState::new(SessionMeta::new("s", "t"), "p", "m"));
    assert!(matches!(out.as_slice(), [DomainEvent::ModelChunkArrived { .. }]));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn model_text_delta_command_emits_model_chunk_arrived_event -- --nocapture`
Expected: FAIL with missing `HandlerRegistry` or missing event mapping.

**Step 3: Write minimal implementation**

```rust
pub trait CommandHandler {
    fn handle(&self, cmd: &DomainCommand, state: &TurnState) -> Vec<DomainEvent>;
}

pub struct HandlerRegistry {
    handlers: Vec<Box<dyn CommandHandler + Send + Sync>>,
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-turn model_text_delta_command_emits_model_chunk_arrived_event -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-turn/src/handlers/mod.rs agent-turn/src/handlers/model.rs agent-turn/src/handlers/input.rs
git commit -m "refactor(agent-turn): add handler registry and model/input handlers"
```

---

### Task 4: Implement Tool/Lifecycle/Subagent Handlers

**Files:**
- Create: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/handlers/tool.rs`
- Create: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/handlers/lifecycle.rs`
- Create: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/handlers/subagent.rs`
- Modify: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/handlers/mod.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/handlers/tool.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/handlers/lifecycle.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn tool_result_ok_command_emits_tool_finished_event() {
    let reg = HandlerRegistry::default();
    let cmd = DomainCommand::ToolResultOk {
        id: "c1".into(),
        epoch: 0,
        result: ToolResult::ok("call-1", serde_json::json!({"ok": true})),
    };
    let out = reg.handle(cmd, &state_with_inflight("call-1"));
    assert!(out.iter().any(|e| matches!(e, DomainEvent::ToolFinished { is_error: false, .. })));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn tool_result_ok_command_emits_tool_finished_event -- --nocapture`
Expected: FAIL with missing handler mapping.

**Step 3: Write minimal implementation**

```rust
// tool handler emits ToolPlanned/ToolStarted/ToolProgress/ToolFinished events.
// lifecycle handler emits RetryScheduled/RetryFired/TurnFailed/TurnCancelled.
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-turn tool_result_ok_command_emits_tool_finished_event -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-turn/src/handlers/mod.rs agent-turn/src/handlers/tool.rs agent-turn/src/handlers/lifecycle.rs agent-turn/src/handlers/subagent.rs
git commit -m "refactor(agent-turn): add tool lifecycle and subagent command handlers"
```

---

### Task 5: Add StateProjector and OutputProjector

**Files:**
- Create: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/projectors/state.rs`
- Create: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/projectors/output.rs`
- Create: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/projectors/mod.rs`
- Modify: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/state.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/projectors/state.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/projectors/output.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn state_projector_updates_output_buffer_on_model_chunk() {
    let mut s = TurnState::new(SessionMeta::new("s1", "t1"), "p", "m");
    StateProjector::apply(&mut s, &DomainEvent::ModelChunkArrived {
        epoch: 0,
        kind: ModelChunkKind::Text,
        payload: "hello".into(),
    });
    assert_eq!(s.output_buffer, "hello");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn state_projector_updates_output_buffer_on_model_chunk -- --nocapture`
Expected: FAIL due to missing projector.

**Step 3: Write minimal implementation**

```rust
pub struct StateProjector;
impl StateProjector {
    pub fn apply(state: &mut TurnState, event: &DomainEvent) {
        // all state mutations centralized here
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-turn state_projector_updates_output_buffer_on_model_chunk -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-turn/src/projectors/mod.rs agent-turn/src/projectors/state.rs agent-turn/src/projectors/output.rs agent-turn/src/state.rs
git commit -m "refactor(agent-turn): centralize state mutation and output mapping via projectors"
```

---

### Task 6: Wire Event Bus Into TurnEngine and Runtime

**Files:**
- Modify: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/engine.rs`
- Modify: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/runtime_impl.rs`
- Modify: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/effect.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/runtime_impl.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn runtime_routes_runtime_event_through_bus_pipeline() {
    // assert output is produced via bus pump, not legacy reducer
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn runtime_routes_runtime_event_through_bus_pipeline -- --nocapture`
Expected: FAIL because engine is still reducer-centric.

**Step 3: Write minimal implementation**

```rust
// TurnEngine::run now:
// 1) RuntimeEvent -> DomainCommand
// 2) normalize
// 3) handlers -> DomainEvent
// 4) project state + output
// 5) dispatch run/ui/effect/checkpoint outputs
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-turn runtime_routes_runtime_event_through_bus_pipeline -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-turn/src/engine.rs agent-turn/src/runtime_impl.rs agent-turn/src/effect.rs
git commit -m "refactor(agent-turn): switch runtime to event bus pipeline"
```

---

### Task 7: Remove Legacy Reducer Path and Migrate Existing Tests

**Files:**
- Modify: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/reducer.rs`
- Modify: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/test_helpers.rs`
- Modify: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/lib.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/reducer.rs`

**Step 1: Write the failing migration test**

```rust
#[test]
fn legacy_reducer_equivalence_trace_matches_bus_trace_for_core_flow() {
    // same scenario run through bus pipeline and expected sequence asserted
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn legacy_reducer_equivalence_trace_matches_bus_trace_for_core_flow -- --nocapture`
Expected: FAIL because equivalence harness is not complete.

**Step 3: Write minimal implementation**

```rust
// Keep reducer as thin compatibility shim or remove direct runtime dependency.
// Migrate scenario helpers to bus-driven harness.
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-turn -- --nocapture`
Expected: PASS for migrated suites.

**Step 5: Commit**

```bash
git add agent-turn/src/reducer.rs agent-turn/src/test_helpers.rs agent-turn/src/lib.rs
git commit -m "refactor(agent-turn): retire legacy reducer main path and migrate tests"
```

---

### Task 8: Evolve Cross-Crate Event Contracts and Remove Duplicates

**Files:**
- Modify: `/Users/wanyaozhong/Projects/argusx/agent-core/src/runtime_event.rs`
- Modify: `/Users/wanyaozhong/Projects/argusx/agent-core/src/events.rs`
- Modify: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/effect.rs`
- Modify: `/Users/wanyaozhong/Projects/argusx/agent-session/src/session_runtime.rs`
- Modify: `/Users/wanyaozhong/Projects/argusx/desktop/src-tauri/src/lib.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/agent-session/tests/*.rs`
- Test: `/Users/wanyaozhong/Projects/argusx/agent-turn/tests/*.rs`

**Step 1: Write failing contract tests**

```rust
#[test]
fn runtime_event_contract_no_longer_contains_redundant_tool_queue_variants() {
    // compile-time or serde contract assertions
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-core runtime_event_contract_no_longer_contains_redundant_tool_queue_variants -- --nocapture`
Expected: FAIL with old event variants still present.

**Step 3: Write minimal implementation**

```rust
// collapse redundant runtime/run/ui events and update mappings in turn/session/desktop.
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p agent-core -p agent-turn -p agent-session -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-core/src/runtime_event.rs agent-core/src/events.rs agent-turn/src/effect.rs agent-session/src/session_runtime.rs desktop/src-tauri/src/lib.rs
git commit -m "refactor(core/turn): simplify event contracts and remove duplicate event variants"
```

---

### Task 9: Golden Trace, Stress, and Recovery Verification

**Files:**
- Create: `/Users/wanyaozhong/Projects/argusx/agent-turn/tests/golden_trace_equivalence_test.rs`
- Create: `/Users/wanyaozhong/Projects/argusx/agent-turn/tests/bus_stress_backpressure_test.rs`
- Create: `/Users/wanyaozhong/Projects/argusx/agent-turn/tests/checkpoint_replay_consistency_test.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn golden_trace_core_flows_match_expected_contract() { /* ... */ }

#[tokio::test]
async fn bus_handles_high_volume_tool_deltas_without_unbounded_growth() { /* ... */ }

#[tokio::test]
async fn checkpoint_replay_reconstructs_state_consistently() { /* ... */ }
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p agent-turn golden_trace_core_flows_match_expected_contract -- --nocapture`
Expected: FAIL because fixture and comparison harness are not implemented.

**Step 3: Write minimal implementation**

```rust
// build canonical traces for normal/tool-fail/retry/cancel/long-output flows.
// assert deterministic order and whitelisted differences only.
```

**Step 4: Run full verification**

Run: `cargo test -p agent-turn -- --nocapture`
Expected: PASS for all unit and integration tests.

Run: `cargo test -p agent-session -- --nocapture`
Expected: PASS to validate session/runtime integration.

Run: `cargo fmt && cargo clippy -p agent-turn --all-targets -- -D warnings`
Expected: PASS with zero warnings.

**Step 5: Commit**

```bash
git add agent-turn/tests/golden_trace_equivalence_test.rs agent-turn/tests/bus_stress_backpressure_test.rs agent-turn/tests/checkpoint_replay_consistency_test.rs
git commit -m "test(agent-turn): add golden trace stress and replay coverage for event bus refactor"
```

---

### Task 10: Final Cleanup and Documentation Updates

**Files:**
- Modify: `/Users/wanyaozhong/Projects/argusx/agent-turn/src/lib.rs`
- Modify: `/Users/wanyaozhong/Projects/argusx/agent-turn/README.md` (if present)
- Modify: `/Users/wanyaozhong/Projects/argusx/docs/plans/2026-03-05-agent-turn-event-bus-design.md`

**Step 1: Write failing documentation check**

```bash
rg -n "reducer" agent-turn/src agent-turn/README.md
```
Expected: old architecture references still present.

**Step 2: Run and verify fail condition**

Run: `rg -n "legacy reducer|reduce\(" agent-turn/src agent-turn/README.md`
Expected: references exist.

**Step 3: Write minimal implementation**

```text
Update module exports and architecture docs to event bus vocabulary.
Remove obsolete comments and helper APIs tied to monolithic reducer.
```

**Step 4: Run final verification**

Run: `cargo test -p agent-core -p agent-turn -p agent-session -- --nocapture`
Expected: PASS.

Run: `cargo fmt && cargo clippy -p agent-turn --all-targets -- -D warnings`
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-turn/src/lib.rs agent-turn/README.md docs/plans/2026-03-05-agent-turn-event-bus-design.md
git commit -m "docs(agent-turn): finalize event bus architecture documentation and cleanup"
```
