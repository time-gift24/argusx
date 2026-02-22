# Agent Tooling Unification Design

**Date:** 2026-02-22  
**Status:** Approved  
**Decision:** Use "Core defines contracts, Turn orchestrates demand, Tool implements execution" model.

## 1. Background

Current state in workspace:

- `agent-turn` has a complete event-driven loop and local tool execution trait.
- `agent-tool` is an independent registry/execution crate with built-in tools.
- `agent-turn` does not depend on `agent-tool` yet.
- `bigmodel` adapter currently does not emit structured `ToolCall` events from model responses.

Result: tool execution framework exists, but end-to-end tool calling is not unified.

## 2. Goals

1. Move all tool-related contracts from `agent-turn` into `agent-core`.
2. Keep `agent-turn` focused on orchestration: it only raises tool demand and consumes tool results.
3. Make `agent-tool` the concrete implementation of tool runtime and catalog.
4. Support end-to-end tool flow: model tool declaration -> tool call -> execution -> result injection.
5. Allow breaking changes in this migration (single cutover).

## 3. Non-goals

1. Implementing a full permission/sandbox/orchestrator system in this iteration.
2. Rewriting `agent-turn` reducer/effect architecture.
3. Designing plugin distribution protocol.

## 4. Architecture Decision

### 4.1 Module responsibilities

- `agent-core`
  - Owns all tool domain contracts (`tools` module).
  - Owns shared types and execution policy semantics.
- `agent-turn`
  - Owns turn loop, reducer, effects, and runtime event orchestration.
  - Depends only on `agent-core` tool contracts, not on concrete tool implementations.
- `agent-tool`
  - Implements `agent-core` tool contracts.
  - Provides registry, built-ins, and MCP-backed execution.
- Callers (`agent-turn-cli`, session runtime, future facade)
  - Wire model + turn runtime + tool runtime together.

### 4.2 Contract direction

- `agent-turn` -> depends on `agent-core::tools::{ToolExecutor, ToolCatalog, ...}`
- `agent-tool` -> implements `agent-core::tools` traits
- `agent-turn` does not import `agent-tool` directly (composition at caller layer)

## 5. Core Contract Design (`agent-core::tools`)

Add a `tools` module with:

1. `ToolSpec`
   - `name`
   - `description`
   - `input_schema` (JSON schema)
   - `execution_policy`
2. `ToolExecutionPolicy`
   - `parallel_mode`: `ParallelSafe | Exclusive`
   - `timeout_ms: Option<u64>`
   - `retry: Never | Transient { max_retries, backoff_ms }`
3. `ToolExecutionContext`
   - `session_id`
   - `turn_id`
   - `epoch`
   - `cwd: Option<PathBuf>`
4. `ToolExecutionError`
   - `kind`: `User | Runtime | Transient | Internal`
   - `message`
   - `retry_after_ms: Option<u64>`
5. Traits
   - `ToolCatalog`: exposes available `ToolSpec`s
   - `ToolExecutor`: executes a `ToolCall` under `ToolExecutionContext`
   - optional combined trait `ToolRuntime = ToolCatalog + ToolExecutor`

`ToolCall` and `ToolResult` stay as `agent-core` domain types.

## 6. End-to-end Data Flow

1. Caller constructs a tool runtime from `agent-tool` (registry + built-ins + optional MCP).
2. `agent-turn` starts a model step and requests tool specs from `ToolCatalog`.
3. Model adapter maps core tool specs to provider request fields (`tools`, `tool_choice`).
4. Provider stream is parsed into `ModelOutputEvent::ToolCall`.
5. Reducer stores inflight call and emits `Effect::ExecuteTool`.
6. Effect layer calls `ToolExecutor::execute_tool(...)`.
7. Tool result returns as `ToolResultOk` or `ToolResultErr` runtime event.
8. Reducer injects tool output into `InputEnvelope::tool_json(...)` and continues loop.

## 7. Concurrency Model

### 7.1 Current behavior to preserve

- Reducer is single-threaded state transition, never executes tools directly.
- Tool execution is asynchronous in effect tasks.

### 7.2 New policy-driven behavior

- Replace ad-hoc/implicit behavior with `ToolExecutionPolicy` per tool.
- Scheduler honors:
  - `ParallelSafe`: can run under shared concurrency slots.
  - `Exclusive`: serialized execution.
- Global cap (`max_parallel_tools`) remains as hard upper bound.

## 8. Failure Semantics

Tool execution outcomes:

1. `User` / `Runtime` failure
   - Emit `ToolResultErr`.
   - Keep turn alive; model receives error JSON for recovery.
2. `Transient` failure
   - Retry per tool policy in executor layer.
   - Exhausted retries emit `ToolResultErr`.
3. `Internal` failure
   - If localized to one call: emit `ToolResultErr`.
   - If runtime infrastructure is broken (e.g., channel closed): fail turn.

Invariants:

- Every accepted `call_id` must eventually produce one terminal result (`Ok` or `Err`).
- Unknown/late results are dropped with protocol warning, without corrupting state.

## 9. Migration Plan (Breaking, single cutover)

1. Add `agent-core::tools` contracts and export them.
2. Update `agent-turn` to consume core contracts; remove local tool trait definitions.
3. Implement core contracts in `agent-tool` (adapter over existing registry).
4. Update `agent-turn-cli` to inject `agent-tool` runtime instead of hardcoded echo executor.
5. Extend model request/response path:
   - request includes tool specs
   - response parser emits structured `ToolCall`
6. Remove obsolete duplicated types/usages.

## 10. Verification Strategy

1. Unit tests
   - core tools contracts and error mapping
   - agent-tool contract adapter
2. Reducer/effect tests
   - tool call lifecycle
   - inflight removal and result reinjection
   - concurrency rules (`ParallelSafe` vs `Exclusive`)
3. Adapter tests
   - tool spec serialization to provider format
   - provider response parsing into `ToolCall`
4. End-to-end test
   - model emits tool call -> tool executes -> model resumes -> turn done

## 11. Risks

1. Model provider schema mismatch for tool call payloads.
2. Hidden coupling in callers currently assuming local tool executor types.
3. Migration touches multiple crates; CI must validate cross-crate API consistency.

## 12. Acceptance Criteria

1. `agent-turn` contains no crate-local tool contract definitions.
2. `agent-turn` tool orchestration compiles only against `agent-core::tools`.
3. `agent-tool` provides concrete `ToolRuntime` implementation for callers.
4. At least one runtime path (`agent-turn-cli`) runs a real tool through `agent-tool` end-to-end.
