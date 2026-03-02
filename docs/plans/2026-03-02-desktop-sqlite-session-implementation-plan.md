# Desktop SQLite Session Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace desktop session persistence with a SQLite single source of truth, load startup sessions by latest completed turn (`ended_at`) with `updated_at` fallback, default to 24h data window, and keep frontend cache memory-only with a 64MB hard cap.

**Architecture:** Keep `SessionRuntime` as orchestration core and introduce a SQLite storage backend in `agent-session`. Wire desktop Tauri commands to real SQLite-backed queries (session list, default 24h message load, manual full-history pagination). Remove frontend full-history persistence (`zustand persist`) and convert chat store to bounded runtime cache with explicit eviction.

**Tech Stack:** Rust workspace crates (`agent-session`, `desktop`, `agent-core`), SQLite (`rusqlite` + `r2d2_sqlite` or equivalent), Tauri v2 commands/events, Next.js 16, React 19, Zustand, TypeScript, Vitest (for new frontend store/cache tests).

---

## Implementation Notes

- Reference design: `docs/plans/2026-03-02-desktop-sqlite-session-design.md`
- Scope is desktop-only. Do not migrate `agent` or `agent-cli` storage path in this plan.
- Keep changes DRY and YAGNI: only fields/queries needed for defined UX.
- Relevant skills to apply during execution: `@rust-router`, `@test-driven-development`, `@verification-before-completion`.

---

### Task 1: Add SQLite Storage Foundation in `agent-session`

**Files:**
- Modify: `agent-session/Cargo.toml`
- Modify: `agent-session/src/lib.rs`
- Modify: `agent-session/src/storage.rs`
- Create: `agent-session/src/sqlite_store.rs`
- Test: `agent-session/tests/sqlite_store_foundation_test.rs`

**Step 1: Write failing foundation test for SQLite store bootstrap**

```rust
#[tokio::test]
async fn sqlite_store_creates_schema_on_init() {
    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("session.db");
    let store = SqliteSessionStore::new(db_path.clone()).expect("init sqlite store");
    let sessions = store.list(SessionFilter::default()).await.expect("list sessions");
    assert!(sessions.is_empty());
    assert!(db_path.exists());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p agent-session sqlite_store_creates_schema_on_init -- --nocapture`  
Expected: FAIL with missing `SqliteSessionStore` / unresolved imports.

**Step 3: Add dependencies and minimal SQLite store skeleton**

- Add SQLite dependencies in `agent-session/Cargo.toml`.
- Create `SqliteSessionStore` with constructor + schema bootstrap.
- Export module in `agent-session/src/lib.rs`.

**Step 4: Run test to verify it passes**

Run: `cargo test -p agent-session sqlite_store_creates_schema_on_init -- --nocapture`  
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-session/Cargo.toml agent-session/src/lib.rs agent-session/src/storage.rs agent-session/src/sqlite_store.rs agent-session/tests/sqlite_store_foundation_test.rs
git commit -m "feat(agent-session): add sqlite store foundation and schema bootstrap"
```

---

### Task 2: Implement Session/Turn/Transcript CRUD + Ordering in SQLite

**Files:**
- Modify: `agent-session/src/sqlite_store.rs`
- Modify: `agent-session/src/storage.rs`
- Test: `agent-session/tests/sqlite_store_ordering_test.rs`

**Step 1: Write failing tests for required query semantics**

Add tests for:
1. `list` sorting by `last_ended_at DESC` then `updated_at DESC`
2. fallback sorting when no completed turn exists
3. turn summary save/list and transcript save/load sequence

**Step 2: Run tests to verify they fail**

Run: `cargo test -p agent-session sqlite_store_ordering_test -- --nocapture`  
Expected: FAIL on ordering and/or missing turn artifact methods.

**Step 3: Implement SQLite-backed CRUD and ordering SQL**

- Persist `sessions`, `turns`, `transcript_items`.
- Implement aggregation query using `MAX(ended_at_ms)` and fallback ordering.
- Implement transcript insert/load ordered by `(turn_id, seq)`.
- Enforce idempotency with `UNIQUE(turn_id, seq)`.

**Step 4: Run targeted tests**

Run: `cargo test -p agent-session sqlite_store_ordering_test -- --nocapture`  
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-session/src/sqlite_store.rs agent-session/src/storage.rs agent-session/tests/sqlite_store_ordering_test.rs
git commit -m "feat(agent-session): implement sqlite session turn transcript storage with ordering"
```

---

### Task 3: Make `SessionRuntime` Storage Injectable and Keep Existing Behavior

**Files:**
- Modify: `agent-session/src/session_runtime.rs`
- Modify: `agent-session/src/lib.rs`
- Test: `agent-session/src/session_runtime.rs` (new tests) or `agent-session/tests/sqlite_runtime_integration_test.rs`

**Step 1: Write failing runtime integration test with SQLite store**

Test should verify:
1. create session
2. run turn
3. restart runtime with same db
4. list sessions and restore transcript still works

**Step 2: Run test to verify fail**

Run: `cargo test -p agent-session sqlite_runtime_integration -- --nocapture`  
Expected: FAIL because runtime constructor is not yet storage-injectable.

**Step 3: Implement injectable store wiring**

- Add constructor path that accepts a storage backend implementation.
- Keep existing constructor behavior unchanged for non-desktop callers.
- Ensure turn checkpoint/transcript behavior remains compatible.

**Step 4: Run full `agent-session` tests**

Run: `cargo test -p agent-session -- --nocapture`  
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-session/src/session_runtime.rs agent-session/src/lib.rs agent-session/tests/sqlite_runtime_integration_test.rs
git commit -m "refactor(agent-session): support injectable runtime storage backend"
```

---

### Task 4: Wire Desktop Tauri Commands to SQLite Session Runtime

**Files:**
- Modify: `desktop/src-tauri/Cargo.toml`
- Modify: `desktop/src-tauri/src/lib.rs`
- Create: `desktop/src-tauri/src/chat_repository.rs`
- Test: `desktop/src-tauri/src/lib.rs` (new command tests)

**Step 1: Write failing desktop command tests**

Cover:
1. `list_chat_sessions` returns ordered data from sqlite
2. startup query includes `last_ended_at`
3. `get_chat_messages` supports `last_24h` and `all` with cursor

**Step 2: Run tests to verify fail**

Run: `cargo test -p desktop list_chat_sessions -- --nocapture`  
Expected: FAIL because commands currently return mock/empty data.

**Step 3: Implement command repository layer and real queries**

- Add repository that translates runtime/sql rows to desktop API DTOs.
- Replace mock handlers (`list_chat_sessions`, `get_chat_messages`, create/delete/update) with real implementations.
- Keep stream bridge untouched.

**Step 4: Run desktop tests**

Run: `cargo test -p desktop -- --nocapture`  
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/src-tauri/Cargo.toml desktop/src-tauri/src/lib.rs desktop/src-tauri/src/chat_repository.rs
git commit -m "feat(desktop-tauri): back chat commands with sqlite session runtime"
```

---

### Task 5: Frontend API Contract for 24h Default + Full-Range Pagination

**Files:**
- Modify: `desktop/lib/api/chat.ts`
- Modify: `desktop/types/index.ts` (if shared chat DTO types are kept here)
- Test: `desktop/lib/api/chat.test.ts` (new)
- Modify: `desktop/package.json` (add test command/tooling)
- Create: `desktop/vitest.config.ts`

**Step 1: Write failing API contract tests**

Test:
1. default message request uses `range = "last_24h"`
2. full-history request uses `range = "all"` with cursor
3. dto parsing includes `lastEndedAt`

**Step 2: Run tests to verify fail**

Run: `pnpm --dir desktop test chat-api --runInBand`  
Expected: FAIL because API signatures still old and fields missing.

**Step 3: Implement API types and invoke payload updates**

- Add request/response interfaces for range/cursor and overview.
- Update invoke wrappers to match tauri command payload schema.
- Add safe parsing defaults for optional fields.

**Step 4: Run tests to verify pass**

Run: `pnpm --dir desktop test chat-api --runInBand`  
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/lib/api/chat.ts desktop/types/index.ts desktop/lib/api/chat.test.ts desktop/vitest.config.ts desktop/package.json
git commit -m "feat(desktop): add chat api contract for 24h and full-history queries"
```

---

### Task 6: Convert Chat Store to Memory-Only Bootstrap Model

**Files:**
- Modify: `desktop/lib/stores/chat-store.ts`
- Modify: `desktop/components/features/chat/chat-page.tsx`
- Test: `desktop/lib/stores/chat-store.bootstrap.test.ts`

**Step 1: Write failing bootstrap tests**

Test:
1. no `persist` hydration for chat history
2. startup loads session list from backend
3. first session is auto-selected

**Step 2: Run tests to verify fail**

Run: `pnpm --dir desktop test chat-store.bootstrap --runInBand`  
Expected: FAIL because store still uses `persist` and local create-on-empty logic.

**Step 3: Implement memory-only store and bootstrap actions**

- Remove `persist(...)` wrapper for chat store.
- Add async bootstrap actions (`loadSessions`, `loadSessionWindow24h`).
- Update `chat-page.tsx` startup effect to call bootstrap flow and select first session from backend response.

**Step 4: Run tests**

Run: `pnpm --dir desktop test chat-store.bootstrap --runInBand`  
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/lib/stores/chat-store.ts desktop/components/features/chat/chat-page.tsx desktop/lib/stores/chat-store.bootstrap.test.ts
git commit -m "refactor(desktop): make chat store memory-only and backend-bootstrapped"
```

---

### Task 7: Add Full Info Entry and 24h/All Data Loading UX

**Files:**
- Modify: `desktop/components/features/chat/chat-page.tsx`
- Create: `desktop/components/features/chat/chat-full-info-dialog.tsx`
- Modify: `desktop/components/features/chat/conversation-view.tsx`
- Modify: `desktop/components/features/chat/chat-session-bar.tsx` (if props needed)
- Test: `desktop/components/features/chat/chat-full-info-dialog.test.tsx`

**Step 1: Write failing UI behavior tests**

Test:
1. top-left full-info trigger renders
2. opening modal triggers overview + paginated all-range fetch on user action
3. default conversation still shows only 24h window

**Step 2: Run test to verify fail**

Run: `pnpm --dir desktop test chat-full-info-dialog --runInBand`  
Expected: FAIL because component/flow not implemented.

**Step 3: Implement modal flow**

- Add top-left button in `chat-page.tsx`.
- Build dialog component with:
  - overview section
  - explicit "Load Full History" action
  - pagination controls/cursor
- Keep default timeline bound to 24h data unless user requests all.

**Step 4: Run tests**

Run: `pnpm --dir desktop test chat-full-info-dialog --runInBand`  
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/components/features/chat/chat-page.tsx desktop/components/features/chat/chat-full-info-dialog.tsx desktop/components/features/chat/conversation-view.tsx desktop/components/features/chat/chat-session-bar.tsx desktop/components/features/chat/chat-full-info-dialog.test.tsx
git commit -m "feat(desktop): add full-info modal and explicit full-history loading"
```

---

### Task 8: Enforce 64MB Cache Budget with LRU Eviction

**Files:**
- Create: `desktop/lib/stores/chat-cache-budget.ts`
- Modify: `desktop/lib/stores/chat-store.ts`
- Test: `desktop/lib/stores/chat-cache-budget.test.ts`

**Step 1: Write failing cache budget tests**

Test:
1. overflow above 64MB evicts non-active session details first
2. active session old details collapse to previews
3. large per-turn fields are truncated to cap

**Step 2: Run tests to verify fail**

Run: `pnpm --dir desktop test chat-cache-budget --runInBand`  
Expected: FAIL because no budget manager/eviction exists.

**Step 3: Implement estimator + LRU + truncation**

- Add byte estimator helpers.
- Add global hard limit `64 * 1024 * 1024`.
- Integrate eviction in store write paths.
- Ensure modal-scoped full-history cache is released on close.

**Step 4: Run tests**

Run: `pnpm --dir desktop test chat-cache-budget --runInBand`  
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/lib/stores/chat-cache-budget.ts desktop/lib/stores/chat-store.ts desktop/lib/stores/chat-cache-budget.test.ts
git commit -m "feat(desktop): enforce 64mb chat cache budget with lru eviction"
```

---

### Task 9: Legacy Cleanup, Verification, and Docs

**Files:**
- Modify: `desktop/app/layout.tsx` (one-time legacy `chat-storage` cleanup if needed)
- Modify: `desktop/README.md` (document storage behavior)
- Modify: `docs/plans/2026-03-02-desktop-sqlite-session-design.md` (implementation notes appendix if needed)

**Step 1: Add cleanup for legacy local persistence key**

- Remove stale `chat-storage` key once during startup migration window.

**Step 2: Run verification suite**

Run: `cargo test -p agent-session -- --nocapture`  
Expected: PASS.

Run: `cargo test -p desktop -- --nocapture`  
Expected: PASS.

Run: `pnpm --dir desktop test --runInBand`  
Expected: PASS.

Run: `pnpm --dir desktop lint`  
Expected: PASS.

Run: `pnpm --dir desktop build`  
Expected: PASS.

**Step 3: Manual QA checklist**

1. Launch desktop, verify first session auto-selected.
2. Verify default list/history only shows last 24h.
3. Open full-info modal and manually request all history.
4. Stress load large histories and confirm no runaway memory growth.
5. Restart app and verify session ordering semantics remain stable.

**Step 4: Commit**

```bash
git add desktop/app/layout.tsx desktop/README.md docs/plans/2026-03-02-desktop-sqlite-session-design.md
git commit -m "docs(desktop): document sqlite-backed session model and migration behavior"
```

---

## Final Delivery Criteria

1. Desktop session list and message history are fully backend-driven from SQLite.
2. Frontend no longer stores full chat history in persistent local storage.
3. Startup behavior: auto-select top session by required ordering semantics.
4. Default window: last 24h; full history only by explicit user action.
5. Runtime cache stays within 64MB budget through deterministic eviction.
