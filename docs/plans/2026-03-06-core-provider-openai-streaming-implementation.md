# Core + Provider-OpenAI Streaming Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a new parallel streaming foundation with `core` event protocol and `provider-openai` SSE adapter (without changing the current runtime path).

**Architecture:** Introduce two new crates. `core` owns protocol types and stream-state invariants. `provider-openai` maps Chat Completions SSE chunks into `core::ResponseEvent` and replays the approved fixture as integration proof. Keep old `llm-client/llm-provider/agent-turn` untouched in this phase.

**Tech Stack:** Rust workspace crates, `serde`, `serde_json`, `thiserror`, `futures`, vendored SSE parser, fixture-based integration tests, `cargo test`.

---

### Task 1: Add New Workspace Crates (parallel path only)

**Files:**
- Modify: `Cargo.toml`
- Create: `core/Cargo.toml`
- Create: `core/src/lib.rs`
- Create: `provider-openai/Cargo.toml`
- Create: `provider-openai/src/lib.rs`
- Test: `provider-openai/tests/compile_smoke_test.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn crates_compile() {
    let _ = core::ResponseEvent::Done(None);
    let _ = provider_openai::VERSION;
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p provider-openai --test compile_smoke_test -q`
Expected: FAIL with unresolved crate/items.

**Step 3: Write minimal implementation**

```rust
// core/src/lib.rs
pub enum ResponseEvent { Done(Option<Usage>), Error(Error) }
pub struct Usage { pub input_tokens: u64, pub output_tokens: u64, pub total_tokens: u64 }
pub struct Error { pub message: String }

// provider-openai/src/lib.rs
pub const VERSION: &str = "0.1.0";
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p provider-openai --test compile_smoke_test -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add Cargo.toml core provider-openai
git commit -m "feat(core,provider-openai): scaffold new parallel crates"
```

### Task 2: Implement `core` Event Model And Domain Types

**Files:**
- Modify: `core/src/lib.rs`
- Create: `core/src/types.rs`
- Test: `core/tests/response_event_shapes_test.rs`

**Step 1: Write the failing test**

```rust
use std::sync::Arc;
use core::{Meta, ResponseEvent, ToolCall, Usage};

#[test]
fn response_event_shape_matches_design() {
    let _ = ResponseEvent::Created(Meta { model: "glm-5".into(), provider: "openai".into() });
    let _ = ResponseEvent::ContentDelta(Arc::<str>::from("hi"));
    let _ = ResponseEvent::ReasoningDelta(Arc::<str>::from("think"));
    let _ = ResponseEvent::ToolDelta(Arc::<str>::from("{\"city\""));
    let _ = ResponseEvent::ToolDone(ToolCall::FunctionCall {
        call_id: "call_1".into(),
        name: "get_weather".into(),
        arguments_json: "{\"city\":\"北京\"}".into(),
    });
    let _ = ResponseEvent::Done(Some(Usage { input_tokens: 1, output_tokens: 2, total_tokens: 3 }));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p core --test response_event_shapes_test -q`
Expected: FAIL with missing `Meta/ToolCall/ResponseEvent` variants.

**Step 3: Write minimal implementation**

```rust
pub struct Meta { pub model: String, pub provider: String }

pub enum ToolCall {
    FunctionCall { call_id: String, name: String, arguments_json: String },
    Mcp { server: String, method: String, payload_json: String, call_id: Option<String> },
}

pub enum ResponseEvent {
    Created(Meta),
    ContentDelta(std::sync::Arc<str>),
    ReasoningDelta(std::sync::Arc<str>),
    ToolDelta(std::sync::Arc<str>),
    ContentDone(String),
    ReasoningDone(String),
    ToolDone(ToolCall),
    Done(Option<Usage>),
    Error(Error),
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p core --test response_event_shapes_test -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add core/src core/tests
git commit -m "feat(core): add response event and tool call domain types"
```

### Task 3: Implement `core` Stream Contract State Machine

**Files:**
- Create: `core/src/contract.rs`
- Modify: `core/src/lib.rs`
- Test: `core/tests/contract_state_machine_test.rs`

**Step 1: Write the failing test**

```rust
use core::{ResponseContract, ResponseEvent, Usage};

#[test]
fn terminal_event_is_exclusive() {
    let mut c = ResponseContract::new();
    assert!(c.accept(ResponseEvent::Done(Some(Usage::zero()))).is_ok());
    assert!(c.accept(ResponseEvent::Error("late".into())).is_err());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p core --test contract_state_machine_test -q`
Expected: FAIL with missing `ResponseContract`.

**Step 3: Write minimal implementation**

```rust
pub struct ResponseContract { terminated: bool }

impl ResponseContract {
    pub fn new() -> Self { Self { terminated: false } }
    pub fn accept(&mut self, event: ResponseEvent) -> Result<(), ContractError> {
        if self.terminated { return Err(ContractError::AfterTerminal); }
        if matches!(event, ResponseEvent::Done(_) | ResponseEvent::Error(_)) {
            self.terminated = true;
        }
        Ok(())
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p core --test contract_state_machine_test -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add core/src core/tests
git commit -m "feat(core): enforce terminal-event contract with state machine"
```

### Task 4: Build OpenAI Chunk Models And Parser Entry

**Files:**
- Create: `provider-openai/src/chunk.rs`
- Create: `provider-openai/src/parser.rs`
- Modify: `provider-openai/src/lib.rs`
- Test: `provider-openai/tests/chunk_parse_test.rs`

**Step 1: Write the failing test**

```rust
use provider_openai::parse_chunk;

#[test]
fn parse_reasoning_chunk() {
    let raw = r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"delta":{"reasoning_content":"用户"}}]}"#;
    let chunk = parse_chunk(raw).unwrap();
    assert_eq!(chunk.model, "glm-5");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p provider-openai --test chunk_parse_test -q`
Expected: FAIL with missing parser.

**Step 3: Write minimal implementation**

```rust
pub fn parse_chunk(raw: &str) -> Result<ChatCompletionsChunk, Error> {
    serde_json::from_str(raw).map_err(Error::Parse)
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p provider-openai --test chunk_parse_test -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add provider-openai/src provider-openai/tests
git commit -m "feat(provider-openai): add chat completions chunk parser"
```

### Task 5: Implement Event Mapper + ToolCall Assembly

**Files:**
- Create: `provider-openai/src/mapper.rs`
- Modify: `provider-openai/src/lib.rs`
- Test: `provider-openai/tests/event_mapper_test.rs`

**Step 1: Write the failing test**

```rust
use provider_openai::Mapper;

#[test]
fn assembles_tool_call_on_finish_reason_tool_calls() {
    let mut m = Mapper::new("openai".into());
    m.feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"delta":{"tool_calls":[{"id":"call_1","index":0,"type":"function","function":{"name":"get_weather","arguments":"{\""}}]}}]}"#).unwrap();
    m.feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"type":"function","function":{"arguments":"city\":\"北京\"}"}}]}}]}"#).unwrap();
    let events = m.feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"finish_reason":"tool_calls","delta":{"content":""}}]}"#).unwrap();
    assert!(events.iter().any(|e| matches!(e, core::ResponseEvent::ToolDone(_))));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p provider-openai --test event_mapper_test -q`
Expected: FAIL with missing mapper/assembly.

**Step 3: Write minimal implementation**

```rust
pub struct Mapper { /* usage cache + reasoning/content buffer + tool buffers */ }

impl Mapper {
    pub fn feed(&mut self, raw: &str) -> Result<Vec<core::ResponseEvent>, Error> {
        // parse chunk
        // emit Created once
        // emit Delta events
        // on finish_reason == "tool_calls" emit ToolDone(ToolCall::FunctionCall{...})
        // cache usage if exists
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p provider-openai --test event_mapper_test -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add provider-openai/src provider-openai/tests
git commit -m "feat(provider-openai): map chunks to core events and assemble tool calls"
```

### Task 6: Add Fixture Replay Integration Test (user-provided stream)

**Files:**
- Create: `provider-openai/tests/fixtures/openai-chat-completions-sse.txt`
- Create: `provider-openai/tests/fixture_replay_test.rs`
- Copy from: `docs/plans/testdata/2026-03-06-openai-chat-completions-sse.txt`

**Step 1: Write the failing test**

```rust
#[test]
fn replay_fixture_emits_done_with_usage() {
    let lines = std::fs::read_to_string("provider-openai/tests/fixtures/openai-chat-completions-sse.txt").unwrap();
    let mut mapper = provider_openai::Mapper::new("openai".into());

    let mut all = Vec::new();
    for line in lines.lines().filter(|l| l.starts_with("data: ")) {
        let payload = &line[6..];
        if payload == "[DONE]" {
            all.extend(mapper.on_done().unwrap());
            continue;
        }
        all.extend(mapper.feed(payload).unwrap());
    }

    assert!(all.iter().any(|e| matches!(e, core::ResponseEvent::ReasoningDelta(_))));
    assert!(all.iter().any(|e| matches!(e, core::ResponseEvent::ContentDelta(_))));
    assert!(all.iter().any(|e| matches!(e, core::ResponseEvent::ToolDone(_))));
    assert!(all.iter().any(|e| matches!(e, core::ResponseEvent::Done(Some(_)))));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p provider-openai --test fixture_replay_test -q`
Expected: FAIL until fixture replay and terminal handling exist.

**Step 3: Write minimal implementation**

```rust
impl Mapper {
    pub fn on_done(&mut self) -> Result<Vec<core::ResponseEvent>, Error> {
        Ok(vec![core::ResponseEvent::Done(self.usage.take())])
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p provider-openai --test fixture_replay_test -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add provider-openai/tests provider-openai/src
git commit -m "test(provider-openai): add fixture replay coverage for reasoning/content/tool/usage"
```

### Task 7: Enforce Error/Terminal Contract In Adapter

**Files:**
- Modify: `provider-openai/src/mapper.rs`
- Test: `provider-openai/tests/terminal_contract_test.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn no_events_after_done() {
    let mut m = provider_openai::Mapper::new("openai".into());
    let _ = m.on_done().unwrap();
    let err = m.feed("{\"id\":\"x\",\"created\":1,\"object\":\"chat.completion.chunk\",\"model\":\"glm-5\",\"choices\":[]}").unwrap_err();
    assert!(format!("{err}").contains("terminal"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p provider-openai --test terminal_contract_test -q`
Expected: FAIL before terminal guard is wired.

**Step 3: Write minimal implementation**

```rust
if self.terminated {
    return Err(Error::Protocol("event after terminal".into()));
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p provider-openai --test terminal_contract_test -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add provider-openai/src provider-openai/tests
git commit -m "fix(provider-openai): enforce terminal exclusivity contract"
```

### Task 8: Verification Pass And Developer Notes

**Files:**
- Modify: `provider-openai/README.md`
- Modify: `docs/plans/2026-03-06-core-provider-openai-streaming-design.md`

**Step 1: Write the failing doc check (manual)**

Document expected commands and outputs in README before running full verification.

**Step 2: Run verification commands**

Run: `cargo test -p core -p provider-openai`
Expected: PASS all tests.

Run: `cargo check --workspace`
Expected: PASS (existing crates unchanged by behavior).

**Step 3: Write minimal doc implementation**

Add:
- fixture replay instructions
- event contract summary
- known out-of-scope list

**Step 4: Run verification again**

Run: `cargo test -p core -p provider-openai && cargo check --workspace`
Expected: PASS.

**Step 5: Commit**

```bash
git add provider-openai/README.md docs/plans/2026-03-06-core-provider-openai-streaming-design.md
git commit -m "docs: add core/provider-openai usage and verification notes"
```

## Skills To Apply During Execution

- `@test-driven-development`
- `@verification-before-completion`
- `@rust-skills`
- `@rust-best-practices`

## Guardrails

- Do not modify old runtime call path in this phase.
- Keep implementation minimal: only contract correctness and fixture-backed behavior.
- Prefer small commits per task.
- No optimization work until correctness and tests are green.
