# Desktop Large-Screen Layout Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the desktop app open directly into the chat workspace while rebalancing shell and page widths for large displays.

**Architecture:** Reuse the existing chat placeholder page for both `/` and `/chat`, then adjust the desktop shell width defaults so navigation recedes and the center workspace expands. Keep the route structure simple and apply one targeted layout change to the SOP annotation grid so large displays get a better default review surface.

**Tech Stack:** Next.js 16, React 19, TypeScript, Vitest, Tailwind CSS, shadcn/ui

---

### Task 1: Lock the new root route and sidebar behavior with tests

**Files:**
- Modify: `desktop/app/page.test.tsx`
- Modify: `desktop/components/layouts/sidebar/app-sidebar.test.tsx`
- Modify: `desktop/components/placeholders/chat-module-placeholder.test.tsx`

**Step 1: Write the failing tests**

Add or update tests that assert:

- `/` renders the chat workspace copy instead of dashboard copy
- the left sidebar no longer exposes `仪表板`
- the sidebar still links to `/chat` and `/sop/annotation`
- the chat placeholder page uses the wider desktop layout markers

**Step 2: Run tests to verify they fail**

Run: `pnpm --dir ./desktop exec vitest run app/page.test.tsx components/layouts/sidebar/app-sidebar.test.tsx components/placeholders/chat-module-placeholder.test.tsx`

Expected: FAIL because the root page and sidebar still reflect the previous layout assumptions.

**Step 3: Write minimal implementation**

Update only the root page, sidebar nav items, and chat placeholder structure needed to satisfy the tests.

**Step 4: Re-run tests**

Run: `pnpm --dir ./desktop exec vitest run app/page.test.tsx components/layouts/sidebar/app-sidebar.test.tsx components/placeholders/chat-module-placeholder.test.tsx`

Expected: PASS

### Task 2: Rebalance the desktop shell for large screens

**Files:**
- Modify: `desktop/components/layouts/app-layout.tsx`
- Modify: `desktop/components/ui/sidebar.tsx`

**Step 1: Write the failing test**

If an existing shell test does not cover the desktop width defaults, extend the sidebar test coverage to assert the reduced left-nav emphasis indirectly through rendered labels and structural classes, then verify the old layout assumptions fail.

**Step 2: Write minimal implementation**

- reduce left sidebar default and minimum widths
- keep right sidebar behavior intact
- slightly increase horizontal breathing room in the main inset container

**Step 3: Run focused tests**

Run: `pnpm --dir ./desktop exec vitest run components/layouts/sidebar/app-sidebar.test.tsx`

Expected: PASS

### Task 3: Widen the chat and SOP workspace surfaces

**Files:**
- Modify: `desktop/components/placeholders/chat-module-placeholder.tsx`
- Modify: `desktop/components/features/annotation/annotation-workspace.tsx`

**Step 1: Write the failing test**

Use the placeholder test from Task 1 as the red state for chat width changes, then add or extend a targeted test for the SOP workspace class structure if coverage is missing.

**Step 2: Write minimal implementation**

- change the chat page variant from a centered narrow card to a top-aligned wide workspace card
- widen the SOP right panel column from `360px` to a larger desktop-friendly width

**Step 3: Re-run focused tests**

Run: `pnpm --dir ./desktop exec vitest run components/placeholders/chat-module-placeholder.test.tsx components/features/annotation/annotation-page.test.tsx`

Expected: PASS

### Task 4: Full verification

**Files:**
- Review all modified files above

**Step 1: Run tests**

Run: `pnpm --dir ./desktop test`

Expected: PASS

**Step 2: Run lint**

Run: `pnpm --dir ./desktop lint`

Expected: PASS

**Step 3: Run typecheck**

Run: `pnpm --dir ./desktop exec tsc --noEmit`

Expected: PASS

**Step 4: Run desktop Rust tests**

Run: `cargo test --manifest-path desktop/src-tauri/Cargo.toml`

Expected: PASS
