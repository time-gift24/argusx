# Streamdown Default Runtime Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove all runtime Streamdown custom styling/plugin wiring in `desktop` so current surfaces render with default Streamdown behavior while preserving the old customization layer as deprecated reference code.

**Architecture:** Update each runtime Streamdown callsite to the minimal default API, then mark the shared customization modules and CSS block as deprecated instead of deleting them. Refresh tests so they assert default runtime usage rather than custom code/mermaid surfaces.

**Tech Stack:** Next.js App Router, React, Vitest, Streamdown, Tailwind CSS

---

### Task 1: Deprecate the shared Streamdown customization layer

**Files:**
- Modify: `desktop/components/ai/streamdown-config.ts`
- Modify: `desktop/components/ai/streamdown-code.tsx`
- Modify: `desktop/app/globals.css`

**Step 1: Add deprecation markers to the shared config module**

Add file-level comments and export-level JSDoc `@deprecated` notes explaining that runtime Streamdown now uses the default renderer and these exports remain only for historical reference.

**Step 2: Add deprecation markers to the custom code component module**

Add a file-level comment and `@deprecated` note for `StreamdownCode`, `sharedStreamdownComponents`, and any helper exports still exposed for the old path.

**Step 3: Mark the `.ai-streamdown` CSS block as deprecated**

Add a comment above the `.ai-streamdown` rules in `desktop/app/globals.css` stating the block is preserved for historical reference and is no longer used by runtime callsites.

**Step 4: Commit**

```bash
git add desktop/components/ai/streamdown-config.ts desktop/components/ai/streamdown-code.tsx desktop/app/globals.css
git commit -m "docs: deprecate custom streamdown runtime layer"
```

### Task 2: Replace runtime Streamdown callsites with default usage

**Files:**
- Modify: `desktop/app/chat/page.tsx`
- Modify: `desktop/components/ai/reasoning.tsx`
- Modify: `desktop/components/ai-elements/reasoning.tsx`
- Modify: `desktop/app/dev/streamdown/streamdown-playground.tsx`

**Step 1: Write or update failing tests**

Update the existing tests that currently assert custom class names or custom code surfaces so they instead verify the pages/components still render Streamdown output without relying on custom wrappers.

**Step 2: Run targeted tests to confirm they fail for the right reason**

Run:

```bash
pnpm --dir desktop exec vitest run components/ai/reasoning.test.tsx app/chat/page.test.tsx app/dev/streamdown/page.test.tsx
```

Expected: failures because the tests still describe the old custom Streamdown contract.

**Step 3: Remove shared custom props from runtime callsites**

At every runtime callsite:

- delete imports from `@/components/ai/streamdown`
- keep only `children`
- keep `isAnimating` only where streaming state matters
- do not pass `className`, `plugins`, `components`, `controls`, `icons`, `translations`, or `shikiTheme`

**Step 4: Run the same targeted tests and make them pass**

Run:

```bash
pnpm --dir desktop exec vitest run components/ai/reasoning.test.tsx app/chat/page.test.tsx app/dev/streamdown/page.test.tsx
```

Expected: PASS

**Step 5: Commit**

```bash
git add desktop/app/chat/page.tsx desktop/components/ai/reasoning.tsx desktop/components/ai-elements/reasoning.tsx desktop/app/dev/streamdown/streamdown-playground.tsx components/ai/reasoning.test.tsx app/chat/page.test.tsx app/dev/streamdown/page.test.tsx
git commit -m "refactor: use default streamdown runtime surfaces"
```

### Task 3: Update tests that encode the old custom Streamdown contract

**Files:**
- Modify: `desktop/components/ai/reasoning.test.tsx`
- Modify: `desktop/components/ai/tool-call-item.test.tsx`
- Modify: `desktop/components/ai/stream-item.test.tsx`
- Modify: `desktop/app/dev/streamdown/page.test.tsx`
- Inspect: `desktop/lib/chat.test.ts`

**Step 1: Remove assertions tied to the custom code/mermaid shell**

Delete or replace assertions that require:

- `.ai-streamdown`
- `custom-code-panel`
- custom code block actions layout
- custom mermaid wrappers
- old compact CSS contract

**Step 2: Keep only behavior-level assertions**

Retain tests that assert:

- markdown text renders
- reasoning/tool items still open and close
- chat page still streams text into the assistant area
- dev playground still mounts and shows Streamdown content

**Step 3: Run focused tests**

Run:

```bash
pnpm --dir desktop exec vitest run components/ai/reasoning.test.tsx components/ai/tool-call-item.test.tsx components/ai/stream-item.test.tsx app/dev/streamdown/page.test.tsx lib/chat.test.ts
```

Expected: PASS

**Step 4: Commit**

```bash
git add desktop/components/ai/reasoning.test.tsx desktop/components/ai/tool-call-item.test.tsx desktop/components/ai/stream-item.test.tsx desktop/app/dev/streamdown/page.test.tsx
git commit -m "test: align streamdown coverage with default runtime"
```

### Task 4: Final verification

**Files:**
- Inspect: `desktop/app/chat/page.tsx`
- Inspect: `desktop/components/ai/reasoning.tsx`
- Inspect: `desktop/components/ai-elements/reasoning.tsx`
- Inspect: `desktop/app/dev/streamdown/streamdown-playground.tsx`

**Step 1: Run front-end verification**

```bash
pnpm --dir desktop exec vitest run app/chat/page.test.tsx components/layouts/app-layout.test.tsx components/ai/stream-item.test.tsx components/ai/reasoning.test.tsx components/ai/tool-call-item.test.tsx app/dev/streamdown/page.test.tsx
```

Expected: PASS

**Step 2: Run workspace sanity checks**

```bash
cargo check -p desktop
git diff --check
```

Expected: both commands succeed with exit code 0

**Step 3: Commit**

```bash
git add -A
git commit -m "chore: verify default streamdown runtime rollout"
```
