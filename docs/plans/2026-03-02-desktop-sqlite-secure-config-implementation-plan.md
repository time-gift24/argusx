# Desktop SQLite Session + Secure Runtime Config Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Re-plan desktop chat persistence after recent refactor: move desktop session data to SQLite as the single source of truth, and add persistent model/API-key config storage encrypted/decrypted with a key derived from local machine address fingerprint.

**Architecture:** Keep `SessionRuntime` as orchestration layer but add a SQLite-backed storage path for desktop usage. In desktop Tauri, add a secure runtime-config repository that stores provider/model config in SQLite with encrypted API keys. Encryption key is derived from stable host fingerprint (local machine address + hostname) and app salt, so decryption only works on the same machine identity.

**Tech Stack:** Rust (`agent-session`, `desktop/src-tauri`), SQLite (`rusqlite`), crypto (`aes-gcm`, `hkdf`, `sha2`, `rand`), host fingerprint crate (`mac_address` or equivalent), Tauri v2 commands, Next.js 16 + Zustand, Vitest for new TS tests.

---

## Preconditions

1. Worktree branch: `codex/desktop-sqlite-secure-config`
2. Source design references:
   - `docs/plans/2026-03-02-desktop-sqlite-session-design.md`
   - `docs/plans/2026-03-02-desktop-sqlite-session-implementation-plan.md`
3. Required execution skills during implementation:
   - `@test-driven-development`
   - `@verification-before-completion`
   - `@rust-router`
   - `@m06-error-handling`
   - `@m10-performance`

---

### Task 1: Add Host Fingerprint + Crypto Primitives (TDD-first)

**Files:**
- Modify: `desktop/src-tauri/Cargo.toml`
- Create: `desktop/src-tauri/src/secure_config/mod.rs`
- Create: `desktop/src-tauri/src/secure_config/host_fingerprint.rs`
- Create: `desktop/src-tauri/src/secure_config/crypto.rs`
- Test: `desktop/src-tauri/src/secure_config/host_fingerprint.rs` (`#[cfg(test)]`)
- Test: `desktop/src-tauri/src/secure_config/crypto.rs` (`#[cfg(test)]`)

**Step 1: Write failing fingerprint determinism test**

```rust
#[test]
fn fingerprint_is_stable_for_same_input() {
    let a = derive_fingerprint("00:11:22:33:44:55", "my-host");
    let b = derive_fingerprint("00:11:22:33:44:55", "my-host");
    assert_eq!(a, b);
}
```

**Step 2: Run test to verify fail**

Run: `cargo test -p desktop fingerprint_is_stable_for_same_input -- --nocapture`  
Expected: FAIL with unresolved module/function.

**Step 3: Implement minimal fingerprint helpers**

- Canonicalize local address and hostname.
- Build stable digest string.
- Expose `load_host_fingerprint()` with clear error when machine address unavailable.

**Step 4: Write failing encryption roundtrip test**

```rust
#[test]
fn encrypt_decrypt_roundtrip() {
    let key = derive_key_from_fingerprint("fp");
    let cipher = encrypt_secret(&key, "sk-test").unwrap();
    let plain = decrypt_secret(&key, &cipher).unwrap();
    assert_eq!(plain, "sk-test");
}
```

**Step 5: Run test to verify fail**

Run: `cargo test -p desktop encrypt_decrypt_roundtrip -- --nocapture`  
Expected: FAIL before crypto implementation.

**Step 6: Implement AES-GCM helpers**

- Derive key with HKDF-SHA256 from fingerprint + app salt.
- Encrypt with random nonce.
- Return serializable envelope (`nonce`, `ciphertext`, `algo`, `v`).
- Decrypt and map auth-failure to explicit domain error.

**Step 7: Run both test groups**

Run: `cargo test -p desktop secure_config:: -- --nocapture`  
Expected: PASS.

**Step 8: Commit**

```bash
git add desktop/src-tauri/Cargo.toml desktop/src-tauri/src/secure_config/mod.rs desktop/src-tauri/src/secure_config/host_fingerprint.rs desktop/src-tauri/src/secure_config/crypto.rs
git commit -m "feat(desktop): add host-fingerprint based crypto primitives for secure config"
```

---

### Task 2: Add SQLite Schema for Session + Runtime Config Storage

**Files:**
- Create: `desktop/src-tauri/src/persistence/mod.rs`
- Create: `desktop/src-tauri/src/persistence/schema.rs`
- Test: `desktop/src-tauri/src/persistence/schema.rs` (`#[cfg(test)]`)

**Step 1: Write failing schema bootstrap test**

Test should assert these tables exist after init:
1. `sessions`
2. `turns`
3. `transcript_items`
4. `llm_runtime_config`
5. `llm_provider_configs`

**Step 2: Run test to verify fail**

Run: `cargo test -p desktop sqlite_schema_bootstraps -- --nocapture`  
Expected: FAIL (module missing).

**Step 3: Implement schema bootstrap**

- Add migration runner with `PRAGMA user_version`.
- Add required indexes:
  - session ordering index by `ended_at`/`updated_at`
  - transcript `(turn_id, seq)` uniqueness.
- Add runtime config tables:
  - `llm_runtime_config` (`default_provider`, `updated_at_ms`)
  - `llm_provider_configs` (`provider_id`, `base_url`, `models_json`, `headers_json`, `api_key_cipher_json`, `updated_at_ms`)

**Step 4: Run schema tests**

Run: `cargo test -p desktop sqlite_schema_bootstraps -- --nocapture`  
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/src-tauri/src/persistence/mod.rs desktop/src-tauri/src/persistence/schema.rs
git commit -m "feat(desktop): add sqlite schema bootstrap for chat and secure runtime config"
```

---

### Task 3: Implement SQLite Session Store in `agent-session`

**Files:**
- Modify: `agent-session/Cargo.toml`
- Create: `agent-session/src/sqlite_store.rs`
- Modify: `agent-session/src/storage.rs`
- Modify: `agent-session/src/lib.rs`
- Test: `agent-session/tests/sqlite_store_ordering_test.rs`

**Step 1: Write failing test for session ordering semantics**

Test should assert:
1. `MAX(turns.ended_at_ms)` DESC first
2. fallback to `sessions.updated_at_ms` when `ended_at_ms` null.

**Step 2: Run test to verify fail**

Run: `cargo test -p agent-session sqlite_store_ordering_test -- --nocapture`  
Expected: FAIL with missing sqlite store.

**Step 3: Implement `SqliteSessionStore` CRUD**

- Session CRUD.
- Turn summary save/list.
- Transcript save/load by seq.
- `find_session_id_by_turn_id`.
- `restore_to_turn` equivalent behavior.

**Step 4: Run targeted tests**

Run: `cargo test -p agent-session sqlite_store_ordering_test -- --nocapture`  
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-session/Cargo.toml agent-session/src/sqlite_store.rs agent-session/src/storage.rs agent-session/src/lib.rs agent-session/tests/sqlite_store_ordering_test.rs
git commit -m "feat(agent-session): add sqlite session store with turn-based ordering semantics"
```

---

### Task 4: Inject SQLite Store into `SessionRuntime` (Desktop path)

**Files:**
- Modify: `agent-session/src/session_runtime.rs`
- Test: `agent-session/tests/sqlite_runtime_integration_test.rs`

**Step 1: Write failing integration test**

Scenario:
1. start runtime with sqlite store
2. create session + run a turn
3. recreate runtime from same sqlite db
4. verify session/turn restore works.

**Step 2: Run test to verify fail**

Run: `cargo test -p agent-session sqlite_runtime_integration_test -- --nocapture`  
Expected: FAIL before injectable constructor path.

**Step 3: Implement injectable storage constructor**

- Keep old constructors for compatibility.
- Add new constructor that accepts concrete store for desktop.

**Step 4: Run all `agent-session` tests**

Run: `cargo test -p agent-session -- --nocapture`  
Expected: PASS.

**Step 5: Commit**

```bash
git add agent-session/src/session_runtime.rs agent-session/tests/sqlite_runtime_integration_test.rs
git commit -m "refactor(agent-session): support runtime with injected sqlite store"
```

---

### Task 5: Add Desktop Secure Runtime Config Repository

**Files:**
- Create: `desktop/src-tauri/src/persistence/runtime_config_repo.rs`
- Modify: `desktop/src-tauri/src/lib.rs`
- Modify: `desktop/src-tauri/src/llm_runtime_config.rs`
- Test: `desktop/src-tauri/src/persistence/runtime_config_repo.rs` (`#[cfg(test)]`)
- Test: `desktop/src-tauri/src/lib.rs` (command-level tests)

**Step 1: Write failing repository tests**

Test cases:
1. save config encrypts API key (ciphertext stored, not plaintext)
2. load config decrypts correctly with same host fingerprint
3. load fails with mismatched fingerprint.

**Step 2: Run test to verify fail**

Run: `cargo test -p desktop runtime_config_repo -- --nocapture`  
Expected: FAIL before repository exists.

**Step 3: Implement secure repository**

- Normalize config before save.
- Persist non-secret fields in plain JSON columns.
- Persist API key as encrypted envelope JSON.
- On load, decrypt using current host fingerprint-derived key.
- Return actionable error: "stored config bound to different machine fingerprint".

**Step 4: Wire into Tauri commands**

- `get_llm_runtime_config`: read persisted config; fallback default if absent.
- `set_llm_runtime_config`: validate + save encrypted + apply to runtime client.
- Add new command `clear_llm_runtime_config` to wipe stored credentials.

**Step 5: Run desktop Rust tests**

Run: `cargo test -p desktop -- --nocapture`  
Expected: PASS.

**Step 6: Commit**

```bash
git add desktop/src-tauri/src/persistence/runtime_config_repo.rs desktop/src-tauri/src/lib.rs desktop/src-tauri/src/llm_runtime_config.rs
git commit -m "feat(desktop): persist encrypted runtime model and api-key config in sqlite"
```

---

### Task 6: Replace Mock Chat Session Commands with SQLite-backed Queries

**Files:**
- Create: `desktop/src-tauri/src/persistence/chat_repo.rs`
- Modify: `desktop/src-tauri/src/lib.rs`
- Modify: `desktop/lib/api/chat.ts`
- Test: `desktop/src-tauri/src/persistence/chat_repo.rs` (`#[cfg(test)]`)

**Step 1: Write failing query tests**

Cases:
1. `list_chat_sessions` returns sorted sessions (`ended_at`, fallback `updated_at`).
2. `get_chat_messages` default range = last 24h.
3. `get_chat_messages` supports `all` + cursor pagination.

**Step 2: Run test to verify fail**

Run: `cargo test -p desktop chat_repo_query -- --nocapture`  
Expected: FAIL while commands still mock.

**Step 3: Implement repository + command wiring**

- Replace `list_chat_sessions()` mock empty list.
- Replace `get_chat_messages()` mock empty list.
- Implement real create/delete/update session persistence.
- Add API payload types for range/cursor.

**Step 4: Run command tests**

Run: `cargo test -p desktop chat_repo_query -- --nocapture`  
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/src-tauri/src/persistence/chat_repo.rs desktop/src-tauri/src/lib.rs desktop/lib/api/chat.ts
git commit -m "feat(desktop): back chat session commands with sqlite repository"
```

---

### Task 7: Frontend Runtime Config UX for Persistent Secure Storage

**Files:**
- Modify: `desktop/lib/api/chat.ts`
- Modify: `desktop/lib/stores/llm-runtime-config-store.ts`
- Modify: `desktop/components/features/chat/chat-runtime-config-dialog.tsx`
- Test: `desktop/lib/stores/llm-runtime-config-store.test.ts`
- Test: `desktop/components/features/chat/chat-runtime-config-dialog.test.tsx`
- Modify: `desktop/package.json`
- Create: `desktop/vitest.config.ts`

**Step 1: Write failing frontend tests**

Cases:
1. bootstrap reads persisted config from backend.
2. save writes config and refreshes model list.
3. clear action removes persisted config and resets UI state.

**Step 2: Run tests to verify fail**

Run: `pnpm --dir desktop test llm-runtime-config-store --runInBand`  
Expected: FAIL before clear flow and updated API wrappers.

**Step 3: Implement API/store/dialog changes**

- Add `clearLlmRuntimeConfig()` API wrapper.
- Add `clearConfig` action in config store.
- Add "Clear Stored Credentials" action in runtime config dialog.
- Keep API key input masked in UI and avoid logging value.

**Step 4: Run tests**

Run: `pnpm --dir desktop test llm-runtime-config-store --runInBand`  
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/lib/api/chat.ts desktop/lib/stores/llm-runtime-config-store.ts desktop/components/features/chat/chat-runtime-config-dialog.tsx desktop/package.json desktop/vitest.config.ts
git commit -m "feat(desktop): add persisted secure runtime-config clear and reload flow"
```

---

### Task 8: Frontend Chat Store Rework for 24h Default + 64MB Budget

**Files:**
- Modify: `desktop/lib/stores/chat-store.ts`
- Create: `desktop/lib/stores/chat-cache-budget.ts`
- Modify: `desktop/components/features/chat/chat-page.tsx`
- Create: `desktop/components/features/chat/chat-full-info-dialog.tsx`
- Test: `desktop/lib/stores/chat-store.bootstrap.test.ts`
- Test: `desktop/lib/stores/chat-cache-budget.test.ts`

**Step 1: Write failing bootstrap + cache tests**

Cases:
1. startup auto-selects first session from backend.
2. default load fetches only last 24h.
3. cache overflow over 64MB evicts non-active details first.
4. full-info modal fetches all history only on explicit user action.

**Step 2: Run tests to verify fail**

Run: `pnpm --dir desktop test chat-store --runInBand`  
Expected: FAIL because store still uses `persist(chat-storage)` and lacks budget manager.

**Step 3: Implement store + budget manager**

- Remove authoritative persistent chat history behavior.
- Add bootstrap actions driven by backend query.
- Enforce global 64MB cap with deterministic eviction.
- Add full-info modal entry in chat top-left.

**Step 4: Run tests**

Run: `pnpm --dir desktop test chat-store --runInBand`  
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/lib/stores/chat-store.ts desktop/lib/stores/chat-cache-budget.ts desktop/components/features/chat/chat-page.tsx desktop/components/features/chat/chat-full-info-dialog.tsx
git commit -m "refactor(desktop): use backend 24h chat bootstrap and enforce 64mb cache budget"
```

---

### Task 9: Migration, Security Hardening, and End-to-End Verification

**Files:**
- Modify: `desktop/src-tauri/src/lib.rs`
- Modify: `desktop/README.md`
- Modify: `docs/plans/2026-03-02-desktop-sqlite-session-design.md`
- Create: `docs/plans/2026-03-02-desktop-sqlite-secure-config-qa-checklist.md`

**Step 1: Add migration behavior**

- One-time cleanup of legacy `chat-storage` hydration key.
- Controlled fallback for missing/invalid encrypted runtime config:
  - keep non-secret model/base_url fields if possible
  - require API key re-entry.

**Step 2: Add security checks**

- Ensure API key never appears in logs/errors.
- Ensure decryption errors are categorized and actionable.

**Step 3: Run full verification commands**

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

**Step 4: Manual QA**

1. Save runtime config with API key, restart app, verify config auto-restored.
2. Verify DB contains encrypted API key only.
3. Verify session startup ordering by latest `ended_at`.
4. Verify default timeline is 24h window.
5. Verify full-info modal loads complete history only after explicit click.
6. Stress long history and confirm cache budget control.

**Step 5: Commit**

```bash
git add desktop/src-tauri/src/lib.rs desktop/README.md docs/plans/2026-03-02-desktop-sqlite-session-design.md docs/plans/2026-03-02-desktop-sqlite-secure-config-qa-checklist.md
git commit -m "docs(desktop): add migration and qa checklist for sqlite secure runtime config rollout"
```

---

## Done Criteria

1. Desktop session/chat data is SQLite-backed and no longer mock-driven.
2. Runtime model/provider config persists across restart.
3. API keys are encrypted at rest and decryptable only with local machine fingerprint key.
4. Startup defaults remain:
   - auto-select top session
   - default 24h data load
   - full-history on explicit user action.
5. Frontend cache remains bounded at 64MB with deterministic eviction.
