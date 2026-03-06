# Desktop Window Size Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Increase the default Tauri desktop window size so the app opens appropriately on large monitors.

**Architecture:** Lock the window defaults with a lightweight regression test that reads `tauri.conf.json`, then update the Tauri window definition to the approved dimensions and positioning. Keep the change isolated to configuration so frontend layout behavior remains untouched.

**Tech Stack:** Tauri 2, JSON configuration, Vitest

---

### Task 1: Lock the desired window defaults with a failing test

**Files:**
- Create: `desktop/src-tauri/tauri-config.test.ts`
- Modify: `desktop/src-tauri/tauri.conf.json`

**Step 1: Write the failing test**

Add a test that reads `desktop/src-tauri/tauri.conf.json` and asserts:

- `width` is `1600`
- `height` is `1000`
- `minWidth` is `1440`
- `minHeight` is `900`
- `center` is `true`

**Step 2: Run test to verify it fails**

Run: `pnpm --dir ./desktop exec vitest run src-tauri/tauri-config.test.ts`

Expected: FAIL because the current config still uses `800x600` and does not set the new minimum size or centering.

**Step 3: Write minimal implementation**

Update only the window configuration keys needed to satisfy the test.

**Step 4: Re-run the focused test**

Run: `pnpm --dir ./desktop exec vitest run src-tauri/tauri-config.test.ts`

Expected: PASS

### Task 2: Full verification

**Files:**
- Review `desktop/src-tauri/tauri.conf.json`
- Review `desktop/src-tauri/tauri-config.test.ts`

**Step 1: Run desktop tests**

Run: `pnpm --dir ./desktop test`

Expected: PASS

**Step 2: Run lint**

Run: `pnpm --dir ./desktop lint`

Expected: PASS

**Step 3: Run typecheck**

Run: `pnpm --dir ./desktop exec tsc --noEmit`

Expected: PASS

**Step 4: Run Tauri Rust tests**

Run: `cargo test --manifest-path desktop/src-tauri/Cargo.toml`

Expected: PASS
