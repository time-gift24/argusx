# Browser Ensure Debug Port Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a `browser.ensure_debug_port` action that makes Chrome reachable on port `9222` on macOS and Windows, restarting the browser with a best-effort session restore when needed.

**Architecture:** Keep the feature inside the browser builtin. Add a runtime-only session snapshot and a platform-specific restart helper that captures tabs, restarts Chrome with a debug port, waits for CDP, and restores any missing tabs.

**Tech Stack:** Rust, `chromiumoxide`, `tokio`, `std::process`, AppleScript on macOS, PowerShell on Windows, existing browser builtin/tool framework.

---

### Task 1: Add the public browser action contract

**Files:**
- Modify: `/Users/wanyaozhong/projects/argusx/.worktrees/feat-browser-tool/tool/src/builtin/browser/mod.rs`
- Test: `/Users/wanyaozhong/projects/argusx/.worktrees/feat-browser-tool/tool/src/builtin/browser/mod.rs`

**Step 1: Write the failing test**

Add a unit test that deserializes browser action args for:

```rust
json!({
    "action": "ensure_debug_port",
    "port": 9222
})
```

and asserts the action enum accepts it.

**Step 2: Run test to verify it fails**

Run: `cargo test -p tool builtin::browser::tests::browser_action_accepts_ensure_debug_port`

Expected: FAIL because the action enum does not include `ensure_debug_port`.

**Step 3: Write minimal implementation**

- Add `EnsureDebugPort { port: Option<u16>, timeout_ms: Option<u64> }` to the browser action enum.
- Add the new action to the tool schema.
- Add execute dispatch to a stub action method returning a structured placeholder.

**Step 4: Run test to verify it passes**

Run: `cargo test -p tool builtin::browser::tests::browser_action_accepts_ensure_debug_port`

Expected: PASS

### Task 2: Introduce runtime snapshot and restart scaffolding

**Files:**
- Create: `/Users/wanyaozhong/projects/argusx/.worktrees/feat-browser-tool/tool/src/builtin/browser/debug_port.rs`
- Modify: `/Users/wanyaozhong/projects/argusx/.worktrees/feat-browser-tool/tool/src/builtin/browser/mod.rs`
- Test: `/Users/wanyaozhong/projects/argusx/.worktrees/feat-browser-tool/tool/src/builtin/browser/debug_port.rs`

**Step 1: Write the failing test**

Add tests covering:
- probe result reports `already_enabled = true` when `/json/version` is reachable
- restore result shape contains counts and warnings fields

**Step 2: Run test to verify it fails**

Run: `cargo test -p tool browser::debug_port`

Expected: FAIL because the module and types do not exist.

**Step 3: Write minimal implementation**

- Define runtime-only structs:
  - `BrowserSessionSnapshot`
  - `BrowserWindowSnapshot`
  - `BrowserTabSnapshot`
  - `EnsureDebugPortResult`
- Add a probe helper for TCP + `/json/version`.

**Step 4: Run test to verify it passes**

Run: `cargo test -p tool browser::debug_port`

Expected: PASS

### Task 3: Add macOS session capture and relaunch support

**Files:**
- Modify: `/Users/wanyaozhong/projects/argusx/.worktrees/feat-browser-tool/tool/src/builtin/browser/debug_port.rs`
- Test: `/Users/wanyaozhong/projects/argusx/.worktrees/feat-browser-tool/tool/src/builtin/browser/debug_port.rs`

**Step 1: Write the failing test**

Add a macOS-gated test for AppleScript command generation and snapshot parsing.

**Step 2: Run test to verify it fails**

Run: `cargo test -p tool browser::debug_port::tests::macos_capture_command_builds`

Expected: FAIL because no macOS capture helper exists.

**Step 3: Write minimal implementation**

- Build AppleScript to enumerate windows/tabs.
- Add `quit` and relaunch helpers for `Google Chrome`.
- Parse snapshot output into runtime structs.

**Step 4: Run test to verify it passes**

Run: `cargo test -p tool browser::debug_port::tests::macos_capture_command_builds`

Expected: PASS

### Task 4: Add Windows session capture and relaunch support

**Files:**
- Modify: `/Users/wanyaozhong/projects/argusx/.worktrees/feat-browser-tool/tool/src/builtin/browser/debug_port.rs`
- Test: `/Users/wanyaozhong/projects/argusx/.worktrees/feat-browser-tool/tool/src/builtin/browser/debug_port.rs`

**Step 1: Write the failing test**

Add a Windows-gated test for PowerShell command generation.

**Step 2: Run test to verify it fails**

Run: `cargo test -p tool browser::debug_port::tests::windows_capture_command_builds`

Expected: FAIL because no Windows helper exists.

**Step 3: Write minimal implementation**

- Build PowerShell capture and relaunch command helpers.
- Tolerate partial snapshot data.

**Step 4: Run test to verify it passes**

Run: `cargo test -p tool browser::debug_port::tests::windows_capture_command_builds`

Expected: PASS

### Task 5: Wire ensure_debug_port into BrowserTool

**Files:**
- Modify: `/Users/wanyaozhong/projects/argusx/.worktrees/feat-browser-tool/tool/src/builtin/browser/mod.rs`
- Modify: `/Users/wanyaozhong/projects/argusx/.worktrees/feat-browser-tool/tool/src/builtin/browser/chrome.rs`
- Test: `/Users/wanyaozhong/projects/argusx/.worktrees/feat-browser-tool/tool/src/builtin/browser/mod.rs`

**Step 1: Write the failing test**

Add a unit test asserting `ensure_debug_port` returns a payload with:
- `port`
- `already_enabled`
- `restarted`
- `warnings`

**Step 2: Run test to verify it fails**

Run: `cargo test -p tool builtin::browser::tests::ensure_debug_port_returns_structured_result`

Expected: FAIL because the stub does not call the restart/probe flow.

**Step 3: Write minimal implementation**

- Call the new platform helper from `action_ensure_debug_port`.
- Reuse existing config/chrome path where possible.
- If already enabled, short-circuit without restart.

**Step 4: Run test to verify it passes**

Run: `cargo test -p tool builtin::browser::tests::ensure_debug_port_returns_structured_result`

Expected: PASS

### Task 6: Add restore of missing tabs after restart

**Files:**
- Modify: `/Users/wanyaozhong/projects/argusx/.worktrees/feat-browser-tool/tool/src/builtin/browser/debug_port.rs`
- Test: `/Users/wanyaozhong/projects/argusx/.worktrees/feat-browser-tool/tool/src/builtin/browser/debug_port.rs`

**Step 1: Write the failing test**

Add a test that compares a saved snapshot against currently restored targets and identifies missing tabs that need reopening.

**Step 2: Run test to verify it fails**

Run: `cargo test -p tool browser::debug_port::tests::restore_plan_reopens_missing_tabs_only`

Expected: FAIL because no restore planner exists.

**Step 3: Write minimal implementation**

- Build a restore planner from saved snapshot to current targets.
- Skip tabs with no recoverable URL.
- Preserve order as much as practical.

**Step 4: Run test to verify it passes**

Run: `cargo test -p tool browser::debug_port::tests::restore_plan_reopens_missing_tabs_only`

Expected: PASS

### Task 7: Verify end-to-end browser behavior manually

**Files:**
- Modify: `/Users/wanyaozhong/projects/argusx/.worktrees/feat-browser-tool/tool/src/bin/browser_debug.rs`

**Step 1: Add minimal debug entry point support if still needed**

Keep the temporary CLI usable for manual verification of:
- connect
- launch
- ensure debug port diagnostics

**Step 2: Run manual verification**

Run:

```bash
cargo test -p tool
cargo run -p tool --bin browser_debug -- connect
```

On macOS and Windows, manually verify:
- existing Chrome with tabs open
- `ensure_debug_port` restarts with `9222`
- tabs mostly restore
- GitHub navigation and cookie read still work

**Step 3: Commit**

```bash
git add tool/src/builtin/browser tool/src/bin/browser_debug.rs docs/plans/2026-03-09-browser-ensure-debug-port-*.md tool/Cargo.toml Cargo.lock
git commit -m "feat: ensure browser debug port with session restore"
```
