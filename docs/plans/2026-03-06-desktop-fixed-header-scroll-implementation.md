# Desktop Fixed Header Scroll Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Keep the desktop header fixed, move page breadcrumb into the global header, and constrain vertical scrolling to the main content region.

**Architecture:** Update `AppLayout` so it computes route-aware breadcrumb UI and wraps page content in a dedicated scroll container under the fixed header. Remove the SOP breadcrumb from `AnnotationPage` so page components only provide local title and content while the shell owns route context.

**Tech Stack:** Next.js 16, React 19, TypeScript, Vitest, Tailwind CSS, shadcn/ui

---

### Task 1: Lock the new shell behavior with tests

**Files:**
- Modify: `desktop/components/layouts/app-layout.test.tsx`
- Modify: `desktop/components/features/annotation/annotation-page.test.tsx`

**Step 1: Write the failing tests**

Add tests that assert:

- `/sop/annotation` renders breadcrumb in the global header
- `/` does not render the breadcrumb row
- the main content region under the header uses a dedicated scroll container class
- `AnnotationPage` no longer renders `工作台` or `SOP` breadcrumb text inside the page body

**Step 2: Run tests to verify they fail**

Run: `pnpm --dir ./desktop exec vitest run components/layouts/app-layout.test.tsx components/features/annotation/annotation-page.test.tsx`

Expected: FAIL because breadcrumb is still rendered in the page body and the main content area is not yet an explicit scroll container.

**Step 3: Write minimal implementation**

Implement only the shell and annotation-page changes required to satisfy the tests.

**Step 4: Re-run tests**

Run: `pnpm --dir ./desktop exec vitest run components/layouts/app-layout.test.tsx components/features/annotation/annotation-page.test.tsx`

Expected: PASS

### Task 2: Move route breadcrumb into the fixed header

**Files:**
- Modify: `desktop/components/layouts/app-layout.tsx`
- Modify: `desktop/components/features/annotation/annotation-page.tsx`

**Step 1: Use the failing tests from Task 1**

Do not add production code before the tests above fail correctly.

**Step 2: Write minimal implementation**

- add a route-aware breadcrumb row in the global header
- keep `/` and `/chat` single-line
- move the SOP ellipsis dropdown into the header breadcrumb row
- remove breadcrumb UI from `AnnotationPage`

**Step 3: Re-run focused tests**

Run: `pnpm --dir ./desktop exec vitest run components/layouts/app-layout.test.tsx components/features/annotation/annotation-page.test.tsx`

Expected: PASS

### Task 3: Make only the main content region scroll

**Files:**
- Modify: `desktop/components/layouts/app-layout.tsx`

**Step 1: Keep the shell test red state**

Use the scroll-container expectation from Task 1.

**Step 2: Write minimal implementation**

- wrap children in a fixed-height, `min-h-0`, `overflow-y-auto` content region below the header
- preserve current spacing while preventing the page body from expanding outside the viewport

**Step 3: Re-run focused tests**

Run: `pnpm --dir ./desktop exec vitest run components/layouts/app-layout.test.tsx`

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
