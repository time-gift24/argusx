# Agent Facade Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a new `agent` crate as the single high-level public API while keeping tests in `agent-turn` and `agent-session`.

**Architecture:** Build a facade-only crate that composes `agent-session` + `agent-turn` + `agent-tool`. Use Hybrid injection: model required, tools/store optional with sane defaults.

**Tech Stack:** Rust, tokio, async-trait, futures, thiserror

---

### Task 1: Scaffold `agent` crate and workspace integration

**Files:**
- Modify: `Cargo.toml`
- Create: `agent/Cargo.toml`
- Create: `agent/src/lib.rs`

**Step 1:** Add `agent` to workspace members.

**Step 2:** Create crate manifest with dependencies on `agent-core`, `agent-session`, `agent-tool`, `tokio`, `futures`, `tokio-stream`, `thiserror`.

**Step 3:** Create empty module exports for `agent`, `builder`, `config`, `error`, `types`.

**Step 4:** Run compile check for new crate.

### Task 2: Write failing facade tests (RED)

**Files:**
- Create: `agent/tests/facade_api_test.rs`

**Step 1:** Add tests for:
- builder requires model
- builder can use default tools when not provided
- create session + chat returns assistant output
- inject/cancel not-found behavior maps correctly

**Step 2:** Run tests and confirm failures.

### Task 3: Implement facade types and error mapping (GREEN)

**Files:**
- Create: `agent/src/error.rs`
- Create: `agent/src/config.rs`
- Create: `agent/src/types.rs`
- Update: `agent/src/lib.rs`

**Step 1:** Add `AgentFacadeError` with categories: `InvalidInput`, `Busy`, `Transient`, `Execution`, `Internal`.

**Step 2:** Add config defaults (`store_dir`, `max_parallel_tools`).

**Step 3:** Add public facade response/stream types.

### Task 4: Implement `AgentBuilder` and `Agent` facade

**Files:**
- Create: `agent/src/builder.rs`
- Create: `agent/src/agent.rs`

**Step 1:** `AgentBuilder` with required `model`, optional `tools`, optional `store_dir`, and `max_parallel_tools`.

**Step 2:** `build()` creates `SessionRuntime` with default `AgentToolRuntime::default_with_builtins()` when tools missing.

**Step 3:** Implement facade methods:
- `create_session`, `list_sessions`, `get_session`, `delete_session`
- `chat`, `chat_stream`, `inject_input`, `cancel_turn`

**Step 4:** Keep facade-only exports; do not re-export low-level runtimes.

### Task 5: Make tests pass and refine behavior (REFACTOR)

**Files:**
- Update: `agent/tests/facade_api_test.rs`
- Update: `agent/src/*.rs`

**Step 1:** Run crate tests until green.

**Step 2:** Refactor small helpers without behavior changes.

### Task 6: Workspace-level verification

**Files:**
- N/A

**Step 1:** Run:
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features`
- `cargo test --workspace --all-features`

**Step 2:** Fix any regressions.

### Task 7: Commit

**Files:**
- All changed files

**Step 1:** Commit with message:
`feat(agent): add facade crate with hybrid builder and session/chat APIs`

