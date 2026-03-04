# Update Plan + Todo Queue Frontend Adaptation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Keep `update_plan` backward compatible while upgrading the UI to a dual-layer process view: `Plan` for planning results and `Queue` for TODO execution steps (single-list, five-state).

**Architecture:** Extend `agent-tool` `update_plan` with optional rich fields (`lifecycle_status`, `progress`, `view`, `queue.todos`). In desktop, parse these fields into separated view models (`plan` + `todoQueue`) while preserving existing tool queue state. Render `Plan` and `Queue` as distinct process sections with graceful fallback from `plan.tasks` when `queue.todos` is missing.

**Tech Stack:** Rust (`serde`, `serde_json`, tokio tests), TypeScript/React (Next.js 16, Zustand), Vitest + Testing Library.

---

## Execution Rules

- Reference skills during implementation: `@test-driven-development`, `@verification-before-completion`.
- Keep commits small and task-scoped.
- Prefer narrow tests first, then broader smoke runs.
- Do not break old `update_plan` payload support.

---

### Task 0: Isolated Worktree Setup

**Files:**
- No source files. Git workspace only.

**Step 1: Create isolated worktree**

Run:
```bash
git worktree add .worktrees/codex/update-plan-todo-queue -b codex/update-plan-todo-queue
```
Expected: worktree directory created and branch checked out.

**Step 2: Enter worktree and verify clean state**

Run:
```bash
cd .worktrees/codex/update-plan-todo-queue
git status --short
```
Expected: no output.

**Step 3: Baseline targeted tests**

Run:
```bash
cargo test -p agent-tool update_plan -- --nocapture
pnpm --dir desktop test -- chat-store-update-plan.test.ts turn-process-sections.test.tsx
```
Expected: PASS on baseline.

**Step 4: Optional checkpoint commit**

Run:
```bash
git commit --allow-empty -m "chore: start plan+todo-queue adaptation"
```
Expected: empty checkpoint commit.

---

### Task 1: Extend `update_plan` backend contract (TDD)

**Files:**
- Modify: `agent-tool/src/builtin/update_plan.rs`
- Modify: `agent-tool/tests/update_plan_tool_test.rs`
- Modify: `agent-tool/tests/runtime_adapter_test.rs`

**Step 1: Add failing backend tests**

Add tests in `agent-tool/tests/update_plan_tool_test.rs`:
```rust
#[tokio::test]
async fn update_plan_accepts_five_task_statuses() {
    // pending/in_progress/blocked/completed/failed
}

#[tokio::test]
async fn update_plan_accepts_optional_queue_todos() {
    // output.plan.queue.todos should be present when provided
}

#[tokio::test]
async fn update_plan_rejects_invalid_queue_todo_status() {
    // unknown todo status => InvalidArgs
}

#[tokio::test]
async fn update_plan_infers_progress_and_lifecycle_when_absent() {
    // lifecycle/progress inferred from tasks
}
```

In `runtime_adapter_test.rs`, assert schema enum includes `blocked` and `failed` for relevant statuses.

**Step 2: Run tests and confirm failures**

Run:
```bash
cargo test -p agent-tool update_plan -- --nocapture
```
Expected: FAIL before implementation.

**Step 3: Implement minimal backend support**

In `agent-tool/src/builtin/update_plan.rs`, extend args and output:
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
    #[serde(default)]
    queue: Option<PlanQueue>,
}
```

Validate `queue.todos[].status` with the same five-state set as tasks.

Emit compatible output:
```rust
"plan": {
  "title": "Execution Plan",
  "description": payload.explanation,
  "tasks": tasks,
  "is_streaming": is_streaming,
  "lifecycle_status": lifecycle_status,
  "progress": progress,
  "view": payload.view,
  "queue": payload.queue
}
```

**Step 4: Re-run backend tests**

Run:
```bash
cargo test -p agent-tool update_plan -- --nocapture
cargo test -p agent-tool runtime_adapter -- --nocapture
```
Expected: PASS.

**Step 5: Commit backend changes**

Run:
```bash
git add agent-tool/src/builtin/update_plan.rs agent-tool/tests/update_plan_tool_test.rs agent-tool/tests/runtime_adapter_test.rs
git commit -m "feat(agent-tool): add optional queue.todos to update_plan payload"
```

---

### Task 2: Parse `plan + todoQueue` in chat store (TDD)

**Files:**
- Modify: `desktop/lib/stores/chat-store.ts`
- Modify: `desktop/lib/stores/chat-store-update-plan.test.ts`

**Step 1: Add failing store tests**

Add tests in `chat-store-update-plan.test.ts`:
```ts
it("hydrates plan and todoQueue from update_plan output", () => {
  // plan.queue.todos present
});

it("derives todoQueue from plan.tasks when queue.todos missing", () => {
  // fallback mapping should happen
});

it("normalizes unknown todo status to pending", () => {
  // unknown -> pending
});
```

**Step 2: Run tests and confirm fail**

Run:
```bash
pnpm --dir desktop test -- chat-store-update-plan.test.ts
```
Expected: FAIL before parser/type updates.

**Step 3: Implement parser and VM separation**

In `chat-store.ts`, extend types:
```ts
export interface TodoQueueItemVM {
  id: string;
  title: string;
  description?: string;
  status: "pending" | "in_progress" | "blocked" | "completed" | "failed";
}

export interface TodoQueueVM {
  todos: TodoQueueItemVM[];
  updatedAt: number;
}
```

Add `todoQueue?: TodoQueueVM` in `AgentTurnVM`.

Add parsers:
- `normalizeTodoStatus`
- `normalizeTodoItem`
- `parseTodoQueueFromPlan`
- `deriveTodoQueueFromTasks`

In `tool_call_completed(update_plan)` and structured plan update path:
1. hydrate `turn.plan`
2. hydrate `turn.todoQueue` from `plan.queue.todos`
3. fallback derive from `plan.tasks` when missing.

Keep existing `turn.queue.items` behavior unchanged (tool queue).

**Step 4: Re-run store tests**

Run:
```bash
pnpm --dir desktop test -- chat-store-update-plan.test.ts
```
Expected: PASS.

**Step 5: Commit store parsing changes**

Run:
```bash
git add desktop/lib/stores/chat-store.ts desktop/lib/stores/chat-store-update-plan.test.ts
git commit -m "feat(desktop): separate todo queue from tool queue in chat store"
```

---

### Task 3: Extend queue component to five-state TODO visualization (TDD)

**Files:**
- Modify: `desktop/components/ai-elements/queue.tsx`
- Modify: `desktop/components/features/chat/turn-process-sections.test.tsx`

**Step 1: Add failing UI assertions for five-state queue items**

In `turn-process-sections.test.tsx`, add assertions for status label rendering and style hooks for:
- `pending`
- `in_progress`
- `blocked`
- `completed`
- `failed`

**Step 2: Run tests to confirm fail**

Run:
```bash
pnpm --dir desktop test -- turn-process-sections.test.tsx
```
Expected: FAIL due to unsupported status rendering.

**Step 3: Implement queue component extension**

In `ai-elements/queue.tsx`:

1. Extend `QueueTodo.status` type to five-state union.
2. Update `QueueItemIndicator` API to accept `status?: ...` while keeping `completed?: boolean` backward compatibility.
3. Map statuses to class tokens (single-list semantics), e.g.:
```ts
const STATUS_INDICATOR_CLASS = {
  pending: "border-muted-foreground/50 bg-transparent",
  in_progress: "border-blue-400 bg-blue-400/20",
  blocked: "border-amber-400 bg-amber-400/20",
  completed: "border-emerald-500/30 bg-emerald-500/15",
  failed: "border-red-500/40 bg-red-500/20",
};
```

**Step 4: Re-run tests**

Run:
```bash
pnpm --dir desktop test -- turn-process-sections.test.tsx
```
Expected: PASS for queue status assertions.

**Step 5: Commit queue component updates**

Run:
```bash
git add desktop/components/ai-elements/queue.tsx desktop/components/features/chat/turn-process-sections.test.tsx
git commit -m "feat(desktop): support five-state todo queue indicators"
```

---

### Task 4: Render TODO Queue section in process UI (TDD)

**Files:**
- Modify: `desktop/components/features/chat/turn-process-view-model.ts`
- Modify: `desktop/components/features/chat/turn-process-view-model.test.ts`
- Modify: `desktop/components/features/chat/turn-process-sections.tsx`
- Modify: `desktop/components/features/chat/turn-process-sections.test.tsx`

**Step 1: Add failing tests for new `queue` process section**

Add tests for:
1. section order includes `plan -> queue -> tools`.
2. queue preview uses latest todo status summary.
3. single-list queue items rendered from `turn.todoQueue.todos`.
4. queue section still works when tools section exists.

**Step 2: Run tests to confirm fail**

Run:
```bash
pnpm --dir desktop test -- turn-process-view-model.test.ts turn-process-sections.test.tsx
```
Expected: FAIL before section implementation.

**Step 3: Implement minimal section logic**

In `turn-process-view-model.ts`:
1. Extend `TurnProcessSectionKey` to include `"queue"`.
2. Build queue section when `turn.todoQueue?.todos.length > 0`.
3. Keep tool queue metrics unchanged.

In `turn-process-sections.tsx`:
1. Add `renderQueue(section)`.
2. Use `Queue`, `QueueSection`, `QueueList`, `QueueItem`, `QueueItemIndicator`, `QueueItemContent`, `QueueItemDescription`.
3. Render single list only.
4. Preserve existing plan/tools/terminal behavior.

**Step 4: Re-run tests**

Run:
```bash
pnpm --dir desktop test -- turn-process-view-model.test.ts turn-process-sections.test.tsx
```
Expected: PASS.

**Step 5: Commit queue section rendering**

Run:
```bash
git add desktop/components/features/chat/turn-process-view-model.ts desktop/components/features/chat/turn-process-view-model.test.ts desktop/components/features/chat/turn-process-sections.tsx desktop/components/features/chat/turn-process-sections.test.tsx
git commit -m "feat(desktop): render todo queue section after plan"
```

---

### Task 5: Verification and docs sync

**Files:**
- Modify: `docs/plans/2026-03-04-update-plan-frontend-adaptation-design.md` (implementation status checklist)

**Step 1: Run focused verification suite**

Run:
```bash
cargo test -p agent-tool update_plan -- --nocapture
pnpm --dir desktop test -- chat-store-update-plan.test.ts turn-process-view-model.test.ts turn-process-sections.test.tsx
```
Expected: PASS.

**Step 2: Run smoke checks**

Run:
```bash
cargo test -p agent-tool -- --nocapture
pnpm --dir desktop test
```
Expected: PASS or documented unrelated failures.

**Step 3: Update design doc status block**

Append/update status block:
```md
## Implementation Status
- [x] update_plan payload extension
- [x] plan + todoQueue parsing
- [x] queue five-state UI
- [x] process section integration
- [x] tests
```

**Step 4: Commit verification + doc status**

Run:
```bash
git add docs/plans/2026-03-04-update-plan-frontend-adaptation-design.md
git commit -m "docs(plan): mark plan+todo-queue implementation status"
```

**Step 5: Produce review handoff summary**

Run:
```bash
git log --oneline -n 10
```
Expected: commit sequence mirrors tasks 1-5.

---

## Done Definition

1. `update_plan` supports optional `view/lifecycle/progress/queue.todos` and keeps old fields.
2. Desktop store separates planning (`plan`) from execution (`todoQueue`) and preserves tool queue behavior.
3. Process view renders `Plan` and `Queue` as separate sections, with Queue as TODO single list.
4. Queue supports `pending/in_progress/blocked/completed/failed`.
5. Targeted Rust + Vitest suites pass.

