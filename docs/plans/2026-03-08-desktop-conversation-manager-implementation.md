# Desktop Conversation Manager Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire the desktop chat surface to the `turn` runtime and add an in-memory conversation manager that supports single-thread multi-turn history replay.

**Architecture:** Build a `desktop/src-tauri/src/chat` backend module that adapts `provider`, `tool`, and `turn::TurnDriver::spawn_recording()` into Tauri commands and a `turn-event` stream. Keep conversation history in an in-memory manager on the desktop backend, and let the frontend own only view state plus IPC subscription logic.

**Tech Stack:** Rust (Tauri v2, turn, provider, tool), TypeScript (Next.js 16, React 19), Vitest, cargo integration tests

---

### Task 1: Add backend chat IPC contracts and event mapping tests

**Files:**
- Modify: `desktop/src-tauri/Cargo.toml`
- Modify: `desktop/src-tauri/src/lib.rs`
- Create: `desktop/src-tauri/src/chat/mod.rs`
- Create: `desktop/src-tauri/src/chat/events.rs`
- Create: `desktop/src-tauri/src/chat/observer.rs`
- Create: `desktop/src-tauri/tests/chat_events_test.rs`
- Create: `desktop/src-tauri/tests/chat_observer_test.rs`

**Step 1: Write the failing tests**

- Add `desktop/src-tauri/tests/chat_events_test.rs` to assert `StartConversationInput`, `ContinueConversationInput`, `CancelConversationInput`, and `DesktopTurnEvent` serialize with camelCase field names.
- Add `desktop/src-tauri/tests/chat_observer_test.rs` to assert a `turn::TurnEvent` maps to a serializable desktop event payload that always includes `conversationId` and `turnId`.

Suggested test shape:

```rust
#[test]
fn desktop_turn_event_serializes_turn_and_conversation_ids() {
    let event = DesktopTurnEvent::text_delta(
        "conversation-1",
        "turn-1",
        "hello",
    );

    let value = serde_json::to_value(event).unwrap();
    assert_eq!(value["conversationId"], "conversation-1");
    assert_eq!(value["turnId"], "turn-1");
    assert_eq!(value["type"], "llm-text-delta");
}
```

**Step 2: Run test to verify it fails**

Run:

- `cargo test -p desktop chat_events_test -- --nocapture`
- `cargo test -p desktop chat_observer_test -- --nocapture`

Expected:

- compile failures because the `chat` module and desktop turn event types do not exist yet

**Step 3: Write minimal implementation**

- Add backend dependencies for `turn`, `tool`, `provider`, `serde`, `serde_json`, `tokio`, and `uuid`.
- Create `desktop/src-tauri/src/chat/mod.rs` plus `events.rs` with IPC payload types for start, continue, cancel, and desktop event envelopes.
- Create `desktop/src-tauri/src/chat/observer.rs` with a small mapper from `turn::TurnEvent` to `DesktopTurnEvent`.
- Export `pub mod chat;` from `desktop/src-tauri/src/lib.rs`.

**Step 4: Run test to verify it passes**

Run:

- `cargo test -p desktop chat_events_test -- --nocapture`
- `cargo test -p desktop chat_observer_test -- --nocapture`

Expected:

- both tests pass

**Step 5: Commit**

```bash
git add desktop/src-tauri/Cargo.toml \
  desktop/src-tauri/src/lib.rs \
  desktop/src-tauri/src/chat/mod.rs \
  desktop/src-tauri/src/chat/events.rs \
  desktop/src-tauri/src/chat/observer.rs \
  desktop/src-tauri/tests/chat_events_test.rs \
  desktop/src-tauri/tests/chat_observer_test.rs
git commit -m "feat(desktop): add chat ipc contracts and event mapping"
```

### Task 2: Add provider/tool adapters and the in-memory conversation manager

**Files:**
- Create: `desktop/src-tauri/src/chat/model.rs`
- Create: `desktop/src-tauri/src/chat/tools.rs`
- Create: `desktop/src-tauri/src/chat/authorizer.rs`
- Create: `desktop/src-tauri/src/chat/manager.rs`
- Modify: `desktop/src-tauri/src/chat/mod.rs`
- Create: `desktop/src-tauri/tests/chat_manager_test.rs`

**Step 1: Write the failing tests**

- Add `desktop/src-tauri/tests/chat_manager_test.rs` covering:
  - starting a new conversation creates a conversation id and first turn id
  - continuing a conversation calls `TurnDriver::spawn_recording()` with prior transcript history
  - cancelling an active conversation removes the live controller but preserves completed history

Suggested test shape:

```rust
#[tokio::test(flavor = "current_thread")]
async fn continuing_conversation_reuses_completed_turn_history() {
    let manager = ConversationManager::new_for_test(/* fake deps */);

    let started = manager
        .start_conversation(StartConversationInput {
            prompt: "hello".into(),
            target_kind: TurnTargetKind::Agent,
            target_id: "reviewer".into(),
        })
        .await
        .unwrap();

    let continued = manager
        .continue_conversation(ContinueConversationInput {
            conversation_id: started.conversation_id.clone(),
            prompt: "continue".into(),
        })
        .await
        .unwrap();

    assert_ne!(started.turn_id, continued.turn_id);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p desktop chat_manager_test -- --nocapture`

Expected:

- compile failures because the manager and adapters do not exist yet

**Step 3: Write minimal implementation**

- Implement `ProviderModelRunner` by mapping `turn::TurnMessage` history into provider chat messages and adapting the provider stream to `turn::ModelRunner`.
- Implement a read-only `ScheduledToolRunner` wrapper over the existing tool scheduler.
- Implement an allow-listed authorizer for `read`, `glob`, and `grep`.
- Implement `ConversationManager` that stores:
  - completed conversation history
  - active `TurnController`
  - live turn metadata needed for event routing
- Make the manager append `TurnOutcome::Completed` transcripts into the current conversation after each finished turn.

**Step 4: Run test to verify it passes**

Run: `cargo test -p desktop chat_manager_test -- --nocapture`

Expected:

- manager tests pass with fake model/tool adapters

**Step 5: Commit**

```bash
git add desktop/src-tauri/src/chat/mod.rs \
  desktop/src-tauri/src/chat/model.rs \
  desktop/src-tauri/src/chat/tools.rs \
  desktop/src-tauri/src/chat/authorizer.rs \
  desktop/src-tauri/src/chat/manager.rs \
  desktop/src-tauri/tests/chat_manager_test.rs
git commit -m "feat(desktop): add in-memory conversation manager"
```

### Task 3: Expose Tauri commands for start, continue, cancel, and event streaming

**Files:**
- Create: `desktop/src-tauri/src/chat/commands.rs`
- Modify: `desktop/src-tauri/src/chat/mod.rs`
- Modify: `desktop/src-tauri/src/lib.rs`
- Create: `desktop/src-tauri/tests/chat_commands_test.rs`

**Step 1: Write the failing tests**

- Add `desktop/src-tauri/tests/chat_commands_test.rs` that asserts the chat command layer can:
  - start a conversation
  - continue an existing conversation
  - cancel an active conversation
- Keep tests at the command/service boundary instead of booting a full Tauri window.

**Step 2: Run test to verify it fails**

Run: `cargo test -p desktop chat_commands_test -- --nocapture`

Expected:

- compile failures because the command functions and shared state are not registered yet

**Step 3: Write minimal implementation**

- Add Tauri command functions in `desktop/src-tauri/src/chat/commands.rs`.
- Register shared `ConversationManager` state during app startup.
- Register chat commands in `tauri::generate_handler![...]`.

**Step 4: Run test to verify it passes**

Run: `cargo test -p desktop chat_commands_test -- --nocapture`

Expected:

- command tests pass

**Step 5: Commit**

```bash
git add desktop/src-tauri/src/chat/mod.rs \
  desktop/src-tauri/src/chat/commands.rs \
  desktop/src-tauri/src/lib.rs \
  desktop/src-tauri/tests/chat_commands_test.rs
git commit -m "feat(desktop): expose conversation commands over tauri"
```

### Task 4: Add frontend IPC client and multi-turn chat page state

**Files:**
- Create: `desktop/lib/chat.ts`
- Create: `desktop/lib/chat.test.ts`
- Modify: `desktop/app/chat/page.tsx`
- Modify: `desktop/app/chat/page.test.tsx`

**Step 1: Write the failing tests**

- Add `desktop/lib/chat.test.ts` that mocks `@tauri-apps/api/core` and `@tauri-apps/api/event` and verifies:
  - `startConversation()`
  - `continueConversation()`
  - `cancelConversation()`
  - `subscribeToTurnEvents()`
- Expand `desktop/app/chat/page.test.tsx` to verify:
  - first submit starts a conversation
  - second submit continues the same conversation
  - streamed text, reasoning, and tool output render into the page
  - active turn can be cancelled

**Step 2: Run test to verify it fails**

Run:

- `pnpm --dir desktop test -- --runInBand lib/chat.test.ts`
- `pnpm --dir desktop test -- --runInBand app/chat/page.test.tsx`

Expected:

- test failures because the IPC layer and multi-turn page state do not exist yet

**Step 3: Write minimal implementation**

- Create `desktop/lib/chat.ts` as the frontend IPC surface.
- Refactor `desktop/app/chat/page.tsx` from a submit-only shell into a view model that tracks:
  - current conversation id
  - active turn id
  - transcript items
  - streaming assistant text
  - reasoning text
  - tool call state
  - submit/cancel status
- Keep rendering grounded in existing `PromptComposer`, `Streamdown`, `Reasoning`, and `ToolCallItem`.

**Step 4: Run test to verify it passes**

Run:

- `pnpm --dir desktop test -- --runInBand lib/chat.test.ts`
- `pnpm --dir desktop test -- --runInBand app/chat/page.test.tsx`

Expected:

- frontend IPC and page tests pass

**Step 5: Commit**

```bash
git add desktop/lib/chat.ts \
  desktop/lib/chat.test.ts \
  desktop/app/chat/page.tsx \
  desktop/app/chat/page.test.tsx
git commit -m "feat(desktop): wire chat page to multi-turn conversation manager"
```

### Task 5: Full desktop verification and architecture progress update

**Files:**
- Modify: `docs/plans/2026-03-08-multi-turn-conversation-implementation.md`

**Step 1: Update architecture progress**

- Mark `In-Memory Conversation Manager` as completed.
- Leave persistence / compression / advanced session features as pending.

**Step 2: Run full backend and frontend verification**

Run:

- `cargo test -p desktop`
- `pnpm --dir desktop test -- --runInBand app/chat/page.test.tsx lib/chat.test.ts`
- `cargo check -p desktop`

Expected:

- desktop backend tests pass
- targeted frontend tests pass
- desktop crate compiles

**Step 3: Run linting if time allows**

Run: `pnpm --dir desktop lint`

Expected:

- no lint errors in touched frontend files

**Step 4: Commit**

```bash
git add docs/plans/2026-03-08-multi-turn-conversation-implementation.md
git commit -m "feat(desktop): complete phase b multi-turn conversation manager"
```
