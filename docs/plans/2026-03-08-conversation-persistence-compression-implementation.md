# Conversation Persistence And Compression Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Persist completed multi-turn conversation history and add a compression boundary that keeps long-running conversations reusable after app restart.

**Architecture:** Keep `turn` as a runtime-only boundary and persist conversation history in the desktop orchestration layer. Introduce a repository that stores completed conversation transcripts, then add a compression service that can replace older history with summaries when the configured threshold is crossed.

**Tech Stack:** Rust (desktop Tauri backend, serde, future SQLite adapter), TypeScript (desktop chat surface), cargo integration tests

---

### Task 1: Add persisted conversation repository contracts

**Files:**
- Create: `desktop/src-tauri/src/chat/storage.rs`
- Modify: `desktop/src-tauri/src/chat/mod.rs`
- Create: `desktop/src-tauri/tests/chat_storage_test.rs`

**Step 1: Write the failing test**

- Add `desktop/src-tauri/tests/chat_storage_test.rs` covering:
  - saving a completed conversation transcript snapshot
  - loading a conversation by id
  - listing conversations ordered by last-updated timestamp

**Step 2: Run test to verify it fails**

Run: `cargo test -p desktop --test chat_storage_test -- --nocapture`

Expected:

- compile failures because the storage module and repository contracts do not exist yet

**Step 3: Write minimal implementation**

- Add `chat/storage.rs` with:
  - `ConversationRecord`
  - `ConversationRepository` trait
  - an in-memory repository implementation used by tests and the first persistence wiring pass
- Re-export the storage module from `chat/mod.rs`

**Step 4: Run test to verify it passes**

Run: `cargo test -p desktop --test chat_storage_test -- --nocapture`

Expected:

- storage tests pass

**Step 5: Commit**

```bash
git add desktop/src-tauri/src/chat/storage.rs \
  desktop/src-tauri/src/chat/mod.rs \
  desktop/src-tauri/tests/chat_storage_test.rs
git commit -m "feat(desktop): add conversation storage contracts"
```

### Task 2: Persist completed multi-turn conversations

**Files:**
- Modify: `desktop/src-tauri/src/chat/manager.rs`
- Modify: `desktop/src-tauri/src/chat/commands.rs`
- Modify: `desktop/src-tauri/src/lib.rs`
- Create: `desktop/src-tauri/tests/chat_persistence_integration_test.rs`

**Step 1: Write the failing test**

- Add `desktop/src-tauri/tests/chat_persistence_integration_test.rs` covering:
  - completed turns are saved through the repository
  - app startup can rehydrate a saved conversation into the manager
  - continue-conversation reuses the persisted transcript after rehydrate

**Step 2: Run test to verify it fails**

Run: `cargo test -p desktop --test chat_persistence_integration_test -- --nocapture`

Expected:

- compile or assertion failures because the manager does not persist or reload conversations yet

**Step 3: Write minimal implementation**

- Inject a `ConversationRepository` into `ConversationManager`
- Save completed transcripts in the watcher path after `TurnOutcome::Completed`
- Add startup wiring that loads stored conversations into manager state before commands are served

**Step 4: Run test to verify it passes**

Run: `cargo test -p desktop --test chat_persistence_integration_test -- --nocapture`

Expected:

- persistence integration tests pass

**Step 5: Commit**

```bash
git add desktop/src-tauri/src/chat/manager.rs \
  desktop/src-tauri/src/chat/commands.rs \
  desktop/src-tauri/src/lib.rs \
  desktop/src-tauri/tests/chat_persistence_integration_test.rs
git commit -m "feat(desktop): persist completed conversations"
```

### Task 3: Add transcript compression policy boundary

**Files:**
- Create: `desktop/src-tauri/src/chat/compression.rs`
- Modify: `desktop/src-tauri/src/chat/manager.rs`
- Create: `desktop/src-tauri/tests/chat_compression_test.rs`

**Step 1: Write the failing test**

- Add `desktop/src-tauri/tests/chat_compression_test.rs` covering:
  - conversations below threshold are stored unchanged
  - conversations above threshold replace old turns with a summary artifact
  - recent turns remain verbatim after compression

**Step 2: Run test to verify it fails**

Run: `cargo test -p desktop --test chat_compression_test -- --nocapture`

Expected:

- compile failures because the compression boundary does not exist yet

**Step 3: Write minimal implementation**

- Add `chat/compression.rs` with:
  - compression policy trait
  - no-op default policy
  - threshold-based summary replacement contract
- Call the compression boundary before saving completed history

**Step 4: Run test to verify it passes**

Run: `cargo test -p desktop --test chat_compression_test -- --nocapture`

Expected:

- compression tests pass

**Step 5: Commit**

```bash
git add desktop/src-tauri/src/chat/compression.rs \
  desktop/src-tauri/src/chat/manager.rs \
  desktop/src-tauri/tests/chat_compression_test.rs
git commit -m "feat(desktop): add conversation compression boundary"
```
