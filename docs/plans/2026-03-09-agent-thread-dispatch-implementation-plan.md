# Agent Thread Dispatch Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build global agent profiles, thread-bound frozen agent snapshots, and blocking subagent dispatch/monitoring while preserving the current `Session -> Thread -> Turn` ownership boundary.

**Architecture:** Add a new `agent` crate for profile storage, prompt resolution, and orchestration tools; extend the session schema for thread bindings, frozen snapshots, and dispatch records; plumb agent system prompts through turn startup; and keep subagent waiting inside a tool-side waiter that listens to session events with store polling fallback.

**Tech Stack:** Rust workspace, sqlx/sqlite, tokio, session/turn/tool crates, Tauri desktop runtime, Next.js desktop UI tests.

---

### Task 0: Create an isolated implementation worktree

**Files:**
- Reference: `docs/plans/2026-03-09-agent-thread-dispatch-design.md`

**Step 1: Create the worktree and branch**

Run:

```bash
git worktree add /Users/wanyaozhong/Projects/argusx-agent-thread-dispatch -b codex/agent-thread-dispatch
```

Expected: Git prints `Preparing worktree` and checks out a new `codex/agent-thread-dispatch` branch.

**Step 2: Switch to the worktree and verify the branch**

Run:

```bash
cd /Users/wanyaozhong/Projects/argusx-agent-thread-dispatch
git rev-parse --abbrev-ref HEAD
```

Expected: `codex/agent-thread-dispatch`

**Step 3: Run the baseline targeted tests before touching code**

Run:

```bash
cargo test -p session --test session_manager_flow --test session_resume_test
cargo test -p turn --test transcript_turn_test --test permission_turn_test
cargo test -p desktop --test chat_tools_test --test chat_model_test
```

Expected: PASS. If any baseline test already fails, stop and record that failure before continuing.

### Task 1: Add the `agent` crate and global profile registry

**Files:**
- Create: `agent/Cargo.toml`
- Create: `agent/src/lib.rs`
- Create: `agent/src/types.rs`
- Create: `agent/src/store.rs`
- Create: `agent/tests/profile_store_test.rs`
- Modify: `Cargo.toml`

**Step 1: Write the failing profile store test**

Create `agent/tests/profile_store_test.rs` with a single end-to-end test that proves builtin seeding plus custom CRUD:

```rust
#[tokio::test(flavor = "current_thread")]
async fn store_seeds_builtin_main_agent_and_round_trips_custom_profile() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let store = agent::AgentProfileStore::new(pool);

    store.init_schema().await.unwrap();
    store.seed_builtin_profiles().await.unwrap();

    let builtin = store.get_profile("builtin-main").await.unwrap().unwrap();
    assert!(matches!(builtin.kind, agent::AgentProfileKind::BuiltinMain));
    assert!(builtin.allow_subagent_dispatch);

    let custom = agent::AgentProfileRecord::custom(
        "reviewer",
        "Reviewer",
        "Review code for regressions",
        "You are a strict reviewer.",
        serde_json::json!({"builtins": ["read", "grep"]}),
    );

    store.upsert_profile(&custom).await.unwrap();

    let loaded = store.get_profile("reviewer").await.unwrap().unwrap();
    assert_eq!(loaded.display_name, "Reviewer");
    assert!(!loaded.allow_subagent_dispatch);
}
```

**Step 2: Run the test to verify it fails**

Run:

```bash
cargo test -p agent --test profile_store_test store_seeds_builtin_main_agent_and_round_trips_custom_profile -- --exact
```

Expected: FAIL with an error like `package ID specification 'agent' did not match any packages` or unresolved `agent` imports.

**Step 3: Add the new crate to the workspace and implement the minimal store**

Implement:

- `Cargo.toml` workspace member entry for `agent`
- `agent/Cargo.toml` with `sqlx`, `chrono`, `serde`, `serde_json`, `tokio`, and `anyhow`
- `AgentProfileKind`
- `AgentProfileRecord`
- `AgentProfileStore::{new, init_schema, seed_builtin_profiles, upsert_profile, get_profile}`

Use a single builtin seed row:

```rust
AgentProfileRecord {
    id: "builtin-main".into(),
    kind: AgentProfileKind::BuiltinMain,
    display_name: "Planner".into(),
    description: "System planning and dispatch agent".into(),
    system_prompt: builtin_main_prompt().into(),
    tool_policy_json: serde_json::json!({
        "builtins": ["read", "glob", "grep", "update_plan", "dispatch_subagent", "list_subagent_dispatches", "get_subagent_dispatch"]
    }),
    model_config_json: serde_json::Value::Null,
    allow_subagent_dispatch: true,
    is_active: true,
    created_at: Utc::now(),
    updated_at: Utc::now(),
}
```

**Step 4: Run the new crate tests**

Run:

```bash
cargo test -p agent --test profile_store_test
```

Expected: PASS

**Step 5: Commit**

Run:

```bash
git add Cargo.toml agent/Cargo.toml agent/src/lib.rs agent/src/types.rs agent/src/store.rs agent/tests/profile_store_test.rs
git commit -m "feat: add agent profile registry"
```

### Task 2: Extend session persistence for agent-bound threads and dispatch tracking

**Files:**
- Modify: `sql/session_schema.sql`
- Modify: `session/src/types.rs`
- Modify: `session/src/store.rs`
- Modify: `session/src/lib.rs`
- Create: `session/tests/agent_thread_binding_test.rs`

**Step 1: Write the failing session persistence test**

Create `session/tests/agent_thread_binding_test.rs`:

```rust
#[tokio::test(flavor = "current_thread")]
async fn store_round_trips_thread_agent_snapshot_and_dispatch_record() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let store = session::store::ThreadStore::new(pool);
    store.init_schema().await.unwrap();

    let thread_id = uuid::Uuid::new_v4();
    let parent_turn_id = uuid::Uuid::new_v4();

    store
        .insert_thread_agent_snapshot(&session::ThreadAgentSnapshotRecord {
            thread_id,
            profile_id: "reviewer".into(),
            display_name_snapshot: "Reviewer".into(),
            system_prompt_snapshot: "You are a reviewer.".into(),
            tool_policy_snapshot_json: serde_json::json!({"builtins": ["read"]}),
            model_config_snapshot_json: serde_json::Value::Null,
            allow_subagent_dispatch_snapshot: false,
            created_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    store
        .insert_subagent_dispatch(&session::SubagentDispatchRecord {
            id: uuid::Uuid::new_v4(),
            parent_thread_id: thread_id,
            parent_turn_id,
            dispatch_tool_call_id: "call-1".into(),
            child_thread_id: uuid::Uuid::new_v4(),
            child_agent_profile_id: "reviewer".into(),
            status: session::SubagentDispatchStatus::Running,
            requested_at: chrono::Utc::now(),
            finished_at: None,
            result_summary: None,
        })
        .await
        .unwrap();

    let snapshot = store.get_thread_agent_snapshot(thread_id).await.unwrap().unwrap();
    assert_eq!(snapshot.profile_id, "reviewer");

    let dispatches = store.list_subagent_dispatches(thread_id).await.unwrap();
    assert_eq!(dispatches.len(), 1);
}
```

**Step 2: Run the test to verify it fails**

Run:

```bash
cargo test -p session --test agent_thread_binding_test store_round_trips_thread_agent_snapshot_and_dispatch_record -- --exact
```

Expected: FAIL with missing types or methods such as `ThreadAgentSnapshotRecord` or `insert_subagent_dispatch`.

**Step 3: Add the new schema and records**

Implement:

- `threads.agent_profile_id TEXT`
- `threads.is_subagent INTEGER NOT NULL DEFAULT 0`
- new tables `thread_agent_snapshots` and `subagent_dispatches`
- new `session::types` records:
  - `ThreadAgentSnapshotRecord`
  - `SubagentDispatchRecord`
  - `SubagentDispatchStatus`
- new store methods:
  - `insert_thread_agent_snapshot`
  - `get_thread_agent_snapshot`
  - `insert_subagent_dispatch`
  - `update_subagent_dispatch`
  - `list_subagent_dispatches`
  - `mark_incomplete_dispatches_interrupted`

Keep `ThreadRecord` focused on thread identity and turn sequencing. Do not move mutable profile data into `ThreadRecord`.

**Step 4: Run the new and existing session tests**

Run:

```bash
cargo test -p session --test agent_thread_binding_test
cargo test -p session --test session_manager_flow --test session_resume_test --test transcript_persistence_test
```

Expected: PASS

**Step 5: Commit**

Run:

```bash
git add sql/session_schema.sql session/src/types.rs session/src/store.rs session/src/lib.rs session/tests/agent_thread_binding_test.rs
git commit -m "feat: persist agent thread bindings and dispatch records"
```

### Task 3: Plumb system prompts through turn startup without polluting transcript history

**Files:**
- Modify: `turn/src/context.rs`
- Modify: `turn/src/model.rs`
- Modify: `turn/src/driver.rs`
- Create: `turn/tests/system_prompt_turn_test.rs`
- Modify: `desktop/src-tauri/src/chat/model.rs`
- Modify: `desktop/src-tauri/tests/chat_model_test.rs`

**Step 1: Write the failing turn test**

Create `turn/tests/system_prompt_turn_test.rs`:

```rust
#[tokio::test(flavor = "current_thread")]
async fn turn_seed_system_prompt_reaches_model_but_not_transcript() {
    let model = support::FakeModelRunner::new_text_only("done");
    let seed = turn::TurnSeed {
        session_id: "session-1".into(),
        turn_id: "turn-1".into(),
        prior_messages: vec![],
        user_message: "hello".into(),
        system_prompt: Some("You are a planner.".into()),
    };

    let (_handle, task) = turn::TurnDriver::spawn(
        seed,
        std::sync::Arc::new(model.clone()),
        std::sync::Arc::new(support::FakeToolRunner::default()),
        std::sync::Arc::new(support::FakeAuthorizer::allow_all()),
        std::sync::Arc::new(support::FakeObserver::default()),
    );

    let outcome = task.await.unwrap().unwrap();
    assert_eq!(model.last_request().unwrap().system_prompt.as_deref(), Some("You are a planner."));
    assert!(!outcome.transcript.iter().any(|msg| matches!(msg, turn::TurnMessage::SystemNote { .. })));
}
```

**Step 2: Run the test to verify it fails**

Run:

```bash
cargo test -p turn --test system_prompt_turn_test turn_seed_system_prompt_reaches_model_but_not_transcript -- --exact
```

Expected: FAIL with `struct TurnSeed has no field named system_prompt` or missing `LlmStepRequest.system_prompt`.

**Step 3: Implement minimal prompt plumbing**

Make these changes:

- add `system_prompt: Option<String>` to `TurnSeed`
- add `system_prompt: Option<String>` to `LlmStepRequest`
- pass `self.seed.system_prompt.clone()` from `TurnDriver` when building each request
- in `desktop/src-tauri/src/chat/model.rs`, prepend one provider `system` message from `request.system_prompt`

Do not:

- append the prompt to the persisted transcript
- turn it into replayable `SystemNote` history

**Step 4: Run turn and desktop model tests**

Run:

```bash
cargo test -p turn --test system_prompt_turn_test --test transcript_turn_test
cargo test -p desktop --test chat_model_test
```

Expected: PASS

**Step 5: Commit**

Run:

```bash
git add turn/src/context.rs turn/src/model.rs turn/src/driver.rs turn/tests/system_prompt_turn_test.rs desktop/src-tauri/src/chat/model.rs desktop/src-tauri/tests/chat_model_test.rs
git commit -m "feat: add per-turn system prompt plumbing"
```

### Task 4: Implement prompt composition and the agent execution resolver

**Files:**
- Create: `agent/src/prompts.rs`
- Create: `agent/src/resolver.rs`
- Modify: `agent/src/lib.rs`
- Create: `agent/tests/execution_resolver_test.rs`

**Step 1: Write the failing resolver test**

Create `agent/tests/execution_resolver_test.rs`:

```rust
#[tokio::test(flavor = "current_thread")]
async fn resolver_merges_session_prompt_builtin_role_and_thread_snapshot() {
    let snapshot = agent::ThreadAgentSnapshot {
        profile_id: "builtin-main".into(),
        display_name_snapshot: "Planner".into(),
        system_prompt_snapshot: "You are the main planner.".into(),
        tool_policy_snapshot_json: serde_json::json!({
            "builtins": ["read", "update_plan", "dispatch_subagent"]
        }),
        model_config_snapshot_json: serde_json::Value::Null,
        allow_subagent_dispatch_snapshot: true,
    };

    let resolved = agent::AgentExecutionResolver::new()
        .resolve("Session base prompt", &snapshot)
        .unwrap();

    assert!(resolved.system_prompt.contains("Session base prompt"));
    assert!(resolved.system_prompt.contains("You are the main planner."));
    assert!(resolved.system_prompt.contains("dispatch_subagent"));
    assert!(resolved.allow_subagent_dispatch);
}
```

**Step 2: Run the test to verify it fails**

Run:

```bash
cargo test -p agent --test execution_resolver_test resolver_merges_session_prompt_builtin_role_and_thread_snapshot -- --exact
```

Expected: FAIL with unresolved `AgentExecutionResolver` or prompt builder functions.

**Step 3: Implement the resolver and prompt builder**

Implement:

- `builtin_main_prompt_block()`
- platform tool-usage guidance
- prompt merge order:
  1. platform rules
  2. session prompt
  3. builtin role block
  4. frozen thread prompt
  5. tool surface block
- `ResolvedAgentExecution` with:
  - `system_prompt`
  - `tool_policy`
  - `model_override`
  - `allow_subagent_dispatch`

Keep all prompt assembly in this module. Do not scatter string concatenation across session or desktop code.

**Step 4: Run the agent tests**

Run:

```bash
cargo test -p agent --test execution_resolver_test --test profile_store_test
```

Expected: PASS

**Step 5: Commit**

Run:

```bash
git add agent/src/lib.rs agent/src/prompts.rs agent/src/resolver.rs agent/tests/execution_resolver_test.rs
git commit -m "feat: add agent execution resolver"
```

### Task 5: Wire runtime bootstrap and desktop state to shared agent services

**Files:**
- Modify: `runtime/src/bootstrap.rs`
- Modify: `runtime/src/lib.rs`
- Modify: `desktop/src-tauri/Cargo.toml`
- Modify: `desktop/src-tauri/src/session_commands.rs`
- Modify: `desktop/src-tauri/src/lib.rs`
- Modify: `runtime/tests/runtime_build_test.rs`

**Step 1: Write the failing runtime bootstrap test**

Extend `runtime/tests/runtime_build_test.rs` with:

```rust
#[tokio::test]
async fn build_runtime_initializes_agent_services() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config = runtime::AppConfig {
        paths: runtime::AppPaths {
            sqlite: temp_dir.path().join("argusx.db"),
            log_file: temp_dir.path().join("argusx.log"),
        },
        ..runtime::AppConfig::default_for_tests()
    };

    let runtime = runtime::build_runtime_from_config(config).await.unwrap();
    let builtin = runtime.agent_profiles.get_profile("builtin-main").await.unwrap();
    assert!(builtin.is_some());
}
```

**Step 2: Run the test to verify it fails**

Run:

```bash
cargo test -p runtime --test runtime_build_test build_runtime_initializes_agent_services -- --exact
```

Expected: FAIL with no `agent_profiles` field on `ArgusxRuntime`.

**Step 3: Implement the runtime wiring**

Make these changes:

- initialize `AgentProfileStore` from the same sqlite pool during runtime bootstrap
- call `init_schema` and `seed_builtin_profiles`
- expose the shared agent services on `ArgusxRuntime`
- update `DesktopSessionState` to receive runtime-owned agent services instead of hardcoding agent-agnostic deps

Keep startup ownership in `runtime`; do not re-home bootstrap logic into `desktop`.

**Step 4: Run the runtime and desktop smoke tests**

Run:

```bash
cargo test -p runtime --test runtime_build_test
cargo test -p desktop --test chat_tools_test --test chat_model_test
```

Expected: PASS

**Step 5: Commit**

Run:

```bash
git add runtime/src/bootstrap.rs runtime/src/lib.rs runtime/tests/runtime_build_test.rs desktop/src-tauri/Cargo.toml desktop/src-tauri/src/session_commands.rs desktop/src-tauri/src/lib.rs
git commit -m "feat: wire agent services through runtime bootstrap"
```

### Task 6: Expose agent profiles to desktop chat and make thread selection agent-aware

**Files:**
- Create: `desktop/src-tauri/src/agent_profiles/mod.rs`
- Create: `desktop/src-tauri/src/agent_profiles/commands.rs`
- Modify: `desktop/src-tauri/src/lib.rs`
- Modify: `desktop/src-tauri/src/chat/commands.rs`
- Modify: `desktop/lib/chat.ts`
- Modify: `desktop/app/chat/page.tsx`
- Modify: `desktop/app/chat/page.test.tsx`
- Create: `desktop/src-tauri/tests/agent_profile_commands_test.rs`

**Step 1: Write the failing desktop tests**

Backend test:

```rust
#[tokio::test(flavor = "current_thread")]
async fn list_agent_profiles_returns_builtin_and_custom_profiles() {
    // boot a temp desktop runtime, seed builtin + custom profile
    // assert the command returns both
}
```

Frontend test addition in `desktop/app/chat/page.test.tsx`:

```tsx
it("loads agent options from the backend and sends the selected profile id", async () => {
  // mock listAgentProfiles + startTurn
  // assert targetId equals selected backend profile id
});
```

**Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test -p desktop --test agent_profile_commands_test list_agent_profiles_returns_builtin_and_custom_profiles -- --exact
pnpm vitest run desktop/app/chat/page.test.tsx
```

Expected: FAIL because the command and frontend fetch path do not exist yet.

**Step 3: Implement agent profile listing and chat selection**

Add:

- Tauri command `list_agent_profiles`
- desktop client helper `listAgentProfiles()`
- `ChatPage` load path to replace the hardcoded `AGENTS` array with backend-sourced options

Then make `start_turn` agent-aware:

- if the active thread already matches the selected agent profile, reuse it
- otherwise create a new thread bound to the selected agent profile and switch focus to it
- do not auto-focus child subagent threads created by dispatch

**Step 4: Run the desktop tests**

Run:

```bash
cargo test -p desktop --test agent_profile_commands_test --test chat_events_test
pnpm vitest run desktop/app/chat/page.test.tsx desktop/components/ai/prompt-composer.test.tsx
```

Expected: PASS

**Step 5: Commit**

Run:

```bash
git add desktop/src-tauri/src/agent_profiles/mod.rs desktop/src-tauri/src/agent_profiles/commands.rs desktop/src-tauri/src/lib.rs desktop/src-tauri/src/chat/commands.rs desktop/src-tauri/tests/agent_profile_commands_test.rs desktop/lib/chat.ts desktop/app/chat/page.tsx desktop/app/chat/page.test.tsx
git commit -m "feat: load chat agent targets from profile registry"
```

### Task 7: Make model/tool registration agent-aware instead of hardcoded

**Files:**
- Modify: `core/src/lib.rs`
- Modify: `agent/src/lib.rs`
- Create: `agent/src/tools/mod.rs`
- Modify: `desktop/src-tauri/src/chat/tools.rs`
- Modify: `desktop/src-tauri/src/chat/authorizer.rs`
- Modify: `desktop/src-tauri/src/chat/model.rs`
- Modify: `desktop/src-tauri/tests/chat_tools_test.rs`
- Modify: `desktop/src-tauri/tests/chat_model_test.rs`

**Step 1: Write the failing tool/model tests**

Add to `desktop/src-tauri/tests/chat_tools_test.rs`:

```rust
#[tokio::test(flavor = "current_thread")]
async fn agent_tool_surface_only_registers_allowed_builtins() {
    let surface = desktop_lib::chat::build_agent_tool_surface(serde_json::json!({
        "builtins": ["read", "update_plan", "dispatch_subagent"]
    })).unwrap();

    assert!(surface.has_builtin("dispatch_subagent"));
    assert!(!surface.has_builtin("shell"));
}
```

Add to `desktop/src-tauri/tests/chat_model_test.rs`:

```rust
#[test]
fn provider_model_runner_emits_only_agent_allowed_tool_definitions() {
    // assert read/update_plan/dispatch_subagent present, shell absent
}
```

**Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test -p desktop --test chat_tools_test agent_tool_surface_only_registers_allowed_builtins -- --exact
cargo test -p desktop --test chat_model_test provider_model_runner_emits_only_agent_allowed_tool_definitions -- --exact
```

Expected: FAIL with missing `dispatch_subagent` builtin names or missing `build_agent_tool_surface`.

**Step 3: Implement agent-aware registration**

Make these changes:

- extend `core::Builtin` with:
  - `DispatchSubagent`
  - `ListSubagentDispatches`
  - `GetSubagentDispatch`
- add builder helpers that convert resolved agent tool policy into:
  - runtime `ToolScheduler` registrations
  - provider tool definitions
  - allowlist/authorizer rules

Do not keep one global hardcoded read-only surface in `chat/model.rs` and `chat/tools.rs`.

**Step 4: Run the impacted tests**

Run:

```bash
cargo test -p core
cargo test -p desktop --test chat_tools_test --test chat_model_test
```

Expected: PASS

**Step 5: Commit**

Run:

```bash
git add core/src/lib.rs agent/src/lib.rs agent/src/tools/mod.rs desktop/src-tauri/src/chat/tools.rs desktop/src-tauri/src/chat/authorizer.rs desktop/src-tauri/src/chat/model.rs desktop/src-tauri/tests/chat_tools_test.rs desktop/src-tauri/tests/chat_model_test.rs
git commit -m "feat: resolve model and tool surfaces from agent policy"
```

### Task 8: Implement `dispatch_subagent` and monitoring tools with an internal waiter

**Files:**
- Create: `agent/src/tools/dispatch_subagent.rs`
- Create: `agent/src/tools/list_subagent_dispatches.rs`
- Create: `agent/src/tools/get_subagent_dispatch.rs`
- Modify: `agent/src/tools/mod.rs`
- Modify: `agent/src/lib.rs`
- Create: `agent/tests/dispatch_tool_test.rs`
- Create: `agent/tests/monitoring_tools_test.rs`
- Modify: `desktop/src-tauri/src/chat/tools.rs`

**Step 1: Write the failing dispatch tests**

`agent/tests/dispatch_tool_test.rs`:

```rust
#[tokio::test(flavor = "current_thread")]
async fn dispatch_subagent_waits_for_child_turn_and_returns_structured_summary() {
    // create temp sqlite + session manager + builtin main parent thread + reviewer child profile
    // execute dispatch_subagent
    // assert it returns child ids + completed status + final_output summary
}
```

`agent/tests/monitoring_tools_test.rs`:

```rust
#[tokio::test(flavor = "current_thread")]
async fn get_subagent_dispatch_reports_waiting_permission_metadata() {
    // put a child turn into WaitingPermission
    // assert waiting_permission_request_id and waiting_tool_call_id are returned
}
```

**Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test -p agent --test dispatch_tool_test dispatch_subagent_waits_for_child_turn_and_returns_structured_summary -- --exact
cargo test -p agent --test monitoring_tools_test get_subagent_dispatch_reports_waiting_permission_metadata -- --exact
```

Expected: FAIL with missing tool implementations.

**Step 3: Implement the tools**

Implement three tools:

- `DispatchSubagentTool`
- `ListSubagentDispatchesTool`
- `GetSubagentDispatchTool`

For `DispatchSubagentTool`, use this wait model:

```rust
loop {
    tokio::select! {
        maybe_event = rx.recv() => {
            if child_turn_is_terminal(&maybe_event, child_turn_id) {
                break;
            }
        }
        _ = tokio::time::sleep(Duration::from_millis(250)) => {
            let turn = store.get_turn(child_turn_id).await?;
            if is_terminal(turn.status) {
                break;
            }
        }
        _ = cancel_token.cancelled() => {
            cancel_child_turn_best_effort().await;
            bail!("dispatch_subagent cancelled");
        }
    }
}
```

This wait loop belongs inside the tool implementation, not inside `TurnDriver`.

**Step 4: Run the new and impacted tests**

Run:

```bash
cargo test -p agent --test dispatch_tool_test --test monitoring_tools_test
cargo test -p desktop --test chat_tools_test
```

Expected: PASS

**Step 5: Commit**

Run:

```bash
git add agent/src/tools/dispatch_subagent.rs agent/src/tools/list_subagent_dispatches.rs agent/src/tools/get_subagent_dispatch.rs agent/src/tools/mod.rs agent/src/lib.rs agent/tests/dispatch_tool_test.rs agent/tests/monitoring_tools_test.rs desktop/src-tauri/src/chat/tools.rs
git commit -m "feat: add blocking subagent dispatch and monitoring tools"
```

### Task 9: Harden cancellation, permission, restart recovery, and transcript integrity

**Files:**
- Modify: `session/src/manager.rs`
- Modify: `session/src/store.rs`
- Modify: `agent/src/tools/dispatch_subagent.rs`
- Create: `session/tests/subagent_dispatch_flow_test.rs`
- Modify: `session/tests/session_resume_test.rs`
- Modify: `session/tests/transcript_persistence_test.rs`
- Modify: `desktop/src-tauri/tests/chat_events_test.rs`

**Step 1: Write the failing integration tests**

Create `session/tests/subagent_dispatch_flow_test.rs` with at least these cases:

```rust
#[tokio::test(flavor = "current_thread")]
async fn parent_cancel_attempts_best_effort_child_cancel() {
    // dispatch a child turn, cancel parent while waiting
    // assert child cancel was attempted and dispatch status is terminal
}

#[tokio::test(flavor = "current_thread")]
async fn restart_marks_running_dispatches_interrupted() {
    // seed Running parent turn + Running child turn + Running dispatch
    // initialize SessionManager
    // assert parent/child turns and dispatch all become Interrupted
}
```

Extend `session/tests/transcript_persistence_test.rs` with a case asserting:

```rust
assert_eq!(parent_turn.transcript, vec![
    PersistedMessage::User { .. },
    PersistedMessage::AssistantToolCalls { .. },
    PersistedMessage::ToolResult { tool_name: "dispatch_subagent".into(), .. },
]);
```

**Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test -p session --test subagent_dispatch_flow_test parent_cancel_attempts_best_effort_child_cancel -- --exact
cargo test -p session --test subagent_dispatch_flow_test restart_marks_running_dispatches_interrupted -- --exact
cargo test -p session --test transcript_persistence_test
```

Expected: FAIL because cancellation propagation, dispatch interruption, and parent-child transcript assertions are not implemented yet.

**Step 3: Implement the minimal hardening**

Make these changes:

- on runtime initialization, mark incomplete dispatch rows `Interrupted`
- on parent cancel while waiting in `DispatchSubagentTool`, attempt best-effort child cancel
- keep child permission local to child turn and expose it through `get_subagent_dispatch`
- ensure parent transcript only records the final `dispatch_subagent` tool result, not child history

Do not:

- invent a resumable subagent-wait turn state
- duplicate child transcript messages into the parent thread

**Step 4: Run the full targeted regression set**

Run:

```bash
cargo test -p session --test session_manager_flow --test session_resume_test --test transcript_persistence_test --test subagent_dispatch_flow_test
cargo test -p turn --test transcript_turn_test --test cancel_turn_test --test permission_turn_test
cargo test -p agent --test profile_store_test --test execution_resolver_test --test dispatch_tool_test --test monitoring_tools_test
cargo test -p desktop --test chat_tools_test --test chat_model_test --test chat_events_test --test agent_profile_commands_test
pnpm vitest run desktop/app/chat/page.test.tsx desktop/components/ai/prompt-composer.test.tsx
```

Expected: PASS

**Step 5: Commit**

Run:

```bash
git add session/src/manager.rs session/src/store.rs agent/src/tools/dispatch_subagent.rs session/tests/subagent_dispatch_flow_test.rs session/tests/session_resume_test.rs session/tests/transcript_persistence_test.rs desktop/src-tauri/tests/chat_events_test.rs
git commit -m "feat: harden subagent cancellation and recovery flows"
```

### Task 10: Final verification and design drift check

**Files:**
- Reference: `docs/plans/2026-03-09-agent-thread-dispatch-design.md`
- Reference: `docs/plans/2026-03-09-agent-thread-dispatch-implementation-plan.md`

**Step 1: Run formatting**

Run:

```bash
cargo fmt --all
pnpm --dir desktop exec prettier --check app/chat/page.tsx lib/chat.ts
```

Expected: `cargo fmt` exits 0 and Prettier reports no changed files.

**Step 2: Re-run the end-to-end targeted suite after formatting**

Run:

```bash
cargo test -p session --test session_manager_flow --test session_resume_test --test transcript_persistence_test --test subagent_dispatch_flow_test
cargo test -p turn --test transcript_turn_test --test cancel_turn_test --test permission_turn_test
cargo test -p agent
cargo test -p desktop --test chat_tools_test --test chat_model_test --test chat_events_test --test agent_profile_commands_test
pnpm vitest run desktop/app/chat/page.test.tsx desktop/components/ai/prompt-composer.test.tsx
```

Expected: PASS

**Step 3: Review for design drift**

Manually compare the implementation against these design rules:

- no turn-level suspend/resume added for subagent waiting
- `Thread` still does not own tool-loop semantics
- existing thread history remains incremental
- dispatch waiting lives inside the tool implementation
- subagents still cannot dispatch nested subagents

**Step 4: Commit any final formatting or test-only adjustments**

Run:

```bash
git add -A
git commit -m "chore: finalize agent thread dispatch implementation"
```

Expected: clean working tree.
