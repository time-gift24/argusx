# Agent-Turn Event Bus Refactor Design

## Context
`agent-turn` currently centers around a very large reducer that directly couples:
- runtime event parsing
- state mutation
- run/ui projection
- effect scheduling

This has made behavior hard to reason about, hard to evolve, and expensive to refactor safely.

## Goals
- Replace the monolithic reducer model with an event bus architecture.
- Separate intent (`DomainCommand`) from facts (`DomainEvent`) and outward outputs (`OutputEvent`).
- Move state mutation to explicit projectors.
- Reduce duplicated and noisy events by normalization and merging.
- Keep deterministic turn processing in a single core loop.

## Non-Goals
- Multi-node distributed event bus.
- Reworking model/tool adapters in the same phase.
- UI redesign or protocol polish outside event contract changes required for the refactor.

## Constraints and Decisions
- Refactor mode: one-shot cutover.
- Behavior flexibility: high (`C`), as long as core turn semantics remain valid.
- External event contracts may be changed if duplication/noise is reduced.

## Target Architecture

### 1. Event Model Layers
- `DomainCommand`: external inputs and control intents.
- `DomainEvent`: normalized internal facts.
- `OutputEvent`: side-effect and client-facing outputs.

### 2. Event Bus Core
- `EventBus` owns bounded queues.
- Core pump loop: `Command -> Handler -> DomainEvent -> Projector -> OutputEvent`.
- Ordering is deterministic within a turn.

### 3. Handler Registry
- Replace one giant `match` with domain handlers:
- `model_handler`
- `tool_handler`
- `input_handler`
- `lifecycle_handler`
- `subagent_handler`
- `checkpoint_handler`

Each handler consumes relevant `DomainCommand` variants and emits zero or more `DomainEvent` values.

### 4. Projectors
- `StateProjector`: the only place that mutates `TurnState`.
- `OutputProjector`: maps `DomainEvent` to `RunStreamEvent`, `UiThreadEvent`, and `Effect`.
- `CheckpointProjector`: handles append/snapshot triggers based on event milestones.

### 5. Normalization and Dedup
- Add `CommandNormalizer` before handlers.
- Merge duplicate protocol-level events.
- Keep event-id idempotency here, not in scattered business logic.

## Data Flow Contract
- Input ingress: runtime/api/timer messages converted to `DomainCommand`.
- Command normalization: deduplication and optional coalescing.
- Handler phase: commands become domain facts.
- Project phase: facts mutate state and generate outputs.
- Dispatch phase: outputs route to run/ui channels and effect executor.

## Backpressure and Queueing
- Use bounded command/event queues.
- Control commands (`cancel`, `retry`) are prioritized.
- High-volume deltas (stdout/stderr) can be coalesced with loss accounting.
- Preserve deterministic processing while preventing unbounded memory growth.

## Error Handling Model
- `CommandError`: input/protocol issue, emit warning and drop command.
- `HandlerError`: domain rule issue, convert to failure domain event.
- `ProjectError`: invariant break, fail-fast current turn.
- `DispatchError`: output sink issue, emit warning and continue if possible.

## Observability
- Every domain event carries `turn_id`, `epoch`, `trace_id`, `caused_by`.
- Add bus metrics:
- queue depth
- dropped/coalesced event counters
- handler latency
- projector failures

## File Layout
- `agent-turn/src/bus/mod.rs`
- `agent-turn/src/command/*.rs`
- `agent-turn/src/domain/*.rs`
- `agent-turn/src/handlers/*.rs`
- `agent-turn/src/projectors/*.rs`
- `agent-turn/src/runtime_impl.rs` (wiring only)

## One-Shot Migration Plan
- Step 1: introduce new event type system and bus skeleton.
- Step 2: implement command normalizer.
- Step 3: implement domain handlers.
- Step 4: implement state and output projectors.
- Step 5: switch runtime wiring to bus pipeline.
- Step 6: remove legacy reducer main path and redundant event variants.
- Step 7: clean dead code and simplify tests.

## Progress Snapshot (2026-03-05)
- Runtime ingress is bus-first (`RuntimeEvent -> DomainCommand -> DomainEvent -> OutputEvent`).
- Redundant tool queue/dequeue contracts were removed from `RuntimeEvent`, `RunStreamEvent`, and `UiThreadEvent`.
- Tool lifecycle status now flows through `ToolCallRequested` + `ToolCallProgress` (`Planned`/`Running`/terminal).
- Legacy reducer code remains as a compatibility bridge for unmigrated command paths; full removal is still pending.

## Test Strategy
- Golden trace comparison for representative turn flows.
- Unit tests per handler (`Command -> DomainEvent`).
- Unit tests per projector (`DomainEvent -> state/output`).
- Bus integration tests for ordering and queue behavior.
- Stress tests for high-rate tool output.
- Recovery tests for checkpoint + replay consistency.

## Risks
- Event ordering changes may alter UI behavior.
- One-shot cutover increases integration risk.
- Event contract edits can ripple across crates.

## Risk Controls
- Explicit ordering contract per domain event class.
- Golden trace gating before cutover.
- Cross-crate compile gates and integration tests before merge.

## Approved Scope
This document reflects reviewed decisions:
- One-shot refactor.
- Event bus architecture (approach 1).
- Contract changes allowed.
- Duplicate/noisy events should be consolidated.
