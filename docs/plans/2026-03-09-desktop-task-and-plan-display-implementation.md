# Desktop Task And Plan Display Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Update the desktop chat UI so `update_plan` renders as a floating running-turn card above the composer, while builtin read tools render as a compact collapsed `Summary` task list inside assistant turns.

**Architecture:** Keep the existing Rust/Tauri event protocol and turn hydration unchanged. Implement the behavior entirely in the desktop React layer by deriving a floating plan selector from running turns, introducing AI Elements `Task`-based read-tool grouping, and leaving non-read tools on the existing `ToolCallItem` path.

**Tech Stack:** Next.js 16, React 19, TypeScript, AI Elements, Vitest, Testing Library, Tauri desktop frontend

---

### Task 1: Lock the new page behavior with failing tests

**Files:**
- Modify: `desktop/app/chat/page.test.tsx`

**Step 1: Write the failing test for floating plan placement**

Add a test that emits `plan-updated` for a running turn and asserts:

- the plan renders inside the composer shell area
- the plan does not render inside `[data-slot="chat-turn-assistant"]`

**Step 2: Run test to verify it fails**

Run: `pnpm --dir desktop vitest run app/chat/page.test.tsx`
Expected: FAIL because `PlanQueue` still renders inline in the assistant turn.

**Step 3: Write the failing test for plan teardown**

In the same file, add assertions that after `turn-finished`, the floating plan is no longer present.

**Step 4: Run test to verify it fails**

Run: `pnpm --dir desktop vitest run app/chat/page.test.tsx`
Expected: FAIL because the current UI has no floating plan lifecycle.

**Step 5: Write the failing test for read-task grouping**

Add a test that emits `read`, `glob`, `grep`, and one non-read tool call, then asserts:

- assistant content contains a collapsed `Summary` section
- the section contains only the latest 3 read-tool items
- the non-read tool still renders outside the `Summary` group

**Step 6: Run test to verify it fails**

Run: `pnpm --dir desktop vitest run app/chat/page.test.tsx`
Expected: FAIL because read tools still render through individual `ToolCallItem` rows.

**Step 7: Commit the red state**

```bash
git add desktop/app/chat/page.test.tsx
git commit -m "test: lock desktop plan and read task display"
```

### Task 2: Add AI Elements Task primitives and a read-task group component

**Files:**
- Create: `desktop/components/ai-elements/task.tsx`
- Create: `desktop/components/ai/read-task-group.tsx`
- Create: `desktop/components/ai/read-task-group.test.tsx`
- Modify: `desktop/components/ai/index.ts`
- Modify if generated: `desktop/package.json`
- Modify if generated: `desktop/components.json`
- Modify if generated: `desktop/pnpm-lock.yaml`

**Step 1: Install the Task component scaffold**

Run:

```bash
cd /Users/wanyaozhong/projects/argusx/desktop
npx ai-elements@latest add task
```

Expected: the AI Elements `Task` component files are added under `desktop/components/ai-elements` and any generated dependency metadata is updated.

**Step 2: Write the failing component test**

In `desktop/components/ai/read-task-group.test.tsx`, render a small set of read-task items and assert:

- the header text is `Summary`
- the group is collapsed by default
- expanding reveals compact task rows
- only provided item summaries appear

**Step 3: Run test to verify it fails**

Run: `pnpm --dir desktop vitest run components/ai/read-task-group.test.tsx`
Expected: FAIL because `ReadTaskGroup` does not exist yet.

**Step 4: Write the minimal component implementation**

Create `ReadTaskGroup` on top of the generated `Task` primitives with:

- a narrow `ReadTaskItem` input type
- a collapsed-by-default trigger labeled `Summary`
- compact spacing
- one item row per read tool

**Step 5: Re-export the new component**

Update `desktop/components/ai/index.ts` to export `ReadTaskGroup`.

**Step 6: Run test to verify it passes**

Run: `pnpm --dir desktop vitest run components/ai/read-task-group.test.tsx`
Expected: PASS

**Step 7: Commit**

```bash
git add desktop/components/ai-elements/task.tsx \
  desktop/components/ai/read-task-group.tsx \
  desktop/components/ai/read-task-group.test.tsx \
  desktop/components/ai/index.ts \
  desktop/package.json \
  desktop/components.json \
  desktop/pnpm-lock.yaml
git commit -m "feat: add read task group for desktop chat"
```

### Task 3: Move plan rendering into a floating composer card

**Files:**
- Create: `desktop/components/ai/floating-plan-card.tsx`
- Create: `desktop/components/ai/floating-plan-card.test.tsx`
- Modify: `desktop/components/ai/plan-queue.tsx`
- Modify: `desktop/components/ai/index.ts`
- Modify: `desktop/app/chat/page.tsx`

**Step 1: Write the failing component test**

In `desktop/components/ai/floating-plan-card.test.tsx`, render a plan snapshot and assert:

- the card wraps `PlanQueue`
- the card uses a compact floating container
- no assistant transcript-specific wrapper is required

**Step 2: Run test to verify it fails**

Run: `pnpm --dir desktop vitest run components/ai/floating-plan-card.test.tsx`
Expected: FAIL because `FloatingPlanCard` does not exist yet.

**Step 3: Write the minimal floating plan component**

Create `FloatingPlanCard` as a presentational wrapper around `PlanQueue`.

**Step 4: Update `PlanQueue` only if needed for compact floating use**

Keep behavior intact and only make the smallest styling or prop adjustments required by the floating container.

**Step 5: Wire the floating card into the composer shell**

In `desktop/app/chat/page.tsx`:

- derive the active floating plan from the latest running turn with `latestPlan`
- render `FloatingPlanCard` above `PromptComposer`
- remove inline `PlanQueue` rendering from assistant turns

**Step 6: Run the page test file**

Run: `pnpm --dir desktop vitest run app/chat/page.test.tsx components/ai/floating-plan-card.test.tsx`
Expected: PASS for floating plan placement and teardown assertions.

**Step 7: Commit**

```bash
git add desktop/components/ai/floating-plan-card.tsx \
  desktop/components/ai/floating-plan-card.test.tsx \
  desktop/components/ai/plan-queue.tsx \
  desktop/components/ai/index.ts \
  desktop/app/chat/page.tsx
git commit -m "feat: float update plan above desktop composer"
```

### Task 4: Route read tools through the Summary task group

**Files:**
- Modify: `desktop/app/chat/page.tsx`
- Modify: `desktop/components/ai/tool-call-item.tsx`
- Modify if needed: `desktop/components/ai/plan-queue.tsx`

**Step 1: Write the minimal selectors**

In `desktop/app/chat/page.tsx`, add small helpers to:

- classify `read`, `glob`, and `grep` as read tools
- clip each turn's read-tool list to the latest 3 items
- leave `update_plan` and non-read tools on separate paths

**Step 2: Render `ReadTaskGroup` inside assistant turns**

Place the `Summary` group after assistant markdown and reasoning, but before non-read tool rows.

**Step 3: Keep non-read tools on the existing path**

Ensure generic `ToolCallItem` rendering still excludes `update_plan` and now also excludes read tools.

**Step 4: Run the page test file**

Run: `pnpm --dir desktop vitest run app/chat/page.test.tsx`
Expected: PASS for read-tool grouping, item clipping, and non-read-tool regression assertions.

**Step 5: Commit**

```bash
git add desktop/app/chat/page.tsx desktop/components/ai/tool-call-item.tsx desktop/components/ai/plan-queue.tsx
git commit -m "feat: group read tools into summary tasks"
```

### Task 5: Verify hydration and component regressions

**Files:**
- Modify if needed: `desktop/app/chat/page.test.tsx`
- Modify if needed: `desktop/components/ai/read-task-group.test.tsx`
- Modify if needed: `desktop/components/ai/floating-plan-card.test.tsx`

**Step 1: Add hydration-focused assertions**

Extend `desktop/app/chat/page.test.tsx` to cover hydrated turns that contain read tool calls and confirm the `Summary` group appears for historical turns.

**Step 2: Run focused tests**

Run: `pnpm --dir desktop vitest run app/chat/page.test.tsx components/ai/read-task-group.test.tsx components/ai/floating-plan-card.test.tsx components/ai/tool-call-item.test.tsx`
Expected: PASS

**Step 3: Run the broader desktop AI test slice**

Run: `pnpm --dir desktop vitest run app/chat/page.test.tsx components/ai/*.test.tsx`
Expected: PASS

**Step 4: Run the desktop build**

Run: `pnpm --dir desktop build`
Expected: PASS

**Step 5: Commit any final test adjustments**

```bash
git add desktop/app/chat/page.test.tsx \
  desktop/components/ai/read-task-group.test.tsx \
  desktop/components/ai/floating-plan-card.test.tsx
git commit -m "test: cover desktop floating plan and read task regressions"
```

### Task 6: Final review and handoff

**Files:**
- Modify: `docs/plans/2026-03-09-desktop-task-and-plan-display-implementation.md`

**Step 1: Re-run the final verification commands**

Run:

```bash
pnpm --dir desktop vitest run app/chat/page.test.tsx components/ai/read-task-group.test.tsx components/ai/floating-plan-card.test.tsx components/ai/tool-call-item.test.tsx
pnpm --dir desktop build
```

Expected: PASS

**Step 2: Inspect the diff**

Run: `git status --short`  
Expected: only the intended desktop UI and test files are modified.

**Step 3: Update this plan with any deviations**

Record any file-path or generated-file differences from the `npx ai-elements@latest add task` step.

**Step 4: Prepare merge-ready summary**

Summarize:

- floating plan behavior
- read-tool `Summary` behavior
- generated AI Elements files and dependencies
- verification commands executed
