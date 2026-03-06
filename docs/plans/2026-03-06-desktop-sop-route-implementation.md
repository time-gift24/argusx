# Desktop SOP Route Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Move the desktop SOP module from `/annotation` to `/sop/annotation` and add a breadcrumb header that represents the nested route while keeping the left navigation flat.

**Architecture:** Keep the annotation feature implementation where it is, but relocate the route entry and update links to point at the new path. Add breadcrumb and dropdown UI only in the page header so route hierarchy appears in the center panel instead of the sidebar.

**Tech Stack:** Next.js 16, React 19, TypeScript, Vitest, shadcn/ui

---

### Task 1: Lock the new route and navigation behavior with tests

**Files:**
- Create: `desktop/components/layouts/sidebar/app-sidebar.test.tsx`
- Modify: `desktop/app/page.test.tsx`
- Create: `desktop/components/features/annotation/annotation-page.test.tsx`

**Step 1: Write the failing tests**

Add tests that assert:

- the sidebar contains a single SOP item pointing to `/sop/annotation`
- the sidebar does not contain `/annotation` or `/sop/sample-sop`
- the homepage links to `/sop/annotation`
- the annotation page header renders breadcrumb text for `工作台`, `SOP`, and `标注`

**Step 2: Run tests to verify they fail**

Run: `pnpm --dir ./desktop exec vitest run app/page.test.tsx components/layouts/sidebar/app-sidebar.test.tsx components/features/annotation/annotation-page.test.tsx`

Expected: FAIL because the old route and header structure are still present.

**Step 3: Write minimal implementation**

Implement only the route and header changes needed to satisfy the tests.

**Step 4: Re-run tests**

Run: `pnpm --dir ./desktop exec vitest run app/page.test.tsx components/layouts/sidebar/app-sidebar.test.tsx components/features/annotation/annotation-page.test.tsx`

Expected: PASS

### Task 2: Move the route entry and update desktop links

**Files:**
- Delete: `desktop/app/annotation/page.tsx`
- Create: `desktop/app/sop/annotation/page.tsx`
- Modify: `desktop/app/page.tsx`
- Modify: `desktop/components/layouts/sidebar/app-sidebar.tsx`

**Step 1: Run focused typecheck**

Run: `pnpm --dir ./desktop exec tsc --noEmit`

Expected: FAIL or warn until the route file move and link updates are complete.

**Step 2: Write minimal implementation**

- move the route entry to `/sop/annotation`
- update left nav link
- remove the dead SOP sample route link
- update homepage link targets

**Step 3: Re-run typecheck**

Run: `pnpm --dir ./desktop exec tsc --noEmit`

Expected: PASS

### Task 3: Add breadcrumb + ellipsis dropdown to the SOP annotation header

**Files:**
- Modify: `desktop/components/features/annotation/annotation-page.tsx`
- Reuse: `desktop/components/ui/breadcrumb.tsx`
- Reuse: `desktop/components/ui/dropdown-menu.tsx`

**Step 1: Verify failing UI test exists**

Use the annotation-page test from Task 1 as the red state.

**Step 2: Write minimal implementation**

Add a header structure with:

- breadcrumb links for `工作台` and `SOP`
- an ellipsis dropdown trigger in the breadcrumb
- current page marker `标注`
- existing title and supporting copy below

**Step 3: Re-run focused tests**

Run: `pnpm --dir ./desktop exec vitest run components/features/annotation/annotation-page.test.tsx`

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

**Step 4: Optional Tauri verification**

Run: `cargo test --manifest-path desktop/src-tauri/Cargo.toml`

Expected: PASS
