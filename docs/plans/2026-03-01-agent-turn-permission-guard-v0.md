# Agent Turn Permission Guard V0 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a minimal permission gate that blocks `sandbox_permissions=require_escalated` tool calls unless the turn `approval_policy` is `on_request`.

**Architecture:** Introduce shared permission enums in `agent-core`, carry `approval_policy` on `TurnRequest`, thread that policy through `TurnRuntime -> TurnState -> Effect::ExecuteTool`, and enforce the guard in `EffectExecutor` before tool execution. Keep sandboxing out of scope in this iteration; this is strictly a permission escalation guard with deterministic error surfacing.

**Tech Stack:** Rust 2021, Tokio async runtime, serde/serde_json, workspace crates `agent-core`, `agent-turn`, `agent-tool`, `agent-session`, `agent`.

---

Implementation guidance: apply @test-driven-development, @verification-before-completion, @rust-best-practices.

### Task 1: Add Shared Permission Types And TurnRequest Policy Field

**Files:**
- Create: `agent-core/tests/turn_request_permissions_test.rs`
- Modify: `agent-core/src/model.rs`
- Modify: `agent-core/src/lib.rs`

**Step 1: Write the failing test**

```rust
use agent_core::{AskForApproval, InputEnvelope, SandboxPermissions, SessionMeta, TurnRequest};

#[test]
fn turn_request_defaults_to_on_request() {
    let req = TurnRequest::new(SessionMeta::new("s1", "t1"), InputEnvelope::user_text("hi"));
    assert_eq!(req.approval_policy, AskForApproval::OnRequest);
}

#[test]
fn sandbox_permissions_serializes_in_snake_case() {
    let raw = serde_json::to_string(&SandboxPermissions::RequireEscalated).unwrap();
    assert_eq!(raw, "\"require_escalated\"");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-core --test turn_request_permissions_test -v`
Expected: FAIL with missing types/fields like `AskForApproval`, `SandboxPermissions`, or `approval_policy`.

**Step 3: Write minimal implementation**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AskForApproval {
    Never,
    OnFailure,
    OnRequest,
    UnlessTrusted,
}

impl Default for AskForApproval {
    fn default() -> Self {
        Self::OnRequest
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxPermissions {
    UseDefault,
    RequireEscalated,
}

impl Default for SandboxPermissions {
    fn default() -> Self {
        Self::UseDefault
    }
}

impl SandboxPermissions {
    pub fn requires_escalated_permissions(self) -> bool {
        matches!(self, Self::RequireEscalated)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnRequest {
    pub meta: SessionMeta,
    pub initial_input: InputEnvelope,
    #[serde(default)]
    pub transcript: Vec<TranscriptItem>,
    #[serde(default)]
    pub approval_policy: AskForApproval,
}
```

Also export the new types in `agent-core/src/lib.rs`.

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-core --test turn_request_permissions_test -v`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-core/src/model.rs agent-core/src/lib.rs agent-core/tests/turn_request_permissions_test.rs
git commit -m "feat(core): add turn approval policy and sandbox permission types"
```

### Task 2: Thread Approval Policy Through Turn Runtime And Effects

**Files:**
- Modify: `agent-turn/src/state.rs`
- Modify: `agent-turn/src/runtime_impl.rs`
- Modify: `agent-turn/src/effect.rs`
- Modify: `agent-turn/src/reducer.rs`
- Modify: `agent-turn/src/test_helpers.rs`
- Modify: `agent/src/agent.rs`
- Modify: `agent-session/src/session_runtime.rs`

**Step 1: Write the failing test**

Add to `agent-turn/src/reducer.rs` tests:

```rust
#[test]
fn model_tool_call_effect_carries_turn_approval_policy() {
    let state = StateBuilder::new("s1", "t1")
        .with_lifecycle(Lifecycle::Active)
        .with_model_state(ModelState::Streaming)
        .with_approval_policy(agent_core::AskForApproval::Never)
        .build();

    let tr = reduce(
        state,
        EventBuilder::model_tool_call(
            "c1",
            "shell",
            serde_json::json!({"command": "echo hi", "sandbox_permissions": "require_escalated"}),
        )
        .with_epoch(0)
        .build(),
        &cfg(),
    );

    assert!(matches!(
        tr.effects.first(),
        Some(Effect::ExecuteTool { approval_policy: agent_core::AskForApproval::Never, .. })
    ));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-turn model_tool_call_effect_carries_turn_approval_policy -v`
Expected: FAIL due missing `with_approval_policy` and/or missing `approval_policy` in `Effect::ExecuteTool`.

**Step 3: Write minimal implementation**

```rust
// TurnState
pub struct TurnState {
    pub meta: SessionMeta,
    pub approval_policy: agent_core::AskForApproval,
    // ...existing fields
}

impl TurnState {
    pub fn new(meta: SessionMeta) -> Self {
        Self {
            meta,
            approval_policy: agent_core::AskForApproval::OnRequest,
            // ...existing init
        }
    }
}

// runtime_impl.rs
let TurnRequest { meta, initial_input, transcript, approval_policy } = request;
let mut state = TurnState::new(meta.clone());
state.approval_policy = approval_policy;

// effect.rs
ExecuteTool {
    epoch: u64,
    session_id: String,
    turn_id: String,
    approval_policy: agent_core::AskForApproval,
    call: ToolCall,
}

// reducer.rs
tr.add_effect(Effect::ExecuteTool {
    epoch,
    session_id: tr.state.meta.session_id.clone(),
    turn_id: tr.state.meta.turn_id.clone(),
    approval_policy: tr.state.approval_policy,
    call,
});
```

Update test helpers builder and all `TurnRequest { ... }` call sites (`agent`, `agent-session`) to include `approval_policy: AskForApproval::OnRequest`.

**Step 4: Run test to verify it passes**

Run:
```bash
cargo test -p agent-turn model_tool_call_effect_carries_turn_approval_policy -v
cargo test -p agent-session --lib -v
cargo test -p agent --lib -v
```
Expected: PASS

**Step 5: Commit**

```bash
git add agent-turn/src/state.rs agent-turn/src/runtime_impl.rs agent-turn/src/effect.rs agent-turn/src/reducer.rs agent-turn/src/test_helpers.rs agent/src/agent.rs agent-session/src/session_runtime.rs
git commit -m "feat(turn): propagate approval policy into execute-tool effects"
```

### Task 3: Enforce Escalated Permission Guard In EffectExecutor

**Files:**
- Modify: `agent-turn/src/effect.rs`

**Step 1: Write the failing test**

Add to `agent-turn/src/effect.rs` tests:

```rust
#[tokio::test]
async fn escalated_permissions_are_rejected_when_policy_is_not_on_request() {
    // setup dummy runtime and counting tool
    // execute Effect::ExecuteTool with approval_policy Never and
    // call.arguments containing sandbox_permissions=require_escalated
    // assert ToolResultErr is emitted and tool execute count stays 0
}

#[tokio::test]
async fn escalated_permissions_are_allowed_on_on_request_policy() {
    // same setup but approval_policy OnRequest
    // assert ToolResultOk is emitted and tool execute count is 1
}
```

Use concrete args payload:

```rust
serde_json::json!({
    "command": "echo hi",
    "sandbox_permissions": "require_escalated"
})
```

**Step 2: Run test to verify it fails**

Run:
```bash
cargo test -p agent-turn escalated_permissions_are_rejected_when_policy_is_not_on_request -v
cargo test -p agent-turn escalated_permissions_are_allowed_on_on_request_policy -v
```
Expected: FAIL (no guard yet).

**Step 3: Write minimal implementation**

```rust
#[derive(Debug, Deserialize, Default)]
struct PermissionArgs {
    #[serde(default)]
    sandbox_permissions: Option<agent_core::SandboxPermissions>,
}

fn requested_permissions(args: &serde_json::Value) -> agent_core::SandboxPermissions {
    serde_json::from_value::<PermissionArgs>(args.clone())
        .ok()
        .and_then(|v| v.sandbox_permissions)
        .unwrap_or(agent_core::SandboxPermissions::UseDefault)
}

if requested_permissions(&call.arguments).requires_escalated_permissions()
    && !matches!(approval_policy, agent_core::AskForApproval::OnRequest)
{
    let _ = tx.send(RuntimeEvent::ToolResultErr {
        event_id: new_id(),
        epoch,
        result: ToolResult::err(
            call.call_id.clone(),
            format!(
                "approval policy is {:?}; reject command — escalated permissions require on_request",
                approval_policy
            ),
        ),
    });
    return;
}
```

**Step 4: Run test to verify it passes**

Run:
```bash
cargo test -p agent-turn escalated_permissions_are_rejected_when_policy_is_not_on_request -v
cargo test -p agent-turn escalated_permissions_are_allowed_on_on_request_policy -v
cargo test -p agent-turn -v
```
Expected: PASS

**Step 5: Commit**

```bash
git add agent-turn/src/effect.rs
git commit -m "feat(turn): block escalated tool execution when approval policy disallows it"
```

### Task 4: Expose Permission Parameters In Shell Tool Schema

**Files:**
- Modify: `agent-tool/src/builtin/shell.rs`
- Modify: `agent-tool/tests/runtime_adapter_test.rs`

**Step 1: Write the failing test**

Add to `agent-tool/tests/runtime_adapter_test.rs`:

```rust
use agent_core::tools::ToolCatalog;

#[tokio::test]
async fn shell_tool_spec_exposes_permission_fields() {
    let rt = AgentToolRuntime::default_with_builtins().await;
    let spec = rt.tool_spec("shell").await.expect("shell spec exists");

    let properties = &spec.input_schema["properties"];
    assert!(properties.get("sandbox_permissions").is_some());
    assert!(properties.get("justification").is_some());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-tool --test runtime_adapter_test shell_tool_spec_exposes_permission_fields -v`
Expected: FAIL because schema properties are missing.

**Step 3: Write minimal implementation**

Update shell tool schema in `agent-tool/src/builtin/shell.rs`:

```rust
"sandbox_permissions": {
  "type": "string",
  "enum": ["use_default", "require_escalated"],
  "description": "Set to require_escalated to request execution without sandbox restrictions"
},
"justification": {
  "type": "string",
  "description": "Only set when sandbox_permissions=require_escalated"
}
```

Do not change execution behavior in this task (guard is in `agent-turn`).

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-tool --test runtime_adapter_test -v`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-tool/src/builtin/shell.rs agent-tool/tests/runtime_adapter_test.rs
git commit -m "feat(tool): add permission-related shell schema fields"
```

### Task 5: End-To-End Verification Before Merge

**Files:**
- Modify: none
- Test: workspace crates touched by this feature

**Step 1: Add a failing regression assertion (if missing) in reducer/effect tests**

Use existing tests added in Tasks 2-3. If either assertion is missing, add it now before final verification.

**Step 2: Run full impacted test matrix**

Run:
```bash
cargo test -p agent-core -v
cargo test -p agent-turn -v
cargo test -p agent-tool -v
cargo test -p agent-session -v
cargo test -p agent -v
```
Expected: PASS across all five crates.

**Step 3: Run formatting/lint checks used by this repo**

Run:
```bash
cargo fmt --all -- --check
cargo clippy -p agent-core -p agent-turn -p agent-tool -p agent-session -p agent --all-targets -- -D warnings
```
Expected: PASS

**Step 4: Re-run one critical guard test as a smoke check**

Run: `cargo test -p agent-turn escalated_permissions_are_rejected_when_policy_is_not_on_request -v`
Expected: PASS

**Step 5: Commit verification evidence**

```bash
git add -A
git commit -m "chore: finalize permission guard v0 verification"
```

If no file changes from Step 1, skip this commit.
