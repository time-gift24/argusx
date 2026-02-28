# Custom Subprocess Agent Tools Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a high-performance, non-MCP, subprocess-based user tool runtime with explicit config registration and mixed warmup/lazy-start lifecycle.

**Architecture:** Keep `agent_core::tools::{ToolCatalog, ToolExecutor}` unchanged and implement a new subprocess backend inside `agent-tool`. The backend uses length-prefixed MessagePack over stdio, one long-lived process per tool name, per-tool queue limits, timeout/restart/circuit-breaker, and optional warmup. Existing built-ins remain available and can be combined with user-defined subprocess tools.

**Tech Stack:** Rust, tokio, serde, rmp-serde, bytes, toml, async-trait, tracing

---

## Skills To Apply During Execution

- `@test-driven-development`
- `@systematic-debugging`
- `@verification-before-completion`

## Phase 1: Protocol + Codec Foundation

### Task 1: Add dependencies and module skeleton

**Files:**
- Modify: `Cargo.toml`
- Modify: `agent-tool/Cargo.toml`
- Create: `agent-tool/src/subprocess/mod.rs`
- Modify: `agent-tool/src/lib.rs`

**Step 1: Write the failing compile check (imports that do not exist yet)**

```rust
// agent-tool/src/lib.rs (temporary during TDD)
pub mod subprocess;
```

**Step 2: Run check to verify fail**

Run: `cargo check -p agent-tool`
Expected: FAIL with module not found for `subprocess`

**Step 3: Add minimal module + deps**

```toml
# agent-tool/Cargo.toml
[dependencies]
bytes = "1"
rmp-serde = "1"
toml = "0.8"
```

```rust
// agent-tool/src/subprocess/mod.rs
pub mod protocol;
pub mod codec;
pub mod config;
pub mod runtime;
```

```rust
// agent-tool/src/lib.rs
pub mod subprocess;
```

**Step 4: Run check to verify pass**

Run: `cargo check -p agent-tool`
Expected: PASS

**Step 5: Commit**

```bash
git add Cargo.toml agent-tool/Cargo.toml agent-tool/src/lib.rs agent-tool/src/subprocess/mod.rs
git commit -m "feat(agent-tool): add subprocess module skeleton"
```

---

### Task 2: Implement protocol model + length-prefixed MessagePack codec

**Files:**
- Create: `agent-tool/src/subprocess/protocol.rs`
- Create: `agent-tool/src/subprocess/codec.rs`
- Create: `agent-tool/tests/subprocess_codec_test.rs`

**Step 1: Write failing tests for frame encode/decode**

```rust
// agent-tool/tests/subprocess_codec_test.rs
#[tokio::test]
async fn roundtrip_msgpack_frame() {
    let msg = protocol::CallReq {
        call_id: "c1".into(),
        tool_name: "my_read".into(),
        arguments: serde_json::json!({"path":"/tmp/a"}),
        context: protocol::CallContext { session_id: "s1".into(), turn_id: "t1".into(), epoch: 0 },
    };

    let mut buf = Vec::new();
    codec::write_frame(&mut buf, &msg).await.unwrap();
    let decoded: protocol::CallReq = codec::read_frame(buf.as_slice()).await.unwrap();

    assert_eq!(decoded.call_id, "c1");
    assert_eq!(decoded.tool_name, "my_read");
}
```

**Step 2: Run test to verify fail**

Run: `cargo test -p agent-tool subprocess_codec_test::roundtrip_msgpack_frame -- --nocapture`
Expected: FAIL with unresolved modules/types

**Step 3: Write minimal protocol + codec implementation**

```rust
// protocol.rs
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CallReq {
    pub call_id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub context: CallContext,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CallContext {
    pub session_id: String,
    pub turn_id: String,
    pub epoch: u64,
}
```

```rust
// codec.rs
pub async fn write_frame<W, T>(w: &mut W, msg: &T) -> std::io::Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
    T: serde::Serialize,
{ /* length-prefix + msgpack */ }

pub async fn read_frame<R, T>(r: R) -> std::io::Result<T>
where
    R: tokio::io::AsyncRead + Unpin,
    T: serde::de::DeserializeOwned,
{ /* inverse decode */ }
```

**Step 4: Run targeted tests**

Run: `cargo test -p agent-tool subprocess_codec_test -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-tool/src/subprocess/protocol.rs agent-tool/src/subprocess/codec.rs agent-tool/tests/subprocess_codec_test.rs
git commit -m "feat(agent-tool): add msgpack length-prefixed codec"
```

---

## Phase 2: Config-Driven Tool Registration

### Task 3: Add `tools.toml` parser and validation

**Files:**
- Create: `agent-tool/src/subprocess/config.rs`
- Create: `agent-tool/tests/subprocess_config_test.rs`

**Step 1: Write failing validation tests**

```rust
#[test]
fn duplicate_tool_names_should_fail() {
    let s = r#"
[[tools]]
name = "x"
command = "/bin/echo"

[[tools]]
name = "x"
command = "/bin/cat"
"#;

    let err = config::parse_and_validate(s).unwrap_err();
    assert!(err.to_string().contains("duplicate"));
}
```

**Step 2: Run tests to verify fail**

Run: `cargo test -p agent-tool subprocess_config_test -- --nocapture`
Expected: FAIL

**Step 3: Implement parser + validator**

```rust
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ToolFileConfig {
    pub tools: Vec<ToolDefinition>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub command: String,
    #[serde(default)] pub args: Vec<String>,
    #[serde(default)] pub warmup: bool,
    #[serde(default = "default_max_queue")] pub max_queue: usize,
    #[serde(default = "default_call_timeout_ms")] pub call_timeout_ms: u64,
}

pub fn parse_and_validate(input: &str) -> anyhow::Result<ToolFileConfig> { /* ... */ }
```

**Step 4: Run tests**

Run: `cargo test -p agent-tool subprocess_config_test -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-tool/src/subprocess/config.rs agent-tool/tests/subprocess_config_test.rs
git commit -m "feat(agent-tool): add subprocess tool config parser"
```

---

## Phase 3: Child Process Host Runtime

### Task 4: Implement child process client with request routing (`call_id -> oneshot`)

**Files:**
- Create: `agent-tool/src/subprocess/host.rs`
- Create: `agent-tool/tests/subprocess_host_test.rs`
- Create: `agent-tool/tests/fixtures/mock_tool_server.rs`

**Step 1: Write failing host integration test (single call)**

```rust
#[tokio::test]
async fn host_can_call_mock_server() {
    let mut host = TestHost::spawn_fixture("mock_tool_server").await.unwrap();
    let out = host.call("c1", "my_read", serde_json::json!({"path":"/tmp/x"})).await.unwrap();
    assert!(!out.is_error);
    assert_eq!(out.output["ok"], true);
}
```

**Step 2: Run test to verify fail**

Run: `cargo test -p agent-tool subprocess_host_test::host_can_call_mock_server -- --nocapture`
Expected: FAIL

**Step 3: Implement `ToolHostHandle` minimal path**

```rust
pub struct ToolHostHandle {
    child: tokio::process::Child,
    pending: std::collections::HashMap<String, tokio::sync::oneshot::Sender<CallResp>>,
    tx: tokio::sync::mpsc::Sender<CallReq>,
}
```

Include:
- start child with piped stdin/stdout
- spawn writer task (encode frames)
- spawn reader task (decode frames and resolve `pending`)

**Step 4: Run test**

Run: `cargo test -p agent-tool subprocess_host_test::host_can_call_mock_server -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-tool/src/subprocess/host.rs agent-tool/tests/subprocess_host_test.rs agent-tool/tests/fixtures/mock_tool_server.rs
git commit -m "feat(agent-tool): add subprocess host with call routing"
```

---

### Task 5: Add timeout and queue limit behavior

**Files:**
- Modify: `agent-tool/src/subprocess/host.rs`
- Modify: `agent-tool/tests/subprocess_host_test.rs`

**Step 1: Add failing tests for timeout and queue full**

```rust
#[tokio::test]
async fn call_timeout_returns_transient_error() {
    // fixture sleeps longer than timeout
}

#[tokio::test]
async fn queue_full_fails_fast_with_transient_error() {
    // max_queue=1, send 3 concurrent calls
}
```

**Step 2: Run tests to verify fail**

Run: `cargo test -p agent-tool subprocess_host_test -- --nocapture`
Expected: FAIL on new cases

**Step 3: Implement minimal behavior**

- enforce bounded queue with `tokio::sync::mpsc::channel(max_queue)`
- wrap await response with `tokio::time::timeout`
- map timeout/queue-full to transient error type

**Step 4: Run tests**

Run: `cargo test -p agent-tool subprocess_host_test -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-tool/src/subprocess/host.rs agent-tool/tests/subprocess_host_test.rs
git commit -m "feat(agent-tool): add timeout and queue backpressure"
```

---

## Phase 4: Runtime Integration

### Task 6: Implement `SubprocessToolRuntime` (`ToolCatalog + ToolExecutor`)

**Files:**
- Create: `agent-tool/src/subprocess/runtime.rs`
- Modify: `agent-tool/src/subprocess/mod.rs`
- Create: `agent-tool/tests/subprocess_runtime_adapter_test.rs`

**Step 1: Write failing adapter tests**

```rust
#[tokio::test]
async fn runtime_lists_tools_from_config() {
    let rt = test_runtime_with_two_tools().await;
    let tools = rt.list_tools().await;
    assert_eq!(tools.len(), 2);
}

#[tokio::test]
async fn runtime_maps_user_error_kind() {
    let err = rt.execute_tool(bad_call(), ctx()).await.unwrap_err();
    assert!(matches!(err.kind, agent_core::tools::ToolExecutionErrorKind::User));
}
```

**Step 2: Run tests to verify fail**

Run: `cargo test -p agent-tool subprocess_runtime_adapter_test -- --nocapture`
Expected: FAIL

**Step 3: Implement runtime mapping**

- `list_tools/tool_spec` from loaded definitions
- `execute_tool` delegates to host manager
- map custom error kinds into `ToolExecutionErrorKind`

**Step 4: Run tests**

Run: `cargo test -p agent-tool subprocess_runtime_adapter_test -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-tool/src/subprocess/runtime.rs agent-tool/src/subprocess/mod.rs agent-tool/tests/subprocess_runtime_adapter_test.rs
git commit -m "feat(agent-tool): implement subprocess runtime adapter"
```

---

### Task 7: Compose built-ins + subprocess tools in `AgentToolRuntime`

**Files:**
- Modify: `agent-tool/src/runtime.rs`
- Modify: `agent-tool/src/lib.rs`
- Modify: `agent/src/builder.rs`
- Create: `agent-tool/tests/runtime_composed_test.rs`

**Step 1: Write failing composed-runtime test**

```rust
#[tokio::test]
async fn composed_runtime_exposes_builtin_and_subprocess_tools() {
    let rt = build_composed_runtime().await;
    let names = rt.list_tools().await.into_iter().map(|t| t.name).collect::<Vec<_>>();
    assert!(names.contains(&"shell".to_string()));
    assert!(names.contains(&"my_read".to_string()));
}
```

**Step 2: Run test to verify fail**

Run: `cargo test -p agent-tool runtime_composed_test -- --nocapture`
Expected: FAIL

**Step 3: Implement composition**

- extend `AgentToolRuntime` to hold multiple backends
- resolve tool name by catalog lookup
- route execution to owner backend

**Step 4: Run tests**

Run: `cargo test -p agent-tool runtime_composed_test -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-tool/src/runtime.rs agent-tool/src/lib.rs agent/src/builder.rs agent-tool/tests/runtime_composed_test.rs
git commit -m "feat(agent-tool): compose builtin and subprocess tool backends"
```

---

## Phase 5: Reliability Controls

### Task 8: Add restart backoff + circuit breaker

**Files:**
- Create: `agent-tool/src/subprocess/supervisor.rs`
- Modify: `agent-tool/src/subprocess/host.rs`
- Create: `agent-tool/tests/subprocess_supervisor_test.rs`

**Step 1: Write failing tests for crash recovery and open/half-open/closed transitions**

```rust
#[tokio::test]
async fn crash_triggers_backoff_restart() { /* ... */ }

#[tokio::test]
async fn repeated_failures_open_circuit() { /* ... */ }
```

**Step 2: Run tests to verify fail**

Run: `cargo test -p agent-tool subprocess_supervisor_test -- --nocapture`
Expected: FAIL

**Step 3: Implement minimal supervisor state machine**

```rust
pub enum CircuitState { Closed, Open { until: std::time::Instant }, HalfOpen }
```

- consecutive failure count
- exponential restart delay
- half-open probe call

**Step 4: Run tests**

Run: `cargo test -p agent-tool subprocess_supervisor_test -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-tool/src/subprocess/supervisor.rs agent-tool/src/subprocess/host.rs agent-tool/tests/subprocess_supervisor_test.rs
git commit -m "feat(agent-tool): add restart and circuit breaker"
```

---

### Task 9: Add mixed warmup/lazy-start orchestration

**Files:**
- Modify: `agent-tool/src/subprocess/runtime.rs`
- Modify: `agent-tool/src/subprocess/config.rs`
- Create: `agent-tool/tests/subprocess_warmup_test.rs`

**Step 1: Write failing tests for warmup behavior**

```rust
#[tokio::test]
async fn warmup_true_tools_start_on_boot() { /* ... */ }

#[tokio::test]
async fn warmup_false_tools_start_on_first_call() { /* ... */ }
```

**Step 2: Run tests to verify fail**

Run: `cargo test -p agent-tool subprocess_warmup_test -- --nocapture`
Expected: FAIL

**Step 3: Implement warmup manager**

- add `warmup_all(max_parallel_warmups)`
- non-blocking startup for failures (mark unhealthy)
- lazy start path unchanged

**Step 4: Run tests**

Run: `cargo test -p agent-tool subprocess_warmup_test -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-tool/src/subprocess/runtime.rs agent-tool/src/subprocess/config.rs agent-tool/tests/subprocess_warmup_test.rs
git commit -m "feat(agent-tool): add mixed warmup and lazy-start"
```

---

## Phase 6: SDK for User Tool Authors

### Task 10: Create `agent-tool-sdk` crate for minimal handler implementation

**Files:**
- Create: `agent-tool-sdk/Cargo.toml`
- Create: `agent-tool-sdk/src/lib.rs`
- Create: `agent-tool-sdk/src/server.rs`
- Modify: `Cargo.toml`
- Create: `agent-tool-sdk/tests/sdk_loop_test.rs`

**Step 1: Write failing test for handler loop contract**

```rust
#[tokio::test]
async fn sdk_server_reads_call_and_returns_response() {
    // feed one frame in-memory, assert one response frame out
}
```

**Step 2: Run test to verify fail**

Run: `cargo test -p agent-tool-sdk sdk_loop_test -- --nocapture`
Expected: FAIL (crate/files missing)

**Step 3: Implement minimal SDK loop**

```rust
#[async_trait::async_trait]
pub trait ToolHandler {
    async fn handle(&self, args: serde_json::Value, ctx: CallContext) -> Result<serde_json::Value, ToolError>;
}

pub async fn run_stdio_server<H: ToolHandler>(handler: H) -> anyhow::Result<()> {
    // read frame -> call handler -> write response frame
}
```

**Step 4: Run tests**

Run: `cargo test -p agent-tool-sdk -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add Cargo.toml agent-tool-sdk/Cargo.toml agent-tool-sdk/src/lib.rs agent-tool-sdk/src/server.rs agent-tool-sdk/tests/sdk_loop_test.rs
git commit -m "feat(agent-tool-sdk): add minimal stdio tool server sdk"
```

---

## Phase 7: Verification + Docs

### Task 11: End-to-end verification and operator docs

**Files:**
- Create: `agent-tool/README-subprocess-tools.md`
- Modify: `agent-cli/README.md`
- Create: `docs/plans/2026-02-28-custom-subprocess-agent-tools-verification.md`

**Step 1: Add failing smoke script (documented command not yet working)**

```bash
cargo run -p agent-cli -- --tool-config ./config/tools.toml
```

Expected initially: FAIL or unsupported flag behavior

**Step 2: Implement CLI wiring if missing**

- add config path option in CLI entry
- pass config into `AgentBuilder`/runtime construction

**Step 3: Run full verification suite**

Run:
- `cargo test -p agent-tool -- --nocapture`
- `cargo test -p agent-turn -- --nocapture`
- `cargo test -p agent-session -- --nocapture`
- `cargo check --workspace`

Expected: all PASS

**Step 4: Write verification report**

Create `docs/plans/2026-02-28-custom-subprocess-agent-tools-verification.md` with:
- test matrix
- command outputs summary
- known limitations

**Step 5: Commit**

```bash
git add agent-tool/README-subprocess-tools.md agent-cli/README.md docs/plans/2026-02-28-custom-subprocess-agent-tools-verification.md
git commit -m "docs: add subprocess tools usage and verification report"
```

---

## Final Validation Gate (Required Before Merge)

1. Run: `cargo fmt --all -- --check`
2. Run: `cargo clippy --workspace --all-targets -- -D warnings`
3. Run: `cargo check --workspace`
4. Run: `cargo test -p agent-tool -- --nocapture`
5. Run: `cargo test -p agent-turn -- --nocapture`
6. Run: `cargo test -p agent-session -- --nocapture`

If any command fails, do not claim completion; fix and re-run.

## Notes

- Keep commits small and task-aligned.
- Do not include MCP terminology in protocol/API naming.
- Prefer fast-fail over hidden queue growth.
- Default behavior should be safe: bounded queue + timeout enabled.
