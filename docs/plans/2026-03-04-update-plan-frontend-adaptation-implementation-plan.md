# Update Plan Frontend Adaptation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extend `update_plan` to emit backward-compatible rich plan payloads and render them in desktop chat UI with structured sections, five task states, lifecycle status, progress, and optional CTA.

**Architecture:** Keep the existing `tool_call_completed -> output.plan -> chat-store` pipeline. Add optional protocol fields in `agent-tool` (`lifecycle_status`, `progress`, `view`), then parse and normalize in `desktop/lib/stores/chat-store.ts`, and finally render in `turn-process` UI using current compound components (`Plan`, `PlanHeader`, `PlanContent`, `PlanFooter`). Preserve strict fallback behavior when new fields are absent.

**Tech Stack:** Rust (`serde`, `serde_json`, tokio tests), TypeScript/React (Next.js 16, Zustand), Vitest + Testing Library.

---

## Execution Rules

- Reference skills during implementation: `@test-driven-development`, `@verification-before-completion`.
- Keep commits small and scoped per task.
- Prefer targeted test commands first, then broader suite smoke checks.
- Do not remove backward-compatible fields.

---

### Task 0: Isolated Worktree Setup

**Files:**
- No source files. Git workspace setup only.

**Step 1: Create worktree for implementation**

Run:
```bash
git worktree add .worktrees/codex/update-plan-rich-view -b codex/update-plan-rich-view
```
Expected: new worktree directory created and branch checked out.

**Step 2: Enter worktree and verify clean state**

Run:
```bash
cd .worktrees/codex/update-plan-rich-view
git status --short
```
Expected: empty output.

**Step 3: Verify baseline tests before any edits**

Run:
```bash
cargo test -p agent-tool update_plan -- --nocapture
pnpm --dir desktop test -- chat-store-update-plan.test.ts
```
Expected: both pass on baseline.

**Step 4: Commit prep marker (optional)**

Run:
```bash
git commit --allow-empty -m "chore: start update_plan rich view implementation"
```
Expected: empty checkpoint commit.

---

### Task 1: Extend `update_plan` Contract in Rust (TDD)

**Files:**
- Modify: `agent-tool/src/builtin/update_plan.rs`
- Modify: `agent-tool/tests/update_plan_tool_test.rs`
- Modify: `agent-tool/tests/runtime_adapter_test.rs`

**Step 1: Write failing tests for new schema and states**

Add tests in `agent-tool/tests/update_plan_tool_test.rs`:
```rust
#[tokio::test]
async fn update_plan_accepts_blocked_and_failed_statuses() {
    // plan contains blocked/failed; expect success and statuses preserved
}

#[tokio::test]
async fn update_plan_infers_lifecycle_and_progress_when_absent() {
    // no lifecycle/progress input; expect inferred values in output.plan
}

#[tokio::test]
async fn update_plan_rejects_invalid_progress_bounds() {
    // completed > total or percent > 100 => InvalidArgs
}

#[tokio::test]
async fn update_plan_passes_through_optional_view_payload() {
    // output.plan.view.overview/sections/cta should exist when provided
}
```

Also extend `agent-tool/tests/runtime_adapter_test.rs` to assert the `update_plan` schema enum includes `blocked` and `failed`.

**Step 2: Run tests to verify they fail first**

Run:
```bash
cargo test -p agent-tool update_plan -- --nocapture
```
Expected: FAIL with status enum / missing field assertions.

**Step 3: Implement minimal backend support**

In `agent-tool/src/builtin/update_plan.rs`, introduce optional contract structs and validation:
```rust
#[derive(Deserialize)]
struct UpdatePlanArgs {
    #[serde(default)]
    explanation: Option<String>,
    plan: Vec<PlanItem>,
    #[serde(default)]
    lifecycle_status: Option<String>,
    #[serde(default)]
    progress: Option<PlanProgress>,
    #[serde(default)]
    view: Option<PlanView>,
}

fn normalize_task_status(raw: &str) -> Result<&'static str, ToolError> {
    match raw {
        "pending" | "in_progress" | "blocked" | "completed" | "failed" => Ok(raw),
        _ => Err(ToolError::InvalidArgs(format!("invalid status: {raw}"))),
    }
}
```

Emit backward-compatible output with optional enrichments:
```rust
"plan": {
  "title": "Execution Plan",
  "description": payload.explanation,
  "tasks": tasks,
  "is_streaming": is_streaming,
  "lifecycle_status": lifecycle_status,
  "progress": progress,
  "view": payload.view
}
```

Keep existing rule: at most one `in_progress` task.

**Step 4: Re-run tests to green**

Run:
```bash
cargo test -p agent-tool update_plan -- --nocapture
cargo test -p agent-tool runtime_adapter -- --nocapture
```
Expected: PASS.

**Step 5: Commit backend contract change**

Run:
```bash
git add agent-tool/src/builtin/update_plan.rs agent-tool/tests/update_plan_tool_test.rs agent-tool/tests/runtime_adapter_test.rs
git commit -m "feat(agent-tool): extend update_plan with lifecycle progress and view"
```

---

### Task 2: Parse New Plan Fields in `chat-store` (TDD)

**Files:**
- Modify: `desktop/lib/stores/chat-store.ts`
- Modify: `desktop/lib/stores/chat-store-update-plan.test.ts`

**Step 1: Add failing store tests for compatibility + enrichment**

Add cases in `desktop/lib/stores/chat-store-update-plan.test.ts`:
```ts
it("parses lifecycle/progress/view from update_plan output", () => {
  // output.plan has lifecycle_status/progress/view
  // expect turn.plan to include parsed fields
});

it("falls back safely when unknown task status is received", () => {
  // status = "unknown" => normalized to pending
});

it("keeps legacy payload working without new fields", () => {
  // old plan shape still parsed as structured plan
});
```

**Step 2: Run tests to confirm failures**

Run:
```bash
pnpm --dir desktop test -- chat-store-update-plan.test.ts
```
Expected: FAIL on missing plan fields/types.

**Step 3: Implement parser + type extensions**

Update `desktop/lib/stores/chat-store.ts`:
```ts
export interface PlanProgressVM {
  completed: number;
  total: number;
  percent: number;
}

export interface PlanViewSectionVM {
  id: string;
  title: string;
  kind: "bullets" | "text";
  items?: string[];
  content?: string;
}

export interface PlanViewVM {
  overview?: string;
  sections: PlanViewSectionVM[];
  cta?: { label: string; shortcut?: string; action: "submit" | "none" };
}
```

Extend `PlanVM` and normalization:
```ts
status: "pending" | "in_progress" | "blocked" | "completed" | "failed";
```
Unknown status => `pending`.

Parse optional fields in `parseStructuredPlanFromEvent` and keep old-path defaults when absent.

**Step 4: Re-run tests**

Run:
```bash
pnpm --dir desktop test -- chat-store-update-plan.test.ts
```
Expected: PASS.

**Step 5: Commit store parsing changes**

Run:
```bash
git add desktop/lib/stores/chat-store.ts desktop/lib/stores/chat-store-update-plan.test.ts
git commit -m "feat(desktop): parse rich update_plan payload with compatibility fallbacks"
```

---

### Task 3: Update Turn Process View Model (TDD)

**Files:**
- Modify: `desktop/components/features/chat/turn-process-view-model.ts`
- Modify: `desktop/components/features/chat/turn-process-view-model.test.ts`

**Step 1: Add failing view-model tests for progress/lifecycle summary**

Add tests:
```ts
it("uses plan.progress for preview when present", () => {
  // expect preview like "1/3 completed"
});

it("falls back to task-derived progress when progress is absent", () => {
  // expect computed completed/total from tasks
});
```

**Step 2: Run view-model tests**

Run:
```bash
pnpm --dir desktop test -- turn-process-view-model.test.ts
```
Expected: FAIL before implementation.

**Step 3: Implement minimal view-model logic**

In `turn-process-view-model.ts`, update plan section builder:
```ts
const completed = turn.plan.progress?.completed
  ?? turn.plan.tasks.filter((task) => task.status === "completed").length;
const total = turn.plan.progress?.total ?? turn.plan.tasks.length;
const preview = `${completed}/${total} completed`;
```

Use lifecycle for header label if present:
```ts
const headerLabel = turn.plan.isStreaming
  ? "Planning..."
  : turn.plan.lifecycleStatus === "failed"
    ? "Plan Failed"
    : "Plan";
```

**Step 4: Re-run tests**

Run:
```bash
pnpm --dir desktop test -- turn-process-view-model.test.ts
```
Expected: PASS.

**Step 5: Commit view-model update**

Run:
```bash
git add desktop/components/features/chat/turn-process-view-model.ts desktop/components/features/chat/turn-process-view-model.test.ts
git commit -m "feat(desktop): enrich plan section preview with progress and lifecycle"
```

---

### Task 4: Render Rich Plan UI in `TurnProcessSections` (TDD)

**Files:**
- Modify: `desktop/components/features/chat/turn-process-sections.tsx`
- Modify: `desktop/components/features/chat/turn-process-sections.test.tsx`

**Step 1: Add failing UI tests for structured plan display**

Add tests covering:
1. `view.overview` rendered in header/body.
2. `view.sections` renders `bullets` and `text`.
3. five-state task rows render status-specific marker.
4. `view.cta` renders footer button.
5. legacy payload still renders old task list.

Example assertion block:
```tsx
expect(screen.getByText("Key Steps")).toBeInTheDocument();
expect(screen.getByRole("button", { name: /build/i })).toBeInTheDocument();
expect(screen.getByText("blocked", { exact: false })).toBeInTheDocument();
```

**Step 2: Run section tests (expect fail)**

Run:
```bash
pnpm --dir desktop test -- turn-process-sections.test.tsx
```
Expected: FAIL on missing overview/sections/cta render paths.

**Step 3: Implement rendering and graceful fallback**

In `turn-process-sections.tsx`:
- keep existing `<Plan>` structure.
- render status chip + task title + optional description.
- render view sections:
```tsx
{turn.plan.view?.sections.map((section) =>
  section.kind === "bullets" ? <ul>...</ul> : <p>...</p>
)}
```
- render CTA:
```tsx
{turn.plan.view?.cta ? (
  <PlanFooter className="justify-end">
    <PlanAction>
      <Button size="sm">{turn.plan.view.cta.label}</Button>
    </PlanAction>
  </PlanFooter>
) : null}
```
- unknown section kind/action => ignore / no-op.

**Step 4: Re-run UI tests**

Run:
```bash
pnpm --dir desktop test -- turn-process-sections.test.tsx
```
Expected: PASS.

**Step 5: Commit UI rendering changes**

Run:
```bash
git add desktop/components/features/chat/turn-process-sections.tsx desktop/components/features/chat/turn-process-sections.test.tsx
git commit -m "feat(desktop): render rich plan panel with sections status and optional CTA"
```

---

### Task 5: End-to-End Verification and Documentation Sync

**Files:**
- Modify: `docs/plans/2026-03-04-update-plan-frontend-adaptation-design.md` (status/checklist update)

**Step 1: Run backend+frontend focused suites**

Run:
```bash
cargo test -p agent-tool update_plan -- --nocapture
pnpm --dir desktop test -- chat-store-update-plan.test.ts turn-process-view-model.test.ts turn-process-sections.test.tsx
```
Expected: PASS.

**Step 2: Run lightweight repo smoke checks**

Run:
```bash
cargo test -p agent-tool -- --nocapture
pnpm --dir desktop test
```
Expected: PASS or known unrelated failures documented.

**Step 3: Update design doc status section**

Append a short “Implementation Status” block in design doc:
```md
## Implementation Status
- [x] Backend protocol extension
- [x] Store parser compatibility
- [x] Rich plan rendering
- [x] Tests
```

**Step 4: Commit verification/doc sync**

Run:
```bash
git add docs/plans/2026-03-04-update-plan-frontend-adaptation-design.md
git commit -m "docs(plan): mark update_plan adaptation implementation status"
```

**Step 5: Prepare handoff summary for review**

Run:
```bash
git log --oneline -n 8
```
Expected: clear commit sequence matching tasks 1-5.

---

## Done Definition

- `update_plan` supports optional `lifecycle_status`, `progress`, `view` while keeping existing fields.
- `update_plan` accepts task states: `pending|in_progress|blocked|completed|failed`.
- Desktop store parses and normalizes new fields with safe fallback.
- Plan section renders overview/sections/tasks/cta and remains backward compatible.
- Targeted Rust and Vitest suites pass.

