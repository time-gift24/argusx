# Advanced Session Features Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add multi-thread session features on top of the persisted conversation layer, including thread switching, checkpoints, and explicit recovery workflows.

**Architecture:** Build these features only after persistence is stable. Keep the manager as the orchestration boundary, store thread metadata separately from turn history, and make checkpoint/recovery behavior explicit instead of pretending a live `turn` can resume from in-flight runtime state.

**Tech Stack:** Rust (desktop Tauri backend, chat orchestration), TypeScript (desktop chat workspace), cargo integration tests, Vitest

---

### Task 1: Add thread catalog and selection state

**Files:**
- Create: `desktop/src-tauri/src/chat/threads.rs`
- Modify: `desktop/src-tauri/src/chat/commands.rs`
- Create: `desktop/src-tauri/tests/chat_threads_test.rs`
- Create: `desktop/lib/chat-threads.test.ts`

**Step 1: Write the failing test**

- Add backend and frontend tests covering:
  - creating a second conversation thread
  - listing threads with title and last-updated metadata
  - switching the active thread without corrupting history

**Step 2: Run test to verify it fails**

Run:

- `cargo test -p desktop --test chat_threads_test -- --nocapture`
- `pnpm --dir desktop exec vitest run lib/chat-threads.test.ts`

Expected:

- failures because thread catalog and switch commands do not exist yet

**Step 3: Write minimal implementation**

- Add thread catalog state in `chat/threads.rs`
- Expose command helpers and Tauri commands for list/create/switch
- Add frontend IPC helpers for thread management

**Step 4: Run test to verify it passes**

Run:

- `cargo test -p desktop --test chat_threads_test -- --nocapture`
- `pnpm --dir desktop exec vitest run lib/chat-threads.test.ts`

Expected:

- thread catalog tests pass

**Step 5: Commit**

```bash
git add desktop/src-tauri/src/chat/threads.rs \
  desktop/src-tauri/src/chat/commands.rs \
  desktop/src-tauri/tests/chat_threads_test.rs \
  desktop/lib/chat-threads.test.ts
git commit -m "feat(desktop): add conversation thread catalog"
```

### Task 2: Add durable checkpoints and undo markers

**Files:**
- Create: `desktop/src-tauri/src/chat/checkpoints.rs`
- Modify: `desktop/src-tauri/src/chat/storage.rs`
- Modify: `desktop/src-tauri/src/chat/commands.rs`
- Create: `desktop/src-tauri/tests/chat_checkpoints_test.rs`

**Step 1: Write the failing test**

- Add `desktop/src-tauri/tests/chat_checkpoints_test.rs` covering:
  - creating a checkpoint from a completed turn
  - restoring history to a previous checkpoint
  - undo creating a new branch instead of mutating historical records in place

**Step 2: Run test to verify it fails**

Run: `cargo test -p desktop --test chat_checkpoints_test -- --nocapture`

Expected:

- failures because checkpoint storage and restore commands do not exist yet

**Step 3: Write minimal implementation**

- Add checkpoint records tied to persisted conversation ids
- Expose create/restore commands in the orchestration layer
- Keep restore behavior explicit: rebuild a new active conversation state from stored history

**Step 4: Run test to verify it passes**

Run: `cargo test -p desktop --test chat_checkpoints_test -- --nocapture`

Expected:

- checkpoint tests pass

**Step 5: Commit**

```bash
git add desktop/src-tauri/src/chat/checkpoints.rs \
  desktop/src-tauri/src/chat/storage.rs \
  desktop/src-tauri/src/chat/commands.rs \
  desktop/src-tauri/tests/chat_checkpoints_test.rs
git commit -m "feat(desktop): add conversation checkpoints"
```

### Task 3: Add explicit recovery and restart flows

**Files:**
- Modify: `desktop/src-tauri/src/chat/manager.rs`
- Modify: `desktop/src-tauri/src/chat/commands.rs`
- Modify: `desktop/app/chat/page.tsx`
- Modify: `desktop/app/chat/page.test.tsx`
- Create: `desktop/src-tauri/tests/chat_recovery_test.rs`

**Step 1: Write the failing test**

- Add backend and frontend tests covering:
  - startup marks interrupted turns as restartable instead of resumable
  - the UI offers a restart action for interrupted turns
  - restarting creates a new turn from the last durable checkpoint or persisted history

**Step 2: Run test to verify it fails**

Run:

- `cargo test -p desktop --test chat_recovery_test -- --nocapture`
- `pnpm --dir desktop exec vitest run app/chat/page.test.tsx`

Expected:

- failures because recovery metadata and restart UI are not implemented yet

**Step 3: Write minimal implementation**

- Add explicit interrupted/restartable conversation state in the manager
- Expose restart commands instead of pretending in-flight `turn` runtime can resume
- Add restart affordance to the chat page

**Step 4: Run test to verify it passes**

Run:

- `cargo test -p desktop --test chat_recovery_test -- --nocapture`
- `pnpm --dir desktop exec vitest run app/chat/page.test.tsx`

Expected:

- recovery tests pass

**Step 5: Commit**

```bash
git add desktop/src-tauri/src/chat/manager.rs \
  desktop/src-tauri/src/chat/commands.rs \
  desktop/app/chat/page.tsx \
  desktop/app/chat/page.test.tsx \
  desktop/src-tauri/tests/chat_recovery_test.rs
git commit -m "feat(desktop): add explicit recovery flows"
```
