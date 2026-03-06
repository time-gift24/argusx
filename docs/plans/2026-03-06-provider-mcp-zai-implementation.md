# Provider Unified Dialects + Z.AI MCP-First Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace split provider parsing with a unified `provider` crate (`openai` + `zai` dialects), keep OpenAI baseline compatibility, and make MCP first-class using Z.AI MCP JSON semantics.

**Architecture:** Introduce a new `provider` crate with a shared mapper contract and dialect-specific schema/parser modules. Route tool calls through a shared normalization layer; classify MCP via `type == "mcp"` or `__mcp__{name}` prefix. Emit only `core::ResponseEvent`, with strict protocol errors for invalid MCP JSON.

**Tech Stack:** Rust workspace crates, `serde`, `serde_json`, `thiserror`, ordered assembly via `BTreeMap`, SSE fixture replay tests, `cargo test`, `cargo clippy`.

---

### Task 1: Scaffold unified `provider` crate and wire workspace

**Files:**
- Modify: `Cargo.toml`
- Create: `provider/Cargo.toml`
- Create: `provider/src/lib.rs`
- Create: `provider/tests/compile_smoke_test.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn provider_crate_exposes_mapper() {
    let _ = provider::VERSION;
    let _ = provider::Mapper::new(provider::Dialect::Openai);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p provider --test compile_smoke_test -q`
Expected: FAIL with unresolved crate/items.

**Step 3: Write minimal implementation**

```rust
pub const VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Copy)]
pub enum Dialect { Openai, Zai }

pub struct Mapper;
impl Mapper {
    pub fn new(_dialect: Dialect) -> Self { Self }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p provider --test compile_smoke_test -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add Cargo.toml provider
git commit -m "feat(provider): scaffold unified provider crate"
```

### Task 2: Migrate current OpenAI stream path into `provider::dialect::openai`

**Files:**
- Create: `provider/src/dialect/openai/{mod.rs,schema.rs,parser.rs,mapper.rs}`
- Create: `provider/src/error.rs`
- Modify: `provider/src/lib.rs`
- Reference: `provider-openai/src/**`

**Step 1: Write failing parser and mapper tests**

```rust
use provider::{Dialect, Mapper};

#[test]
fn openai_chunk_maps_content_delta() {
    let mut m = Mapper::new(Dialect::Openai);
    let events = m.feed(r#"{"id":"x","object":"chat.completion.chunk","created":1,"model":"glm-5","choices":[{"index":0,"delta":{"content":"hi"}}]}"#).unwrap();
    assert!(events.iter().any(|e| matches!(e, argus_core::ResponseEvent::ContentDelta(_))));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p provider --test openai_mapper_test -q`
Expected: FAIL with missing `feed` mapping.

**Step 3: Implement minimal migration**
- Port `provider-openai` schema/parser/mapper behavior into `dialect::openai`
- Keep `feed(raw)` and `on_done()` semantics identical.

**Step 4: Run tests**

Run: `cargo test -p provider --test openai_mapper_test -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add provider/src
git commit -m "feat(provider/openai): migrate chat-completions stream mapper"
```

### Task 3: Break and update core MCP domain model to Z.AI semantics

**Files:**
- Modify: `core/src/lib.rs`
- Create: `core/tests/mcp_shape_test.rs`
- Modify: `core/tests/response_event_shapes_test.rs`

**Step 1: Write failing test for new MCP shape**

```rust
use argus_core::{ToolCall, ZaiMcpCall, ZaiMcpType};

#[test]
fn mcp_shape_is_zai_aligned() {
    let _ = ToolCall::Mcp(ZaiMcpCall {
        sequence: 0,
        id: "call_1".into(),
        mcp_type: ZaiMcpType::McpCall,
        server_label: Some("fs".into()),
        name: Some("read_file".into()),
        arguments_json: Some("{\"path\":\"./config.yaml\"}".into()),
        output_json: None,
        tools_json: None,
        error: None,
    });
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p core --test mcp_shape_test -q`
Expected: FAIL with missing `ZaiMcpCall` types.

**Step 3: Implement minimal core change**
- Add `ZaiMcpCall` and `ZaiMcpType`
- Replace old `ToolCall::Mcp { server, method, payload_json }` variant.

**Step 4: Run tests**

Run: `cargo test -p core -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add core/src core/tests
git commit -m "feat(core): replace MCP tool-call shape with zai-aligned model"
```

### Task 4: Implement shared tool normalization module

**Files:**
- Create: `provider/src/normalize/mod.rs`
- Create: `provider/src/normalize/tool_calls.rs`
- Modify: `provider/src/lib.rs`
- Create: `provider/tests/tool_normalize_test.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn classify_type_mcp_as_mcp() {}

#[test]
fn classify_prefixed_name_as_mcp() {}

#[test]
fn classify_regular_function_as_function() {}
```

**Step 2: Run tests to confirm failure**

Run: `cargo test -p provider --test tool_normalize_test -q`
Expected: FAIL with missing normalizer.

**Step 3: Implement minimal classifier**

```rust
pub fn is_mcp_call(call_type: Option<&str>, name: Option<&str>) -> bool {
    matches!(call_type, Some("mcp"))
        || name.is_some_and(|n| n.starts_with("__mcp__"))
}
```

**Step 4: Run tests to verify pass**

Run: `cargo test -p provider --test tool_normalize_test -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add provider/src/normalize provider/tests/tool_normalize_test.rs
git commit -m "feat(provider): add shared tool-call classifier for mcp/function"
```

### Task 5: OpenAI dialect MCP upgrade path (`__mcp__`)

**Files:**
- Modify: `provider/src/dialect/openai/mapper.rs`
- Modify: `provider/src/normalize/tool_calls.rs`
- Create: `provider/tests/openai_mcp_upgrade_test.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn prefixed_openai_function_becomes_mcp() {}

#[test]
fn invalid_mcp_json_returns_protocol_error() {}
```

**Step 2: Run tests to confirm failure**

Run: `cargo test -p provider --test openai_mcp_upgrade_test -q`
Expected: FAIL.

**Step 3: Implement minimal upgrade**
- During tool flush, if tool is MCP-classified, parse arguments as Z.AI MCP JSON.
- Emit `ToolCall::Mcp(...)` on success.
- Emit `Error::Protocol` on parse failure.

**Step 4: Run tests to verify pass**

Run: `cargo test -p provider --test openai_mcp_upgrade_test -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add provider/src/dialect/openai/mapper.rs provider/src/normalize/tool_calls.rs provider/tests/openai_mcp_upgrade_test.rs
git commit -m "feat(provider/openai): map __mcp__ tool calls to zai mcp model"
```

### Task 6: Add Z.AI dialect schema/parser/mapper

**Files:**
- Create: `provider/src/dialect/zai/{mod.rs,schema.rs,parser.rs,mapper.rs}`
- Modify: `provider/src/mapper.rs`
- Create: `provider/tests/zai_parser_test.rs`
- Create: `provider/tests/zai_mapper_test.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn parses_zai_chunk_with_reasoning_and_tool_delta() {}

#[test]
fn parses_zai_completion_message_tool_calls() {}
```

**Step 2: Run tests to confirm failure**

Run: `cargo test -p provider --test zai_parser_test --test zai_mapper_test -q`
Expected: FAIL.

**Step 3: Implement minimal Z.AI support**
- Parse both `chat.completion.chunk` and terminal `chat.completion` payloads.
- Normalize `delta.tool_calls` and `message.tool_calls` through shared tool normalizer.
- Keep ordering by `sequence/index`.

**Step 4: Run tests to verify pass**

Run: `cargo test -p provider --test zai_parser_test --test zai_mapper_test -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add provider/src/dialect/zai provider/tests/zai_parser_test.rs provider/tests/zai_mapper_test.rs
git commit -m "feat(provider/zai): add stream + completion tool-call parsing"
```

### Task 7: Add fixture replay for Z.AI sample stream

**Files:**
- Create: `provider/tests/fixtures/2026-03-06-zai-chat-completions-sse.txt`
- Create: `provider/tests/zai_fixture_replay_test.rs`

**Step 1: Write failing replay test**

```rust
#[test]
fn replay_zai_fixture_emits_ordered_mcp_and_done_usage() {}
```

**Step 2: Run test to verify failure**

Run: `cargo test -p provider --test zai_fixture_replay_test -q`
Expected: FAIL until mapper/parser integration is complete.

**Step 3: Implement minimal fixture assertions**
- Assert `ReasoningDelta`, `ReasoningDone`, ordered `ToolDone`, `Done(Some(Usage))`.
- Assert MCP-prefixed call was mapped to `ToolCall::Mcp`.

**Step 4: Run test to verify pass**

Run: `cargo test -p provider --test zai_fixture_replay_test -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add provider/tests/fixtures/2026-03-06-zai-chat-completions-sse.txt provider/tests/zai_fixture_replay_test.rs
git commit -m "test(provider/zai): add fixture replay coverage for mcp-first mapping"
```

### Task 8: Keep OpenAI baseline compatibility and terminal contract

**Files:**
- Create: `provider/tests/openai_compat_replay_test.rs`
- Create: `provider/tests/terminal_contract_test.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn openai_fixture_still_emits_expected_sequence() {}

#[test]
fn no_events_after_done() {}
```

**Step 2: Run tests to verify failure**

Run: `cargo test -p provider --test openai_compat_replay_test --test terminal_contract_test -q`
Expected: FAIL.

**Step 3: Implement minimal wiring fixes**
- Ensure shared terminal guard applies across both dialects.
- Ensure openai replay behavior unchanged for non-MCP function calls.

**Step 4: Run tests to verify pass**

Run: `cargo test -p provider --test openai_compat_replay_test --test terminal_contract_test -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add provider/tests/openai_compat_replay_test.rs provider/tests/terminal_contract_test.rs provider/src
git commit -m "test(provider): enforce compatibility and terminal contract across dialects"
```

### Task 9: Migrate references from `provider-openai` to `provider`

**Files:**
- Search/Modify: all `Cargo.toml` referencing `provider-openai`
- Search/Modify: imports in `*.rs` files
- Optional shim: `provider-openai/src/lib.rs`

**Step 1: Write failing build gate**

Run: `cargo check --workspace`
Expected: FAIL with unresolved crate paths after core/provider changes.

**Step 2: Implement minimal migration**
- Replace direct imports to new `provider` crate.
- Optionally keep `provider-openai` thin wrapper re-exporting `provider::Dialect::Openai` path for phased migration.

**Step 3: Run build gate**

Run: `cargo check --workspace`
Expected: PASS.

**Step 4: Run targeted tests**

Run: `cargo test -p core -p provider`
Expected: PASS.

**Step 5: Commit**

```bash
git add .
git commit -m "refactor: switch callers to unified provider crate"
```

### Task 10: Final verification and docs update

**Files:**
- Modify: `provider/README.md`
- Modify: `docs/plans/2026-03-06-provider-mcp-zai-design.md` (Status/links only)

**Step 1: Verification commands**

Run:
- `cargo test -p core`
- `cargo test -p provider`
- `cargo clippy -p provider -- -D warnings`
- `cargo check --workspace`

Expected: all PASS.

**Step 2: Document usage examples**
- Dialect selection example (`Openai` vs `Zai`)
- MCP prefix contract (`__mcp__{name}`)
- Protocol error semantics on invalid MCP JSON

**Step 3: Commit**

```bash
git add provider/README.md docs/plans/2026-03-06-provider-mcp-zai-design.md
git commit -m "docs(provider): add unified dialect and mcp-first usage notes"
```

## Execution Notes

- Keep each commit scoped to one task.
- Do not collapse Tasks 3-7 into one commit; review MCP behavior at each boundary.
- Prefer fixture replay to validate stream order and terminal behavior before workspace-wide migrations.
