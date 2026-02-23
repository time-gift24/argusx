# Agent Tooling Unification Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Unify tool contracts in `agent-core`, make `agent-turn` orchestration-only against those contracts, and make `agent-tool` the concrete runtime adapter used by CLI/session flows.

**Architecture:** Introduce canonical tool contracts in `agent-core::tools`, then migrate `agent-turn` from local `ToolExecutor` trait to `agent-core` contracts (`ToolCatalog` + `ToolExecutor`). Add a runtime adapter in `agent-tool` that maps registry behavior and errors to core types. Finally, wire tool declarations/tool calls through BigModel request and stream paths, and keep compatibility with current `agent-session` while preparing for the new `agent` facade.

**Tech Stack:** Rust, Tokio, async-trait, serde/serde_json, workspace cargo tests

---

Execution rules:
- Use @superpowers:test-driven-development for every code change.
- Use @superpowers:verification-before-completion before any "done" claim.
- Keep commits small and frequent (one task, one commit).

Scope guardrails:
- In scope: tool contracts, adapter wiring, model tool request/response plumbing, scheduler policy.
- Out of scope: full `agent` crate introduction, session store refactor, migration scripts for session/checkpoint storage.
- Compatibility requirement: any temporary `agent-session` adjustments are compile/runtime shims only and should not deepen coupling.

### Task 1: Add canonical tool contracts in `agent-core`

**Files:**
- Create: `agent-core/src/tools/mod.rs`
- Modify: `agent-core/src/lib.rs`
- Test: `agent-core/tests/tools_contracts_test.rs`

**Step 1: Write the failing test**

```rust
// agent-core/tests/tools_contracts_test.rs
use agent_core::tools::{
    ToolCatalog, ToolExecutionContext, ToolExecutionErrorKind, ToolExecutionPolicy,
    ToolExecutor, ToolParallelMode, ToolSpec,
};

#[test]
fn default_policy_is_parallel_safe_without_retry() {
    let policy = ToolExecutionPolicy::default();
    assert!(matches!(policy.parallel_mode, ToolParallelMode::ParallelSafe));
    assert!(policy.timeout_ms.is_none());
    assert!(policy.retry.is_none());
}

#[test]
fn execution_context_roundtrip_json() {
    let ctx = ToolExecutionContext {
        session_id: "s1".into(),
        turn_id: "t1".into(),
        epoch: 3,
        cwd: None,
    };
    let raw = serde_json::to_string(&ctx).unwrap();
    assert!(raw.contains("\"session_id\""));
}

#[test]
fn tool_error_kind_debug_name_stable() {
    assert_eq!(format!("{:?}", ToolExecutionErrorKind::Transient), "Transient");
}

fn _assert_object_safe(_x: &dyn ToolCatalog, _y: &dyn ToolExecutor, _s: &ToolSpec) {}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-core --test tools_contracts_test -q`
Expected: FAIL (missing `agent_core::tools` module/types)

**Step 3: Write minimal implementation**

```rust
// agent-core/src/tools/mod.rs (shape)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolParallelMode { ParallelSafe, Exclusive }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolExecutionPolicy {
    pub parallel_mode: ToolParallelMode,
    pub timeout_ms: Option<u64>,
    pub retry: Option<ToolRetryPolicy>,
}

#[async_trait]
pub trait ToolCatalog: Send + Sync {
    async fn list_tools(&self) -> Vec<ToolSpec>;
    async fn tool_spec(&self, name: &str) -> Option<ToolSpec>;
}

#[async_trait]
pub trait ToolExecutor: Send + Sync {
    async fn execute_tool(
        &self,
        call: crate::ToolCall,
        ctx: ToolExecutionContext,
    ) -> Result<crate::ToolResult, ToolExecutionError>;
}
```

Also export `pub mod tools;` and re-exports in `agent-core/src/lib.rs`.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p agent-core --test tools_contracts_test -q`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-core/src/tools/mod.rs agent-core/src/lib.rs agent-core/tests/tools_contracts_test.rs
git commit -m "feat(agent-core): add canonical tool contracts"
```

### Task 2: Extend `ModelRequest` with tool declarations

**Files:**
- Modify: `agent-core/src/model.rs`
- Modify: `agent-turn/src/effect.rs`
- Modify: `agent-turn/src/adapters/bigmodel.rs`
- Test: `agent-core/tests/model_request_tools_test.rs`

**Step 1: Write the failing test**

```rust
// agent-core/tests/model_request_tools_test.rs
use agent_core::{InputEnvelope, ModelRequest};
use agent_core::tools::{ToolExecutionPolicy, ToolSpec};

#[test]
fn model_request_serializes_tools_field() {
    let req = ModelRequest {
        epoch: 0,
        transcript: vec![],
        inputs: vec![InputEnvelope::user_text("hi")],
        tools: vec![ToolSpec {
            name: "echo".to_string(),
            description: "echo args".to_string(),
            input_schema: serde_json::json!({"type":"object"}),
            execution_policy: ToolExecutionPolicy::default(),
        }],
    };

    let raw = serde_json::to_string(&req).unwrap();
    assert!(raw.contains("\"tools\""));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-core --test model_request_tools_test -q`
Expected: FAIL (`ModelRequest` missing `tools`)

**Step 3: Write minimal implementation**

```rust
// agent-core/src/model.rs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelRequest {
    pub epoch: u64,
    pub transcript: Vec<TranscriptItem>,
    pub inputs: Vec<InputEnvelope>,
    #[serde(default)]
    pub tools: Vec<crate::tools::ToolSpec>,
}
```

Update callsites to compile by setting `tools: Vec::new()` where `ModelRequest` is built.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p agent-core --test model_request_tools_test -q && cargo test -p agent-turn --lib -q`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-core/src/model.rs agent-core/tests/model_request_tools_test.rs agent-turn/src/effect.rs agent-turn/src/adapters/bigmodel.rs
git commit -m "feat(agent-core): add tool declarations to model request"
```

### Task 3: Implement `agent-tool` runtime adapter for core contracts

**Files:**
- Modify: `agent-tool/Cargo.toml`
- Create: `agent-tool/src/runtime.rs`
- Modify: `agent-tool/src/lib.rs`
- Test: `agent-tool/tests/runtime_adapter_test.rs`

**Step 1: Write the failing test**

```rust
// agent-tool/tests/runtime_adapter_test.rs
use agent_core::tools::{ToolCatalog, ToolExecutionContext, ToolExecutor};
use agent_tool::AgentToolRuntime;

#[tokio::test]
async fn runtime_adapter_executes_registered_builtin_tool() {
    let rt = AgentToolRuntime::with_default_builtins().await;

    let out = rt
        .execute_tool(
            agent_core::ToolCall::new("shell", serde_json::json!({"command":"echo ok"})),
            ToolExecutionContext {
                session_id: "s1".into(),
                turn_id: "t1".into(),
                epoch: 0,
                cwd: None,
            },
        )
        .await
        .expect("tool should run");

    assert!(!out.is_error);
}

#[tokio::test]
async fn runtime_adapter_lists_builtin_specs() {
    let rt = AgentToolRuntime::with_default_builtins().await;
    let tools = rt.list_tools().await;
    assert!(tools.iter().any(|t| t.name == "shell"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-tool --test runtime_adapter_test -q`
Expected: FAIL (`AgentToolRuntime` missing)

**Step 3: Write minimal implementation**

```rust
// agent-tool/src/runtime.rs (shape)
pub struct AgentToolRuntime {
    registry: ToolRegistry,
}

impl AgentToolRuntime {
    pub async fn with_default_builtins() -> Self {
        let registry = ToolRegistry::new();
        registry.register(ReadFileTool).await;
        registry.register(ShellTool).await;
        Self { registry }
    }
}

#[async_trait]
impl agent_core::tools::ToolCatalog for AgentToolRuntime { /* list_tools/tool_spec */ }

#[async_trait]
impl agent_core::tools::ToolExecutor for AgentToolRuntime {
    async fn execute_tool(
        &self,
        call: agent_core::ToolCall,
        ctx: agent_core::tools::ToolExecutionContext,
    ) -> Result<agent_core::ToolResult, agent_core::tools::ToolExecutionError> {
        // map ctx -> agent_tool::ToolContext, execute via registry.call
    }
}
```

Expose `pub use runtime::AgentToolRuntime;` in `agent-tool/src/lib.rs`.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p agent-tool --test runtime_adapter_test -q`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-tool/Cargo.toml agent-tool/src/runtime.rs agent-tool/src/lib.rs agent-tool/tests/runtime_adapter_test.rs
git commit -m "feat(agent-tool): add core-contract runtime adapter"
```

### Task 4: Migrate `agent-turn` to core tool contracts

**Files:**
- Modify: `agent-turn/src/effect.rs`
- Modify: `agent-turn/src/engine.rs`
- Modify: `agent-turn/src/runtime_impl.rs`
- Modify: `agent-turn/src/reducer.rs`
- Test: `agent-turn/tests/tool_contract_migration_test.rs`

**Step 1: Write the failing test**

```rust
// agent-turn/tests/tool_contract_migration_test.rs
use std::sync::Arc;

use async_trait::async_trait;
use futures::stream;
use agent_core::{AgentError, ModelEventStream, ModelRequest};
use agent_core::tools::{ToolCatalog, ToolExecutor};
use agent_turn::TurnRuntime;

struct DummyModel;
struct DummyTools;

#[async_trait]
impl agent_core::LanguageModel for DummyModel {
    fn model_name(&self) -> &str { "dummy" }

    async fn stream(&self, _request: ModelRequest) -> Result<ModelEventStream, AgentError> {
        Ok(Box::pin(stream::empty()))
    }
}

#[async_trait]
impl ToolCatalog for DummyTools {
    async fn list_tools(&self) -> Vec<agent_core::tools::ToolSpec> { vec![] }
    async fn tool_spec(&self, _name: &str) -> Option<agent_core::tools::ToolSpec> { None }
}

#[async_trait]
impl ToolExecutor for DummyTools {
    async fn execute_tool(
        &self,
        call: agent_core::ToolCall,
        _ctx: agent_core::tools::ToolExecutionContext,
    ) -> Result<agent_core::ToolResult, agent_core::tools::ToolExecutionError> {
        Ok(agent_core::ToolResult::ok(call.call_id, serde_json::json!({"ok": true})))
    }
}

#[test]
fn turn_runtime_accepts_core_tool_contract_impls() {
    let _runtime = TurnRuntime::new(
        Arc::new(DummyModel),
        Arc::new(DummyTools),
        agent_turn::TurnEngineConfig::default(),
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn --test tool_contract_migration_test -q`
Expected: FAIL (still bound to local `agent_turn::effect::ToolExecutor`)

**Step 3: Write minimal implementation**

```rust
// agent-turn/src/effect.rs (key changes)
// 1) remove local ToolExecutor trait
// 2) use agent_core::tools::{ToolCatalog, ToolExecutionContext, ToolExecutor}
// 3) Effect::ExecuteTool carries session_id + turn_id + epoch + call
// 4) spawn_tool_execution builds ToolExecutionContext from effect payload
```

```rust
// agent-turn/src/reducer.rs (key shape)
tr.add_effect(Effect::ExecuteTool {
    session_id: tr.state.meta.session_id.clone(),
    turn_id: tr.state.meta.turn_id.clone(),
    epoch,
    call,
});
```

Update `engine.rs` and `runtime_impl.rs` generic bounds to `T: ToolExecutor + ToolCatalog + 'static`.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p agent-turn --lib -q`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-turn/src/effect.rs agent-turn/src/engine.rs agent-turn/src/runtime_impl.rs agent-turn/src/reducer.rs agent-turn/tests/tool_contract_migration_test.rs
git commit -m "refactor(turn): consume core tool contracts"
```

### Task 5: Enforce policy-driven tool concurrency in scheduler

**Files:**
- Modify: `agent-turn/src/effect.rs`
- Test: `agent-turn/src/effect.rs` (existing test module)

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn exclusive_tools_are_serialized_even_with_parallel_slots() {
    // Given max_parallel_tools > 1 and tool policy = Exclusive
    // Then observed max concurrent executions for that tool is 1
    assert_eq!(max_seen.load(std::sync::atomic::Ordering::SeqCst), 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn effect::tests::exclusive_tools_are_serialized_even_with_parallel_slots -q`
Expected: FAIL (current scheduler only has global semaphore)

**Step 3: Write minimal implementation**

```rust
// effect executor shape:
// - keep global semaphore
// - read ToolSpec via ToolCatalog::tool_spec(call.tool_name)
// - if policy == Exclusive, acquire per-tool mutex before execute_tool
// - release mutex + semaphore after execution
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p agent-turn --lib -q`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-turn/src/effect.rs
git commit -m "feat(turn): enforce policy-driven tool concurrency"
```

### Task 6: Inject tool declarations from catalog into model requests

**Files:**
- Modify: `agent-turn/src/effect.rs`
- Test: `agent-turn/src/effect.rs` (existing test module)

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn start_model_request_contains_catalog_tools() {
    // Given catalog contains shell/read_file
    // When Effect::StartModel is executed
    // Then LanguageModel::stream receives request.tools with those specs
    assert!(captured_request.tools.iter().any(|t| t.name == "shell"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn effect::tests::start_model_request_contains_catalog_tools -q`
Expected: FAIL (`ModelRequest.tools` not populated by effect executor)

**Step 3: Write minimal implementation**

```rust
// in spawn_model_stream async block
let tools = tools_catalog.list_tools().await;
let request = ModelRequest {
    epoch,
    transcript,
    inputs,
    tools,
};
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p agent-turn --lib -q`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-turn/src/effect.rs
git commit -m "feat(turn): include tool specs in model requests"
```

### Task 7: Map core `ToolSpec` into BigModel request tools

**Files:**
- Modify: `agent-turn/src/adapters/bigmodel.rs`
- Test: `agent-turn/src/adapters/bigmodel.rs` (existing test module)

**Step 1: Write the failing test**

```rust
#[test]
fn convert_request_includes_function_tools() {
    let req = ModelRequest {
        epoch: 0,
        transcript: vec![],
        inputs: vec![InputEnvelope::user_text("hi")],
        tools: vec![agent_core::tools::ToolSpec {
            name: "shell".into(),
            description: "run shell".into(),
            input_schema: serde_json::json!({"type":"object"}),
            execution_policy: agent_core::tools::ToolExecutionPolicy::default(),
        }],
    };

    let out = convert_model_request(req, &BigModelAdapterConfig::default());
    assert!(out.tools.is_some());
    assert_eq!(out.tools.unwrap().len(), 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn adapters::bigmodel::tests::convert_request_includes_function_tools -q`
Expected: FAIL (adapter ignores `request.tools`)

**Step 3: Write minimal implementation**

```rust
// convert_model_request
let tools = request
    .tools
    .into_iter()
    .map(core_spec_to_bigmodel_function_tool)
    .collect::<Vec<_>>();

if !tools.is_empty() {
    chat_request = chat_request.tools(tools);
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p agent-turn --lib -q`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-turn/src/adapters/bigmodel.rs
git commit -m "feat(turn): map tool specs to BigModel request"
```

### Task 8: Parse BigModel stream tool calls into `ModelOutputEvent::ToolCall`

**Files:**
- Modify: `bigmodel-api/src/models.rs`
- Modify: `agent-turn/src/adapters/bigmodel.rs`
- Test: `agent-turn/src/adapters/bigmodel.rs` (existing test module)
- Test: `bigmodel-api/tests/models.rs`

**Step 1: Write the failing tests**

```rust
// agent-turn/src/adapters/bigmodel.rs tests
#[test]
fn stream_chunk_emits_tool_call_event() {
    let chunk = make_tool_call_chunk("c1", "shell", r#"{"command":"echo ok"}"#);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    emit_chunk(chunk, &tx);
    let item = rx.try_recv().unwrap().unwrap();
    assert!(matches!(item, ModelOutputEvent::ToolCall { .. }));
}
```

```rust
// bigmodel-api/tests/models.rs
#[test]
fn delta_with_tool_calls_deserializes() {
    let raw = r#"{"role":"assistant","tool_calls":[{"id":"c1","type":"function","function":{"name":"shell","arguments":"{}"}}]}"#;
    let _: bigmodel_api::Delta = serde_json::from_str(raw).unwrap();
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p bigmodel-api -q && cargo test -p agent-turn adapters::bigmodel::tests::stream_chunk_emits_tool_call_event -q`
Expected: FAIL (delta schema/tool-call parsing missing)

**Step 3: Write minimal implementation**

```rust
// bigmodel-api/src/models.rs
pub struct Delta {
    pub role: Option<String>,
    pub content: Option<String>,
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<DeltaToolCall>>,
}
```

```rust
// adapter emit_chunk
// map delta.tool_calls[*] => ModelOutputEvent::ToolCall
// parse function.arguments JSON string to serde_json::Value (fallback to string wrapper)
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p bigmodel-api -q && cargo test -p agent-turn --lib -q`
Expected: PASS

**Step 5: Commit**

```bash
git add bigmodel-api/src/models.rs bigmodel-api/tests/models.rs agent-turn/src/adapters/bigmodel.rs
git commit -m "feat(turn): parse streamed tool calls from BigModel deltas"
```

### Task 9: Wire callers to `AgentToolRuntime` and keep `agent-session` compatible

**Files:**
- Modify: `agent-turn-cli/Cargo.toml`
- Modify: `agent-turn-cli/src/main.rs`
- Modify: `agent-session/src/session_runtime.rs`
- Modify: `agent-session/tests/integration.rs`

**Step 1: Write failing compile/behavior checks**

```rust
// agent-turn-cli/src/main.rs test module
#[tokio::test]
async fn cli_can_build_agent_tool_runtime() {
    let rt = agent_tool::AgentToolRuntime::with_default_builtins().await;
    assert!(!rt.list_tools().await.is_empty());
}
```

```rust
// agent-session/tests/integration.rs
// adapt mock to satisfy new bounds (ToolExecutor + ToolCatalog)
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p agent-turn-cli -q && cargo test -p agent-session --lib -q`
Expected: FAIL (dependency/generic bounds not updated)

**Step 3: Write minimal implementation**

```rust
// agent-turn-cli/src/main.rs (shape)
let tools = Arc::new(agent_tool::AgentToolRuntime::with_default_builtins().await);
let runtime = TurnRuntime::new(model, tools, runtime_cfg);
```

Update `agent-session` bounds to match new `TurnRuntime` requirements and patch test mocks accordingly.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p agent-turn-cli -q && cargo test -p agent-session --lib -q`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-turn-cli/Cargo.toml agent-turn-cli/src/main.rs agent-session/src/session_runtime.rs agent-session/tests/integration.rs
git commit -m "refactor(callers): use AgentToolRuntime and core tool bounds"
```

### Task 10: Normalize adapter error mapping and invariants

**Files:**
- Modify: `agent-tool/src/runtime.rs`
- Test: `agent-tool/tests/runtime_adapter_test.rs`
- Test: `agent-turn/src/reducer.rs` (existing tests)

**Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn unknown_tool_maps_to_user_error_kind() {
    // execute missing tool and assert ToolExecutionErrorKind::User
}

#[tokio::test]
async fn shell_nonzero_exit_keeps_runtime_error_shape() {
    // execute shell command that exits 1 and assert deterministic mapping
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p agent-tool --test runtime_adapter_test -q`
Expected: FAIL (mapping incomplete/unstable)

**Step 3: Write minimal implementation**

```rust
// runtime adapter mapping
// ToolError::NotFound | ToolError::InvalidArgs => User
// ToolError::Io => Runtime (or Transient when retryable signal exists)
// ToolError::ExecutionFailed => Runtime
```

Keep reducer invariant unchanged: tool error still removes inflight call and reinjects tool JSON input.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p agent-tool -q && cargo test -p agent-turn --lib -q`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-tool/src/runtime.rs agent-tool/tests/runtime_adapter_test.rs agent-turn/src/reducer.rs
git commit -m "fix(tools): normalize adapter error mapping"
```

### Task 11: Full verification, lint, and doc sync

**Files:**
- Modify: `agent-core/src/lib.rs` (exports cleanup, if needed)
- Modify: touched files from prior tasks
- Modify: `docs/plans/2026-02-22-agent-crate-design.md` (only if contract names changed)

**Step 1: Run targeted crate tests**

Run: `cargo test -p agent-core -q && cargo test -p agent-tool -q && cargo test -p agent-turn --lib -q && cargo test -p agent-session --lib -q && cargo test -p agent-turn-cli -q`
Expected: PASS

**Step 2: Run lint checks**

Run: `cargo clippy -p agent-core -p agent-tool -p agent-turn -p agent-session -p agent-turn-cli --all-targets --all-features -- -D warnings`
Expected: PASS

**Step 3: Run CLI smoke test**

Run: `cargo run -p agent-turn-cli -- --help`
Expected: command prints usage, exits cleanly

**Step 4: Sync docs references**

```markdown
Update plan/docs references that still mention `agent_turn::effect::ToolExecutor` as the primary contract.
```

**Step 5: Commit**

```bash
git add -A
git commit -m "chore: finalize agent tooling unification"
```

---

Plan complete and saved to `docs/plans/2026-02-22-agent-tooling-unification-implementation-plan.md`. Two execution options:

1. **Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

2. **Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

Which approach?
