# Tool Runtime Redesign Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the stale `agent-tool` crate with a new `tool` runtime, extend `core::ToolCall` with builtin tools, and implement concrete MCP execution with per-agent policy overrides.

**Architecture:** Start by extending the `core` contract so provider output can distinguish builtin, MCP, and generic function calls. Then add builtin classification in `provider`, scaffold a new workspace `tool` crate, migrate builtin executors, add config-driven policy merging plus bounded scheduling, and finish with concrete `stdio` MCP execution and cleanup of the old `agent-tool` crate.

**Tech Stack:** Rust workspace crates, `serde`, `serde_json`, `tokio`, `async-trait`, `thiserror`, `toml`, bounded concurrency via `tokio::sync::Semaphore`, process management via `tokio::process`, `cargo test`, `cargo clippy`.

**Relevant Skills:** @test-driven-development, @rust-router, @m07-concurrency, @m06-error-handling

---

### Task 1: Extend the core tool-call contract with builtin tools

**Files:**
- Modify: `core/src/lib.rs`
- Modify: `core/tests/response_event_shapes_test.rs`
- Create: `core/tests/builtin_tool_shape_test.rs`

**Step 1: Write the failing test**

```rust
use core::{Builtin, BuiltinToolCall, ToolCall};

#[test]
fn builtin_tool_call_shape_matches_contract() {
    let call = ToolCall::Builtin(BuiltinToolCall {
        sequence: 0,
        call_id: "call_builtin_1".into(),
        builtin: Builtin::Read,
        arguments_json: r#"{"path":"Cargo.toml"}"#.into(),
    });

    match call {
        ToolCall::Builtin(inner) => assert!(matches!(inner.builtin, Builtin::Read)),
        _ => panic!("expected builtin tool call"),
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p core --test builtin_tool_shape_test -q`  
Expected: FAIL with missing `Builtin` / `BuiltinToolCall` / `ToolCall::Builtin`.

**Step 3: Write minimal implementation**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Builtin {
    Read,
    Glob,
    Grep,
    UpdatePlan,
    Shell,
    DomainCookies,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinToolCall {
    pub sequence: u32,
    pub call_id: String,
    pub builtin: Builtin,
    pub arguments_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCall {
    FunctionCall { /* existing fields */ },
    Builtin(BuiltinToolCall),
    Mcp(McpCall),
}
```

Also add helper methods in `core/src/lib.rs`:

```rust
impl Builtin {
    pub fn canonical_name(&self) -> &str { /* match arms */ }
    pub fn from_name(name: &str) -> Option<Self> { /* parse known builtin names */ }
}
```

**Step 4: Run tests to verify it passes**

Run: `cargo test -p core -q`  
Expected: PASS.

**Step 5: Commit**

```bash
git add core/src/lib.rs core/tests/response_event_shapes_test.rs core/tests/builtin_tool_shape_test.rs
git commit -m "feat(core): add builtin tool call contract"
```

### Task 2: Upgrade provider tool-call mapping to emit builtin calls explicitly

**Files:**
- Modify: `provider/src/normalize/tool_calls.rs`
- Modify: `provider/src/dialect/openai/mapper.rs`
- Modify: `provider/src/dialect/zai/mapper.rs`
- Create: `provider/tests/builtin_tool_upgrade_test.rs`

**Step 1: Write the failing test**

```rust
use argus_core::{Builtin, ResponseEvent, ToolCall};
use provider::dialect::openai::mapper::OpenAiStreamMapper;

#[test]
fn known_builtin_name_upgrades_to_builtin_call() {
    let mut mapper = OpenAiStreamMapper::default();
    let events = mapper
        .feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"m","choices":[{"index":0,"finish_reason":"tool_calls","delta":{"tool_calls":[{"id":"call_1","index":0,"type":"function","function":{"name":"read","arguments":"{\"path\":\"Cargo.toml\"}"}}]}}]}"#)
        .unwrap();

    assert!(events.iter().any(|event| matches!(
        event,
        ResponseEvent::ToolDone(ToolCall::Builtin(call))
            if matches!(call.builtin, Builtin::Read)
    )));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p provider --test builtin_tool_upgrade_test -q`  
Expected: FAIL because builtin names still map to `FunctionCall`.

**Step 3: Write minimal implementation**

In `provider/src/normalize/tool_calls.rs`, add a classifier:

```rust
pub fn classify_tool_call(name: Option<&str>, call_type: Option<&str>) -> ToolKind {
    if matches!(call_type, Some("mcp")) || name.is_some_and(|n| n.starts_with("__mcp__")) {
        return ToolKind::Mcp;
    }
    if let Some(name) = name.and_then(argus_core::Builtin::from_name) {
        return ToolKind::Builtin(name);
    }
    ToolKind::Function
}
```

Update both OpenAI and Z.AI mappers to flush builtin calls as:

```rust
ResponseEvent::ToolDone(ToolCall::Builtin(BuiltinToolCall {
    sequence,
    call_id,
    builtin,
    arguments_json,
}))
```

**Step 4: Run tests to verify it passes**

Run: `cargo test -p provider --test builtin_tool_upgrade_test -q`  
Expected: PASS.

Then run: `cargo test -p provider -q`  
Expected: PASS.

**Step 5: Commit**

```bash
git add provider/src/normalize/tool_calls.rs provider/src/dialect/openai/mapper.rs provider/src/dialect/zai/mapper.rs provider/tests/builtin_tool_upgrade_test.rs
git commit -m "feat(provider): upgrade known builtin names to builtin tool calls"
```

### Task 3: Scaffold the new workspace `tool` crate

**Files:**
- Modify: `Cargo.toml`
- Create: `tool/Cargo.toml`
- Create: `tool/src/lib.rs`
- Create: `tool/src/context.rs`
- Create: `tool/src/error.rs`
- Create: `tool/src/trait_def.rs`
- Create: `tool/tests/compile_smoke_test.rs`

**Step 1: Write the failing test**

```rust
use tool::{Tool, ToolContext, ToolError};

#[test]
fn tool_crate_exports_runtime_primitives() {
    let _ctx = ToolContext {
        session_id: "s1".into(),
        turn_id: "t1".into(),
    };
    let _ = std::mem::size_of::<ToolError>();
    fn assert_tool<T: Tool>() {}
    let _ = assert_tool::<tool::builtin::read::ReadTool>;
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p tool --test compile_smoke_test -q`  
Expected: FAIL with missing crate or items.

**Step 3: Write minimal implementation**

- Add `"tool"` to `[workspace].members` in `Cargo.toml`
- Create `tool/Cargo.toml` with dependencies on `core`, workspace `tokio`, `serde`, `serde_json`, `thiserror`, `async-trait`, and `toml`
- Create `tool/src/lib.rs` exporting empty `builtin`, `context`, `error`, and `trait_def` modules
- Port the neutral `ToolContext`, `ToolResult`, `ToolError`, and `Tool` trait shapes from `agent-tool`, but remove all `agent-core` adapter references

Minimal `tool/src/lib.rs`:

```rust
pub mod builtin;
pub mod context;
pub mod error;
pub mod trait_def;

pub use context::{ToolContext, ToolResult};
pub use error::ToolError;
pub use trait_def::Tool;
```

**Step 4: Run tests to verify it passes**

Run: `cargo test -p tool --test compile_smoke_test -q`  
Expected: PASS.

**Step 5: Commit**

```bash
git add Cargo.toml tool
git commit -m "feat(tool): scaffold new workspace tool crate"
```

### Task 4: Migrate builtin tool implementations into `tool`

**Files:**
- Create: `tool/src/builtin/mod.rs`
- Create: `tool/src/builtin/{read.rs,glob.rs,grep.rs,update_plan.rs,shell.rs,domain_cookies.rs,file.rs}`
- Create: `tool/src/builtin/fs/{mod.rs,engine.rs,error.rs,guard.rs,types.rs}`
- Modify: `tool/src/lib.rs`
- Create: `tool/tests/{read_tool_test.rs,glob_tool_test.rs,grep_tool_test.rs,update_plan_tool_test.rs,domain_cookies_tool_test.rs,fs_guard_test.rs,fs_guard_type_test.rs}`
- Reference: `agent-tool/src/builtin/**`
- Reference: `agent-tool/tests/**`

**Step 1: Write the failing test**

```rust
use tool::{builtin::read::ReadTool, Tool, ToolContext};

#[tokio::test]
async fn read_tool_reads_text_file() {
    let tool = ReadTool::default().unwrap();
    let out = tool
        .execute(
            ToolContext { session_id: "s".into(), turn_id: "t".into() },
            serde_json::json!({ "path": "Cargo.toml", "mode": "text" }),
        )
        .await
        .unwrap();
    assert!(!out.is_error);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p tool --test read_tool_test -q`  
Expected: FAIL with missing builtin modules.

**Step 3: Write minimal implementation**

- Copy builtin modules from `agent-tool/src/builtin/**` into `tool/src/builtin/**`
- Update imports from `crate::{...}` and drop `agent-core` compatibility code
- Keep existing behavior and schemas intact while re-exporting from `tool::builtin`

Minimal export module:

```rust
pub mod domain_cookies;
pub mod file;
pub mod fs;
pub mod glob;
pub mod grep;
pub mod read;
pub mod shell;
pub mod update_plan;
```

**Step 4: Run tests to verify it passes**

Run: `cargo test -p tool --test read_tool_test --test glob_tool_test --test grep_tool_test --test update_plan_tool_test --test domain_cookies_tool_test --test fs_guard_test --test fs_guard_type_test -q`  
Expected: PASS.

**Step 5: Commit**

```bash
git add tool/src tool/tests
git commit -m "feat(tool): migrate builtin tool implementations"
```

### Task 5: Add agent `.toml` config parsing and policy merge rules

**Files:**
- Create: `tool/src/config.rs`
- Modify: `tool/src/lib.rs`
- Create: `tool/tests/config_test.rs`
- Create: `tool/tests/fixtures/agent-tools.toml`

**Step 1: Write the failing test**

```rust
use tool::config::AgentToolConfig;

#[test]
fn parses_builtin_whitelist_and_overrides() {
    let raw = r#"
        [tools]
        builtin_tools = ["read", "glob"]

        [tools.defaults]
        allow_parallel = true
        max_concurrency = 4

        [tools.builtin.read]
        max_concurrency = 16
    "#;

    let cfg: AgentToolConfig = toml::from_str(raw).unwrap();
    assert_eq!(cfg.tools.builtin_tools, vec!["read", "glob"]);
}

#[test]
fn rejects_unknown_builtin_override() {
    let raw = r#"
        [tools]
        builtin_tools = ["read"]

        [tools.builtin.nope]
        max_concurrency = 2
    "#;

    assert!(AgentToolConfig::parse_and_validate(raw).is_err());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p tool --test config_test -q`  
Expected: FAIL with missing config model / validation.

**Step 3: Write minimal implementation**

```rust
#[derive(Debug, Deserialize)]
pub struct AgentToolConfig {
    pub tools: ToolConfigSection,
    #[serde(default)]
    pub mcp: McpConfigSection,
}

impl AgentToolConfig {
    pub fn parse_and_validate(raw: &str) -> Result<Self, ConfigError> {
        let cfg: Self = toml::from_str(raw)?;
        cfg.validate()?;
        Ok(cfg)
    }
}
```

Validation rules to implement:

- all names in `builtin_tools` must parse with `Builtin::from_name`
- overrides may only target enabled builtin names
- `max_concurrency` must be `>= 1`
- `allow_parallel = false` is allowed but collapses effective concurrency to `1`

**Step 4: Run tests to verify it passes**

Run: `cargo test -p tool --test config_test -q`  
Expected: PASS.

**Step 5: Commit**

```bash
git add tool/src/config.rs tool/src/lib.rs tool/tests/config_test.rs tool/tests/fixtures/agent-tools.toml
git commit -m "feat(tool): add agent tool config parsing and validation"
```

### Task 6: Implement the scheduler and per-builtin concurrency gates

**Files:**
- Create: `tool/src/catalog.rs`
- Create: `tool/src/scheduler.rs`
- Modify: `tool/src/lib.rs`
- Create: `tool/tests/scheduler_concurrency_test.rs`

**Step 1: Write the failing test**

```rust
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tool::{scheduler::ToolScheduler, ToolContext};

#[tokio::test]
async fn serial_builtin_never_runs_more_than_one_at_a_time() {
    let peak = Arc::new(AtomicUsize::new(0));
    let current = Arc::new(AtomicUsize::new(0));

    let scheduler = ToolScheduler::for_test_serial_builtin(peak.clone(), current.clone());
    scheduler.run_two_read_calls().await.unwrap();

    assert_eq!(peak.load(Ordering::SeqCst), 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p tool --test scheduler_concurrency_test -q`  
Expected: FAIL with missing scheduler / permit logic.

**Step 3: Write minimal implementation**

- Introduce `BuiltinDefinition` in `tool/src/catalog.rs`
- Build one `tokio::sync::Semaphore` per enabled builtin in `tool/src/scheduler.rs`
- Gate execution with permit acquisition before calling the builtin executor

Minimal gate pattern:

```rust
let permit = gate.acquire().await?;
let out = executor.execute(ctx, args).await;
drop(permit);
out
```

**Step 4: Run tests to verify it passes**

Run: `cargo test -p tool --test scheduler_concurrency_test -q`  
Expected: PASS.

Then run: `cargo test -p tool -q`  
Expected: PASS.

**Step 5: Commit**

```bash
git add tool/src/catalog.rs tool/src/scheduler.rs tool/src/lib.rs tool/tests/scheduler_concurrency_test.rs
git commit -m "feat(tool): add bounded builtin scheduler"
```

### Task 7: Implement concrete MCP runtime with `stdio` transport

**Files:**
- Create: `tool/src/mcp/{mod.rs,client.rs,process.rs,transport.rs}`
- Modify: `tool/src/lib.rs`
- Modify: `tool/Cargo.toml`
- Create: `tool/tests/mcp_stdio_test.rs`
- Create: `tool/tests/support/mock_mcp_server.rs`

**Step 1: Write the failing test**

```rust
use tool::mcp::McpClient;

#[tokio::test]
async fn stdio_mcp_client_lists_and_calls_tools() {
    let client = McpClient::spawn_for_test("tool/tests/support/mock_mcp_server.rs").await.unwrap();

    let tools = client.list_tools().await.unwrap();
    assert!(tools.iter().any(|tool| tool.name == "echo"));

    let out = client.call_tool("echo", r#"{"text":"hi"}"#).await.unwrap();
    assert_eq!(out, serde_json::json!({"text":"hi"}));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p tool --test mcp_stdio_test -q`  
Expected: FAIL with missing MCP runtime.

**Step 3: Write minimal implementation**

- Add `McpClient` with lazy child-process startup via `tokio::process::Command`
- Model one session per configured server label
- Implement:

```rust
pub async fn list_tools(&self) -> Result<Vec<McpToolDescriptor>, McpError>;
pub async fn call_tool(&self, name: &str, arguments_json: &str) -> Result<serde_json::Value, McpError>;
```

- Keep v1 transport-specific code isolated under `tool/src/mcp/transport.rs`
- Use a small mock server in `tool/tests/support/mock_mcp_server.rs` that reads stdin JSON lines and writes deterministic JSON replies

**Step 4: Run tests to verify it passes**

Run: `cargo test -p tool --test mcp_stdio_test -q`  
Expected: PASS.

**Step 5: Commit**

```bash
git add tool/Cargo.toml tool/src/mcp tool/src/lib.rs tool/tests/mcp_stdio_test.rs tool/tests/support/mock_mcp_server.rs
git commit -m "feat(tool/mcp): add stdio mcp client runtime"
```

### Task 8: Integrate MCP and builtin dispatch under one scheduler

**Files:**
- Modify: `tool/src/config.rs`
- Modify: `tool/src/catalog.rs`
- Modify: `tool/src/scheduler.rs`
- Modify: `tool/src/lib.rs`
- Create: `tool/tests/runtime_integration_test.rs`

**Step 1: Write the failing test**

```rust
use argus_core::{Builtin, BuiltinToolCall, McpCall, McpCallType, ToolCall};
use tool::{config::AgentToolConfig, scheduler::ToolScheduler};

#[tokio::test]
async fn scheduler_routes_builtin_and_mcp_calls_to_different_executors() {
    let cfg = AgentToolConfig::parse_and_validate(include_str!("fixtures/agent-tools.toml")).unwrap();
    let scheduler = ToolScheduler::from_config_for_test(cfg).await.unwrap();

    let builtin = ToolCall::Builtin(BuiltinToolCall {
        sequence: 0,
        call_id: "call_builtin".into(),
        builtin: Builtin::Read,
        arguments_json: r#"{"path":"Cargo.toml"}"#.into(),
    });

    let mcp = ToolCall::Mcp(McpCall {
        sequence: 1,
        id: "call_mcp".into(),
        mcp_type: McpCallType::McpCall,
        server_label: Some("filesystem".into()),
        name: Some("echo".into()),
        arguments_json: Some(r#"{"text":"hi"}"#.into()),
        output_json: None,
        tools_json: None,
        error: None,
    });

    assert!(!scheduler.execute(builtin).await.unwrap().is_error);
    assert!(!scheduler.execute(mcp).await.unwrap().is_error);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p tool --test runtime_integration_test -q`  
Expected: FAIL because MCP is not yet wired through the scheduler.

**Step 3: Write minimal implementation**

- Extend `ToolScheduler` to hold both builtin executors and MCP server handles
- Build one semaphore map for builtin targets and one for MCP server labels
- Route `ToolCall::Builtin` and `ToolCall::Mcp` through the same outer `execute` method
- Return structured `DispatchError` for disabled builtin, unknown server, or unsupported `FunctionCall`

**Step 4: Run tests to verify it passes**

Run: `cargo test -p tool --test runtime_integration_test -q`  
Expected: PASS.

Then run: `cargo test -p tool -q`  
Expected: PASS.

**Step 5: Commit**

```bash
git add tool/src/config.rs tool/src/catalog.rs tool/src/scheduler.rs tool/src/lib.rs tool/tests/runtime_integration_test.rs
git commit -m "feat(tool): unify builtin and mcp dispatch"
```

### Task 9: Remove the stale `agent-tool` crate and finish verification

**Files:**
- Delete: `agent-tool/Cargo.toml`
- Delete: `agent-tool/src/**`
- Delete: `agent-tool/tests/**`
- Modify: `Cargo.toml`

**Step 1: Write the failing test**

Use the existing workspace test matrix as the guard:

```text
core passes
provider passes
tool passes
no build step depends on agent-tool anymore
```

**Step 2: Run verification before deletion**

Run: `cargo test -p core -q`  
Expected: PASS.

Run: `cargo test -p provider -q`  
Expected: PASS.

Run: `cargo test -p tool -q`  
Expected: PASS.

**Step 3: Delete stale crate and update workspace**

- Remove `agent-tool/**`
- Keep `tool` as the only tool runtime crate
- If any docs or test fixtures still mention `agent-tool`, update them in the same change

**Step 4: Run full verification**

Run: `cargo test -p core -q`  
Expected: PASS.

Run: `cargo test -p provider -q`  
Expected: PASS.

Run: `cargo test -p tool -q`  
Expected: PASS.

Run: `cargo clippy -p core -p provider -p tool --tests -- -D warnings`  
Expected: PASS.

**Step 5: Commit**

```bash
git add Cargo.toml tool core provider
git rm -r agent-tool
git commit -m "refactor(tool): replace stale agent-tool runtime"
```
