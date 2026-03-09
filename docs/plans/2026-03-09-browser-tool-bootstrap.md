# Browser Tool Bootstrap Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rebase `feat/browser-tool` onto `origin/main` and make desktop startup dependencies resolve from runtime bootstrap instead of ad hoc default locations.

**Architecture:** Keep `runtime::build_runtime()` as the single bootstrap truth source, then derive desktop startup dependencies from the resulting runtime/config instead of calling `from_default_location()` or `from_current_dir()` inside `DesktopSessionState`. Add regression tests that prove startup wiring uses runtime-derived paths and roots.

**Tech Stack:** Rust, Tauri 2, Tokio, sqlx, rusqlite

---

### Task 1: Add failing bootstrap regression coverage

**Files:**
- Create: `desktop/src-tauri/tests/desktop_bootstrap_test.rs`
- Modify: `desktop/src-tauri/src/lib.rs`
- Modify: `desktop/src-tauri/src/session_commands.rs`

**Step 1: Write the failing test**

Add an integration test that:
- builds a temporary `runtime::ArgusxRuntime` from a temp config
- bootstraps desktop state from that runtime plus a temp workspace root
- saves a provider profile through the bootstrapped state
- asserts the provider settings SQLite file is created under the runtime bootstrap directory, not under platform default directories

Add a second test that:
- bootstraps desktop state with an explicit workspace root
- builds turn dependencies from that state
- verifies the scheduled tool runner can execute a read-only builtin inside that explicit root

**Step 2: Run test to verify it fails**

Run: `cargo test -p desktop --test desktop_bootstrap_test -- --nocapture`
Expected: FAIL because desktop has no reusable bootstrap helper and startup dependencies still use ambient defaults.

### Task 2: Route desktop startup dependencies through bootstrap

**Files:**
- Create: `desktop/src-tauri/src/bootstrap.rs`
- Modify: `desktop/src-tauri/src/lib.rs`
- Modify: `desktop/src-tauri/src/session_commands.rs`
- Modify: `desktop/src-tauri/src/provider_settings/service.rs`
- Modify: `desktop/src-tauri/src/chat/tools.rs`
- Modify: `desktop/src-tauri/src/chat/model.rs`

**Step 1: Add minimal bootstrap helper**

Introduce a desktop bootstrap module that:
- accepts an `ArgusxRuntime`
- derives an app data directory from the runtime config
- derives a provider settings database path from that bootstrap location
- captures the workspace root once
- constructs `DesktopSessionState` from explicit dependencies instead of ambient defaults

**Step 2: Make dependency constructors explicit**

Add explicit constructors for:
- `ProviderSettingsService` from a caller-supplied SQLite path
- `ScheduledToolRunner` from caller-supplied allowed roots
- `ProviderModelRunner` tool definitions from caller-supplied allowed roots where desktop needs them

**Step 3: Update Tauri run path**

Change desktop startup to:
- call `runtime::build_runtime()`
- pass the resulting runtime into the new desktop bootstrap helper
- run Tauri with the bootstrapped session state
- keep shutdown/error handling behavior unchanged

**Step 4: Run targeted tests**

Run: `cargo test -p desktop --test desktop_bootstrap_test -- --nocapture`
Expected: PASS

### Task 3: Verify no regressions in desktop startup behavior

**Files:**
- Modify: `desktop/src-tauri/tests/chat_tools_test.rs`
- Modify: `desktop/src-tauri/tests/provider_settings_runtime_test.rs`

**Step 1: Update existing tests for explicit constructors**

Adjust tests that previously used `from_current_dir()` or `from_provider_settings()` where needed so they use the new explicit desktop bootstrap path.

**Step 2: Run broader verification**

Run: `cargo test -p desktop -- --nocapture`
Expected: PASS

Run: `cargo test -p runtime -- --nocapture`
Expected: PASS

**Step 3: Commit**

```bash
git add docs/plans/2026-03-09-browser-tool-bootstrap.md \
  desktop/src-tauri/src/bootstrap.rs \
  desktop/src-tauri/src/lib.rs \
  desktop/src-tauri/src/session_commands.rs \
  desktop/src-tauri/src/provider_settings/service.rs \
  desktop/src-tauri/src/chat/tools.rs \
  desktop/src-tauri/src/chat/model.rs \
  desktop/src-tauri/tests/desktop_bootstrap_test.rs \
  desktop/src-tauri/tests/chat_tools_test.rs \
  desktop/src-tauri/tests/provider_settings_runtime_test.rs
git commit -m "fix(desktop): route startup dependencies through bootstrap"
```
