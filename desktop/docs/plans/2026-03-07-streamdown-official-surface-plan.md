# Streamdown Official Surface Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refactor the shared Streamdown integration back onto Streamdown's public APIs and global styling hooks while preserving the compact visual treatment.

**Architecture:** Keep `Reasoning` and the Streamdown playground on direct `<Streamdown>` usage with shared public props: `plugins`, `controls`, `icons`, `translations`, and one root class for global styling. Remove markdown-time detours through custom code and mermaid surfaces, and style official Streamdown blocks globally via `data-streamdown` selectors under the shared root class.

**Tech Stack:** Next.js 16, React, Streamdown, Tailwind v4, Vitest, Testing Library

---

### Task 1: Lock in the public Streamdown surface with failing tests

**Files:**
- Modify: `desktop/components/ai/reasoning.test.tsx`
- Modify: `desktop/app/dev/streamdown/page.test.tsx`

**Step 1: Write the failing test**

Assert that markdown code and mermaid fences render through Streamdown's official `data-streamdown` surfaces and the shared root class, not `runtime-*` surfaces.

**Step 2: Run test to verify it fails**

Run: `pnpm vitest run desktop/components/ai/reasoning.test.tsx desktop/app/dev/streamdown/page.test.tsx`
Expected: FAIL because current code still renders `runtime-code-surface` and `runtime-mermaid-surface`.

**Step 3: Write minimal implementation**

Refactor the shared Streamdown configuration so `Reasoning` and the playground stop routing fenced blocks into custom surfaces.

**Step 4: Run test to verify it passes**

Run: `pnpm vitest run desktop/components/ai/reasoning.test.tsx desktop/app/dev/streamdown/page.test.tsx`
Expected: PASS

### Task 2: Remove non-public mermaid integration

**Files:**
- Modify: `desktop/components/ai/streamdown.ts`
- Modify: `desktop/components/ai/reasoning.tsx`
- Modify: `desktop/app/dev/streamdown/streamdown-playground.tsx`
- Delete: `desktop/components/ai/runtime-mermaid-surface.tsx`
- Delete: `desktop/components/ai/streamdown-components.tsx`
- Delete: `desktop/components/ai/runtime-mermaid-surface.test.tsx`

**Step 1: Write the failing test**

Use the Task 1 tests as the red state and add assertions that official mermaid controls exist via `data-streamdown="mermaid-block-actions"`.

**Step 2: Run test to verify it fails**

Run: `pnpm vitest run desktop/components/ai/reasoning.test.tsx desktop/app/dev/streamdown/page.test.tsx`
Expected: FAIL until deep imports and custom mermaid surfaces are removed.

**Step 3: Write minimal implementation**

Use `plugins.mermaid`, `controls.mermaid`, and `icons` on `<Streamdown>` directly. Keep only shared public config.

**Step 4: Run test to verify it passes**

Run: `pnpm vitest run desktop/components/ai/reasoning.test.tsx desktop/app/dev/streamdown/page.test.tsx`
Expected: PASS

### Task 3: Move markdown styling to one global Streamdown theme root

**Files:**
- Modify: `desktop/components/ai/styles.ts`
- Modify: `desktop/app/globals.css`
- Modify: `desktop/components/ai/reasoning.tsx`
- Modify: `desktop/app/dev/streamdown/streamdown-playground.tsx`

**Step 1: Write the failing test**

Assert the shared Streamdown root class is applied and compact list/code/mermaid rules are expressed via that shared root.

**Step 2: Run test to verify it fails**

Run: `pnpm vitest run desktop/components/ai/reasoning.test.tsx`
Expected: FAIL until the shared root class is in place.

**Step 3: Write minimal implementation**

Replace per-instance markdown utility bundles with one root class and global selectors scoped under that class.

**Step 4: Run test to verify it passes**

Run: `pnpm vitest run desktop/components/ai/reasoning.test.tsx`
Expected: PASS

### Task 4: Regression verification

**Files:**
- Modify if needed: `desktop/components/ai/index.ts`

**Step 1: Run focused tests**

Run: `pnpm vitest run desktop/components/ai/reasoning.test.tsx desktop/components/ai/tool-call-item.test.tsx desktop/app/dev/stream/page.test.tsx desktop/app/dev/streamdown/page.test.tsx`
Expected: PASS

**Step 2: Run build**

Run: `pnpm build`
Expected: PASS

**Step 3: Summarize remaining gaps**

Report any remaining places that still rely on custom runtime surfaces outside Streamdown markdown rendering.
