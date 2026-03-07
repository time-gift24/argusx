# Streamdown Code Surface Refresh Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rebuild Streamdown fenced code blocks so they keep the official `<Streamdown>` entrypoint while using a compact stream-item header, collapsed preview, floating actions, and a muted background.

**Architecture:** Keep Mermaid on the official Streamdown surface. Replace only regular fenced code via the public `components.code` hook, reusing the exported `CodeBlock`, `CodeBlockCopyButton`, `CodeBlockDownloadButton`, and `useIsCodeFenceIncomplete()` APIs. Drive layout and theme changes through the global `.ai-streamdown` stylesheet.

**Tech Stack:** Next.js 16, React 19, Streamdown, Vitest, Testing Library, Tailwind v4, global CSS overrides.

---

### Task 1: Lock behavior with failing tests

**Files:**
- Modify: `desktop/components/ai/reasoning.test.tsx`
- Modify: `desktop/app/dev/streamdown/page.test.tsx`

**Step 1: Write the failing test**

- Assert fenced code renders a custom stream-item shell with:
  - collapsed viewport mounted by default
  - `Running` status and shimmer for incomplete fences
  - floating copy/download actions inside the code content
  - no runtime ai-element slot
- Assert global CSS includes:
  - light/dark muted code background
  - collapsed 3-line and expanded 20-line max heights

**Step 2: Run test to verify it fails**

Run: `pnpm vitest run components/ai/reasoning.test.tsx app/dev/streamdown/page.test.tsx`

Expected: FAIL because the current official surface does not render the custom shell or max-height rules.

### Task 2: Add a reusable stream-item viewport primitive

**Files:**
- Modify: `desktop/components/ai/stream-item.tsx`

**Step 1: Write the failing test**

- Cover the mounted viewport state through the reasoning integration test rather than a separate unit test.

**Step 2: Write minimal implementation**

- Export a small always-mounted viewport component that exposes `data-state="open|closed"` from `StreamItem` context without affecting existing `StreamItemContent`.

**Step 3: Run focused tests**

Run: `pnpm vitest run components/ai/reasoning.test.tsx`

Expected: still failing only on code-surface behavior.

### Task 3: Build the custom fenced-code surface on public Streamdown APIs

**Files:**
- Create: `desktop/components/ai/streamdown-code.tsx`
- Modify: `desktop/components/ai/streamdown.ts`
- Modify: `desktop/components/ai/reasoning.tsx`
- Modify: `desktop/app/dev/streamdown/streamdown-playground.tsx`

**Step 1: Write minimal implementation**

- Add a custom `components.code` renderer.
- Keep inline code simple.
- For `mermaid`, recurse into a nested official `Streamdown` instance without the custom components override.
- For normal fenced code, render:
  - `StreamItem` header with language label and `Running`/`Ready` status
  - always-mounted viewport tied to open/closed state
  - official `CodeBlock` for highlighted body
  - official copy/download buttons in an absolute overlay

**Step 2: Run focused tests**

Run: `pnpm vitest run components/ai/reasoning.test.tsx app/dev/streamdown/page.test.tsx`

Expected: PASS.

### Task 4: Finish the global styling

**Files:**
- Modify: `desktop/app/globals.css`

**Step 1: Write minimal implementation**

- Hide the inner official code header only inside the custom shell.
- Apply a subtle grey background with different light/dark mixes.
- Set collapsed body max-height to 3 lines and expanded max-height to 20 lines.
- Keep internal scrolling enabled.
- Place action buttons directly over the code body at top-right.

**Step 2: Run full targeted verification**

Run: `pnpm vitest run components/ai/reasoning.test.tsx components/ai/tool-call-item.test.tsx app/dev/stream/page.test.tsx app/dev/streamdown/page.test.tsx`

Expected: PASS.

### Task 5: Final verification and commit

**Files:**
- Modify: `desktop/docs/plans/2026-03-07-streamdown-code-surface-refresh.md`

**Step 1: Run build**

Run: `pnpm build`

Expected: PASS with `/dev/streamdown` generated.

**Step 2: Commit**

```bash
git add desktop/docs/plans/2026-03-07-streamdown-code-surface-refresh.md \
  desktop/components/ai/stream-item.tsx \
  desktop/components/ai/streamdown-code.tsx \
  desktop/components/ai/streamdown.ts \
  desktop/components/ai/reasoning.tsx \
  desktop/app/dev/streamdown/streamdown-playground.tsx \
  desktop/app/globals.css \
  desktop/components/ai/reasoning.test.tsx \
  desktop/app/dev/streamdown/page.test.tsx
git commit -m "feat: restyle streamdown code surfaces"
```
