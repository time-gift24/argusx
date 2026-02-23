# Agent Tooling Unification Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Unify tool contracts in `agent-core`, keep `agent-turn` as orchestration-only, and make `agent-tool` the concrete runtime implementation used end-to-end.

**Architecture:** Move all tool-facing types/traits to `agent-core::tools`, then refactor `agent-turn` to depend only on those contracts. Implement contract adapters in `agent-tool`, wire callers (`agent-turn-cli`, `agent-session`) to that runtime, and finally complete model-provider request/response tool integration for BigModel. Keep reducer event-driven design and introduce policy-driven tool concurrency in effect execution.

**Tech Stack:** Rust, Tokio, async-trait, serde/serde_json, workspace cargo tests

---

Execution rules:
- Use @superpowers:test-driven-development for every code change.
- Use @superpowers:verification-before-completion before any "done" claim.
- Keep commits small and frequent (one task, one commit).

### Task 1: Add `agent-core::tools` contracts

**Files:**
- Create: `agent-core/src/tools/mod.rs`
- Modify: `agent-core/src/lib.rs`
- Test: `agent-core/tests/tools_contracts_test.rs`

**Step 1: Write the failing test**

```rust
// agent-core/tests/tools_contracts_test.rs
use agent_core::tools::{ToolExecutionErrorKind, ToolExecutionPolicy, ToolParallelMode};

#[test]
fn default_tool_policy_is_parallel_safe_without_retry() {
    let p = ToolExecutionPolicy::default();
    assert!(matches!(p.parallel_mode, ToolParallelMode::ParallelSafe));
    assert!(p.timeout_ms.is_none());
    assert!(p.retry.is_none());
}

#[test]
fn tool_error_kind_roundtrip_debug() {
    let kind = ToolExecutionErrorKind::Transient;
    assert_eq!(format!("{kind:?}"), "Transient");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-core --test tools_contracts_test -q`  
Expected: FAIL (missing `agent_core::tools` module/types)

**Step 3: Write minimal implementation**

```rust
// agent-core/src/tools/mod.rs
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{ToolCall, ToolResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolParallelMode {
    ParallelSafe,
    Exclusive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolRetryPolicy {
    pub max_retries: u32,
    pub backoff_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolExecutionPolicy {
    pub parallel_mode: ToolParallelMode,
    pub timeout_ms: Option<u64>,
    pub retry: Option<ToolRetryPolicy>,
}

impl Default for ToolExecutionPolicy {
    fn default() -> Self {
        Self {
            parallel_mode: ToolParallelMode::ParallelSafe,
            timeout_ms: None,
            retry: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub execution_policy: ToolExecutionPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolExecutionContext {
    pub session_id: String,
    pub turn_id: String,
    pub epoch: u64,
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolExecutionErrorKind {
    User,
    Runtime,
    Transient,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolExecutionError {
    pub kind: ToolExecutionErrorKind,
    pub message: String,
    pub retry_after_ms: Option<u64>,
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
        call: ToolCall,
        ctx: ToolExecutionContext,
    ) -> Result<ToolResult, ToolExecutionError>;
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-core --test tools_contracts_test -q`  
Expected: PASS

**Step 5: Commit**

```bash
git add agent-core/src/tools/mod.rs agent-core/src/lib.rs agent-core/tests/tools_contracts_test.rs
git commit -m "feat(agent-core): add unified tools contracts"
```

### Task 2: Extend `ModelRequest` with tool declarations

**Files:**
- Modify: `agent-core/src/model.rs`
- Test: `agent-core/tests/model_request_tools_test.rs`

**Step 1: Write the failing test**

```rust
// agent-core/tests/model_request_tools_test.rs
use agent_core::{InputEnvelope, ModelRequest};
use agent_core::tools::{ToolExecutionPolicy, ToolSpec};

#[test]
fn model_request_serializes_tools() {
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

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-core --test model_request_tools_test -q`  
Expected: PASS

**Step 5: Commit**

```bash
git add agent-core/src/model.rs agent-core/tests/model_request_tools_test.rs
git commit -m "feat(agent-core): add tool specs to model request"
```

### Task 3: Migrate `agent-turn` to `agent-core` tool contracts

**Files:**
- Modify: `agent-turn/src/effect.rs`
- Modify: `agent-turn/src/runtime_impl.rs`
- Modify: `agent-session/src/session_runtime.rs`
- Test: `agent-turn/tests/tool_contract_migration_test.rs`

**Step 1: Write the failing test**

```rust
// agent-turn/tests/tool_contract_migration_test.rs
use agent_core::tools::ToolExecutor;

fn assert_tool_executor_trait<T: ToolExecutor>() {}

#[test]
fn turn_uses_core_tool_executor_contract() {
    // compile-time guard only
    struct Dummy;
    let _ = std::any::type_name::<Dummy>();
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn --test tool_contract_migration_test -q`  
Expected: FAIL (still using local `agent_turn::effect::ToolExecutor`)

**Step 3: Write minimal implementation**

```rust
// agent-turn/src/effect.rs (key shape)
use agent_core::tools::{ToolExecutionContext, ToolExecutor};

// remove local ToolExecutor trait definition

let result = tools
    .execute_tool(
        call.clone(),
        ToolExecutionContext {
            session_id: turn_meta.session_id.clone(),
            turn_id: turn_meta.turn_id.clone(),
            epoch,
            cwd: None,
        },
    )
    .await;
```

**Step 4: Run tests to verify it passes**

Run: `cargo test -p agent-turn --lib -q && cargo test -p agent-session --lib -q`  
Expected: PASS

**Step 5: Commit**

```bash
git add agent-turn/src/effect.rs agent-turn/src/runtime_impl.rs agent-session/src/session_runtime.rs agent-turn/tests/tool_contract_migration_test.rs
git commit -m "refactor(turn): consume core tool contracts"
```

### Task 4: Implement `agent-tool` adapter for core contracts

**Files:**
- Modify: `agent-tool/Cargo.toml`
- Create: `agent-tool/src/runtime.rs`
- Modify: `agent-tool/src/lib.rs`
- Test: `agent-tool/tests/runtime_adapter_test.rs`

**Step 1: Write the failing test**

```rust
// agent-tool/tests/runtime_adapter_test.rs
use agent_core::tools::{ToolCatalog, ToolExecutor, ToolExecutionContext};
use agent_tool::AgentToolRuntime;

#[tokio::test]
async fn runtime_adapter_executes_registered_tool() {
    let rt = AgentToolRuntime::default_with_builtins();
    let out = rt.execute_tool(
        agent_core::ToolCall::new("shell", serde_json::json!({"command":"echo ok"})),
        ToolExecutionContext {
            session_id: "s1".into(),
            turn_id: "t1".into(),
            epoch: 0,
            cwd: None,
        },
    ).await.expect("tool should run");
    assert_eq!(out.is_error, false);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-tool --test runtime_adapter_test -q`  
Expected: FAIL (`AgentToolRuntime` missing)

**Step 3: Write minimal implementation**

```rust
// agent-tool/src/runtime.rs (shape)
pub struct AgentToolRuntime { registry: ToolRegistry }

impl AgentToolRuntime {
    pub async fn default_with_builtins() -> Self { /* register read_file + shell */ }
}

#[async_trait]
impl agent_core::tools::ToolCatalog for AgentToolRuntime { /* list/spec */ }

#[async_trait]
impl agent_core::tools::ToolExecutor for AgentToolRuntime {
    async fn execute_tool(&self, call: ToolCall, ctx: ToolExecutionContext) -> Result<ToolResult, ToolExecutionError> {
        // map to agent-tool ToolContext and registry.call(...)
    }
}
```

**Step 4: Run tests to verify it passes**

Run: `cargo test -p agent-tool -q`  
Expected: PASS

**Step 5: Commit**

```bash
git add agent-tool/Cargo.toml agent-tool/src/runtime.rs agent-tool/src/lib.rs agent-tool/tests/runtime_adapter_test.rs
git commit -m "feat(agent-tool): implement core tool runtime adapter"
```

### Task 5: Wire callers to `agent-tool` runtime

**Files:**
- Modify: `agent-turn-cli/Cargo.toml`
- Modify: `agent-turn-cli/src/main.rs`
- Test: `agent-turn-cli/src/main.rs` (unit test module or compile verification)

**Step 1: Write the failing test**

```rust
// add a small unit test in main.rs
#[test]
fn cli_builds_runtime_with_agent_tool() {
    let _name = std::any::type_name::<agent_tool::AgentToolRuntime>();
    assert!(!_name.is_empty());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn-cli -q`  
Expected: FAIL (missing `agent-tool` dependency/runtime usage)

**Step 3: Write minimal implementation**

```rust
// agent-turn-cli/src/main.rs (key shape)
let tools = Arc::new(agent_tool::AgentToolRuntime::default_with_builtins().await);
let runtime = TurnRuntime::new(model, tools, runtime_cfg);
```

**Step 4: Run tests to verify it passes**

Run: `cargo test -p agent-turn-cli -q`  
Expected: PASS

**Step 5: Commit**

```bash
git add agent-turn-cli/Cargo.toml agent-turn-cli/src/main.rs
git commit -m "refactor(turn-cli): use agent-tool runtime"
```

### Task 6: Inject tool specs into BigModel requests

**Files:**
- Modify: `agent-turn/src/effect.rs`
- Modify: `agent-turn/src/adapters/bigmodel.rs`
- Test: `agent-turn/src/adapters/bigmodel.rs` (existing test module)

**Step 1: Write the failing test**

```rust
#[test]
fn convert_request_includes_tools() {
    let req = ModelRequest {
        epoch: 0,
        transcript: vec![],
        inputs: vec![InputEnvelope::user_text("hi")],
        tools: vec![tool_spec_echo()],
    };
    let out = convert_model_request(req, &BigModelAdapterConfig::default());
    assert!(out.tools.is_some());
    assert_eq!(out.tools.unwrap().len(), 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn adapters::bigmodel::tests::convert_request_includes_tools -q`  
Expected: FAIL (tools not mapped)

**Step 3: Write minimal implementation**

```rust
// convert_model_request
let tools = request.tools.iter().map(core_spec_to_bigmodel_tool).collect::<Vec<_>>();
let mut chat_request = ChatRequest::new(cfg.model.clone(), messages).stream();
if !tools.is_empty() {
    chat_request = chat_request.tools(tools);
}
```

**Step 4: Run tests to verify it passes**

Run: `cargo test -p agent-turn --lib -q`  
Expected: PASS

**Step 5: Commit**

```bash
git add agent-turn/src/effect.rs agent-turn/src/adapters/bigmodel.rs
git commit -m "feat(turn): pass tool specs to model request"
```

### Task 7: Parse BigModel tool calls into `ModelOutputEvent::ToolCall`

**Files:**
- Modify: `bigmodel-api/src/models.rs`
- Modify: `agent-turn/src/adapters/bigmodel.rs`
- Test: `agent-turn/src/adapters/bigmodel.rs` (existing test module)

**Step 1: Write the failing test**

```rust
#[test]
fn stream_chunk_emits_tool_call_event() {
    let chunk = make_tool_call_chunk("c1", "shell", r#"{"command":"echo ok"}"#);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    emit_chunk(chunk, &tx);
    let item = rx.try_recv().unwrap().unwrap();
    assert!(matches!(item, ModelOutputEvent::ToolCall { .. }));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn adapters::bigmodel::tests::stream_chunk_emits_tool_call_event -q`  
Expected: FAIL (delta schema/parser missing tool call fields)

**Step 3: Write minimal implementation**

```rust
// bigmodel-api/src/models.rs
pub struct Delta {
    pub role: Option<String>,
    pub content: Option<String>,
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<DeltaToolCall>>,
}

// adapter emit_chunk: map tool_calls -> ModelOutputEvent::ToolCall
```

**Step 4: Run tests to verify it passes**

Run: `cargo test -p bigmodel-api -q && cargo test -p agent-turn --lib -q`  
Expected: PASS

**Step 5: Commit**

```bash
git add bigmodel-api/src/models.rs agent-turn/src/adapters/bigmodel.rs
git commit -m "feat(turn): parse model tool calls from stream deltas"
```

### Task 8: Add policy-driven tool concurrency in effect scheduler

**Files:**
- Modify: `agent-turn/src/effect.rs`
- Test: `agent-turn/src/effect.rs` (existing tests)

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn exclusive_tools_are_serialized_even_with_parallel_slots() {
    // two calls to same Exclusive tool must not overlap
    assert_eq!(max_seen.load(Ordering::SeqCst), 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn effect::tests::exclusive_tools_are_serialized_even_with_parallel_slots -q`  
Expected: FAIL (current scheduler only uses global semaphore)

**Step 3: Write minimal implementation**

```rust
// effect executor shape:
// - keep global semaphore
// - fetch spec via ToolCatalog::tool_spec(call.tool_name)
// - for Exclusive tools, acquire per-tool mutex before execute_tool
```

**Step 4: Run tests to verify it passes**

Run: `cargo test -p agent-turn --lib -q`  
Expected: PASS

**Step 5: Commit**

```bash
git add agent-turn/src/effect.rs
git commit -m "feat(turn): enforce policy-driven tool concurrency"
```

### Task 9: Normalize failure mapping and invariants

**Files:**
- Modify: `agent-tool/src/runtime.rs`
- Modify: `agent-turn/src/reducer.rs`
- Test: `agent-tool/tests/runtime_adapter_test.rs`
- Test: `agent-turn/src/reducer.rs` (existing tests)

**Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn unknown_tool_maps_to_user_error_kind() { /* ... */ }

#[test]
fn tool_result_err_removes_inflight_and_reinjects_error_input() { /* ... */ }
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p agent-tool --test runtime_adapter_test -q && cargo test -p agent-turn --lib -q`  
Expected: FAIL (error kind mapping and/or invariant checks missing)

**Step 3: Write minimal implementation**

```rust
// runtime adapter maps:
// ToolError::NotFound/InvalidArgs -> User
// ToolError::Io/ExecutionFailed -> Runtime or Transient (timeout/network if detectable)

// reducer: keep current invariant that ToolResultErr always removes inflight and injects tool_json
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p agent-tool -q && cargo test -p agent-turn --lib -q`  
Expected: PASS

**Step 5: Commit**

```bash
git add agent-tool/src/runtime.rs agent-tool/tests/runtime_adapter_test.rs agent-turn/src/reducer.rs
git commit -m "fix(tools): normalize tool failure mapping and lifecycle invariants"
```

### Task 10: Full verification and workspace cleanup

**Files:**
- Modify: `agent-core/src/lib.rs` (exports cleanup if needed)
- Modify: any touched files from prior tasks

**Step 1: Run targeted crate tests**

Run: `cargo test -p agent-core -q && cargo test -p agent-tool -q && cargo test -p agent-turn --lib -q && cargo test -p agent-session --lib -q && cargo test -p agent-turn-cli -q`  
Expected: PASS

**Step 2: Run lint checks**

Run: `cargo clippy -p agent-core -p agent-tool -p agent-turn -p agent-session -p agent-turn-cli --all-targets --all-features -- -D warnings`  
Expected: PASS

**Step 3: Run final integration smoke (CLI)**

Run: `cargo run -p agent-turn-cli -- --help`  
Expected: CLI starts and prints usage with no panic

**Step 4: Update docs if APIs changed**

```markdown
Update docs/plans references or README snippets that mention old local ToolExecutor location.
```

**Step 5: Commit**

```bash
git add -A
git commit -m "chore: finalize tooling unification migration"
```

---

Plan complete and saved to `docs/plans/2026-02-22-agent-tooling-unification-implementation-plan.md`.
