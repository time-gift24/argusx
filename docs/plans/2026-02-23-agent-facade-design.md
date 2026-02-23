# Agent Facade Design

**Date:** 2026-02-23  
**Status:** Confirmed  
**Owner:** Runtime team

## 1. Goal

Create a new `agent` crate as the only recommended external entry point.

The new crate must provide a complete facade API while keeping:
- `agent-turn` tests and responsibilities inside `agent-turn`
- `agent-session` tests and responsibilities inside `agent-session`
- `agent-tool` as the tool runtime implementation

This design reduces caller assembly complexity and stabilizes public API boundaries.

## 2. Scope

In scope for v1:
- New `agent` crate with `Agent` and `AgentBuilder`
- Full high-level API:
  - Session management: `create/list/get/delete_session`
  - Turn execution: `chat/chat_stream`
  - Turn control: `inject_input/cancel_turn`
- Hybrid dependency injection:
  - `model` required
  - `tools`, session store, checkpoint store optional
  - default file-based storage when optional dependencies are omitted
- Non-breaking migration path for existing CLI crates

Out of scope for v1:
- Removing existing low-level crates or their tests
- Introducing a new storage backend
- MCP production implementation in `agent-tool`
- Long-term removal of legacy assembly paths

## 3. Final Decisions

1. Public API style: facade-only (`Agent`/`AgentBuilder`), no re-export of low-level runtimes.
2. Coverage: full facade API in v1.
3. Injection strategy: Hybrid.
4. Migration strategy: smooth migration, keep old wiring paths temporarily.

## 4. Architecture and Boundaries

### 4.1 Crate responsibilities

- `agent-core`
  - Shared contracts, model/events/tool domain types.
- `agent-turn`
  - Single-turn runtime engine (reducer + effects).
- `agent-session`
  - Session lifecycle, turn summary persistence, transcript recovery wiring.
- `agent-tool`
  - Tool catalog + execution runtime implementation.
- `agent` (new)
  - Public facade and wiring composition.

### 4.2 Dependency direction

- `agent` depends on `agent-core`, `agent-turn`, `agent-session`, `agent-tool`.
- Existing crates keep their current boundaries.
- Callers should depend only on `agent` after migration.

## 5. Components in `agent` Crate

- `builder.rs`
  - `AgentBuilder` and builder validation.
- `agent.rs`
  - `Agent` API methods and runtime calls.
- `config.rs`
  - Facade-level config and defaults.
- `error.rs`
  - `AgentFacadeError` and mapping utilities.
- `wiring.rs` (internal)
  - Default runtime/store/tool assembly.

## 6. Public API (Facade)

```rust
pub struct Agent<L>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
{
    // internal fields omitted
}

pub struct AgentBuilder<L>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
{
    // required
    model: Option<std::sync::Arc<L>>,

    // optional
    tools: Option<std::sync::Arc<agent_tool::AgentToolRuntime>>,
    store_dir: Option<std::path::PathBuf>,
    max_parallel_tools: usize,
}

impl<L> AgentBuilder<L>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
{
    pub fn new() -> Self;
    pub fn model(self, model: std::sync::Arc<L>) -> Self;
    pub fn tools(self, tools: std::sync::Arc<agent_tool::AgentToolRuntime>) -> Self;
    pub fn store_dir(self, path: std::path::PathBuf) -> Self;
    pub fn max_parallel_tools(self, value: usize) -> Self;
    pub async fn build(self) -> Result<Agent<L>, AgentFacadeError>;
}

impl<L> Agent<L>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
{
    pub async fn create_session(
        &self,
        user_id: Option<String>,
        title: Option<String>,
    ) -> Result<String, AgentFacadeError>;

    pub async fn list_sessions(
        &self,
        filter: agent_session::SessionFilter,
    ) -> Result<Vec<agent_core::SessionInfo>, AgentFacadeError>;

    pub async fn get_session(
        &self,
        session_id: &str,
    ) -> Result<Option<agent_core::SessionInfo>, AgentFacadeError>;

    pub async fn delete_session(&self, session_id: &str) -> Result<(), AgentFacadeError>;

    pub async fn chat(
        &self,
        session_id: &str,
        message: &str,
    ) -> Result<ChatResponse, AgentFacadeError>;

    pub async fn chat_stream(
        &self,
        session_id: &str,
        message: &str,
    ) -> Result<AgentStream, AgentFacadeError>;

    pub async fn inject_input(
        &self,
        turn_id: &str,
        input: agent_core::InputEnvelope,
    ) -> Result<(), AgentFacadeError>;

    pub async fn cancel_turn(
        &self,
        turn_id: &str,
        reason: Option<String>,
    ) -> Result<(), AgentFacadeError>;
}
```

## 7. Data Flow and State Model

### 7.1 `chat`

1. Validate input and session id.
2. Build `TurnRequest` with `SessionMeta + InputEnvelope::user_text`.
3. Call runtime `run_turn`.
4. Consume run/ui streams internally.
5. Return `ChatResponse` with final text, stats, turn metadata.

### 7.2 `chat_stream`

1. Same turn startup path as `chat`.
2. Bridge `RunEventStream` and `UiEventStream` into facade stream type.
3. Emit stable facade events only.
4. Stop on terminal event (`Done` or `Error`) and close channels safely.

### 7.3 State ownership

- Source of truth for turn execution state remains in `agent-turn::TurnState`.
- Source of truth for session state remains in `agent-session`.
- `agent` stores no duplicate business state; it only coordinates calls and streams.

## 8. Error Handling

Define `AgentFacadeError` categories:
- `InvalidInput`
- `Busy`
- `Transient { retry_after_ms }`
- `Execution`
- `Internal`

Mapping rules:
- `TransientError` -> `Transient`
- Session active conflict -> `Busy`
- Tool/model execution failures -> `Execution`
- Storage/runtime unexpected failures -> `Internal`

Edge behavior:
- Turn-start failures must rollback active session state.
- Stream bridge must release tasks/channels on downstream drop.
- Tool error is not equal to turn failure; only terminal turn events decide final status.

## 9. Testing Strategy

### 9.1 Keep existing tests where they belong

- `agent-turn`: reducer/effect runtime behavior.
- `agent-session`: persistence and session lifecycle behavior.
- `agent-tool`: catalog/executor adapter behavior.

### 9.2 Add facade-focused tests in `agent`

- Builder validation and default wiring behavior.
- Error mapping coverage.
- `chat` happy path with mock model/tool runtime.
- `chat_stream` terminal event behavior.
- Busy session path.
- `inject_input/cancel_turn` forwarding checks.

### 9.3 E2E sanity in facade

Use temporary file store:
- create session -> chat stream -> done
- verify session list and summary visibility

## 10. Rollout and Migration Plan

1. Add `agent` crate and compile-only integration.
2. Implement full facade API and tests.
3. Migrate `agent-turn-cli` to use `agent`.
4. Migrate `agent-session-cli` to use `agent`.
5. Keep old low-level assembly paths temporarily (deprecate in docs).
6. After stability window, decide deprecation removal in separate change.

## 11. Acceptance Criteria

- New `agent` crate exists and is documented as the recommended entry point.
- Full facade API compiles and is tested.
- Existing crate-local tests remain in place and passing.
- CLI migration works without breaking behavior.
- Workspace checks pass:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features`
  - `cargo test --workspace --all-features`

