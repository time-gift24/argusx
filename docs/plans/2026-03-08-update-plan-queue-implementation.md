# update_plan Queue 追踪 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 让 desktop chat runtime 暴露并追踪 `update_plan`，在每轮对话前端使用 AI Elements `Queue` 展示该轮最新计划快照，并在计划完成后保留完成态。

**Architecture:** 后端先把 `UpdatePlanTool` 接入 desktop chat 的 tool scheduler 与 provider tool definitions，再由 observer 将成功的 `update_plan` 结果提升成独立的 `plan-updated` 事件。前端新增一个薄 `PlanQueue` 适配层，只保留每轮最新计划快照，并在 chat turn 渲染中将其显示在用户消息之后、助手正文之前，同时避免 `update_plan` 在普通工具列表中重复出现。

**Tech Stack:** Rust (Tauri v2, turn/tool/provider crates), TypeScript (Next.js 16, React 19, Vitest), AI Elements `Queue`

---

### Task 1: 接入 AI Elements Queue 组件与前端适配层

**Files:**
- Create: `desktop/components/ai-elements/queue.tsx`
- Create: `desktop/components/ai/plan-queue.tsx`
- Create: `desktop/components/ai/plan-queue.test.tsx`
- Modify: `desktop/components/ai/index.ts`
- Inspect: `desktop/components/ai-elements/checkpoint.tsx`

**Step 1: 安装 Queue 组件文件**

Run:

```bash
cd /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop
npx ai-elements@latest add queue
```

Expected: `desktop/components/ai-elements/queue.tsx` 被创建

**Step 2: Write the failing test**

```tsx
it("renders the latest plan snapshot with completed and pending tasks", () => {
  render(
    <PlanQueue
      plan={{
        title: "Execution Plan",
        description: "Starting execution",
        isStreaming: true,
        sourceCallId: "call-1",
        tasks: [
          { id: "task-1", title: "Write failing test", status: "completed" },
          { id: "task-2", title: "Implement minimal fix", status: "pending" },
        ],
      }}
    />
  );

  expect(screen.getByText("Write failing test")).toBeInTheDocument();
  expect(screen.getByText("Implement minimal fix")).toBeInTheDocument();
});
```

**Step 3: Run test to verify it fails**

Run:

```bash
pnpm --dir desktop exec vitest run components/ai/plan-queue.test.tsx
```

Expected: FAIL because `PlanQueue` does not exist yet

**Step 4: Write minimal implementation**

```tsx
export type PlanSnapshot = {
  title: string;
  description?: string | null;
  isStreaming: boolean;
  sourceCallId: string;
  tasks: Array<{
    id: string;
    title: string;
    status: "pending" | "in_progress" | "completed";
  }>;
};

export function PlanQueue({ plan }: { plan: PlanSnapshot }) {
  return (
    <Queue>
      <QueueSection>
        <QueueSectionContent>
          {plan.tasks.map((task) => (
            <QueueItem key={task.id}>
              <QueueItemIndicator completed={task.status === "completed"} />
              <QueueItemContent completed={task.status === "completed"}>
                {task.title}
              </QueueItemContent>
            </QueueItem>
          ))}
        </QueueSectionContent>
      </QueueSection>
    </Queue>
  );
}
```

Include:
- thin mapping layer from backend `PlanSnapshot` to `Queue` props
- optional description rendering
- completed vs non-completed indicator behavior
- export from `desktop/components/ai/index.ts`

**Step 5: Run test to verify it passes**

Run:

```bash
pnpm --dir desktop exec vitest run components/ai/plan-queue.test.tsx
```

Expected: PASS

**Step 6: Commit**

```bash
git add desktop/components/ai-elements/queue.tsx desktop/components/ai/plan-queue.tsx desktop/components/ai/plan-queue.test.tsx desktop/components/ai/index.ts
git commit -m "feat: add plan queue adapter"
```

---

### Task 2: 在 desktop chat runtime 中注册 UpdatePlanTool

**Files:**
- Modify: `desktop/src-tauri/src/chat/tools.rs`
- Modify: `desktop/src-tauri/src/chat/model.rs`
- Test: `desktop/src-tauri/src/chat/tools.rs`
- Test: `desktop/src-tauri/src/chat/model.rs`

**Step 1: Write the failing tests**

In `desktop/src-tauri/src/chat/model.rs` test module, add:

```rust
#[test]
fn build_request_includes_update_plan_tool_when_allowed() {
    let runner = ProviderModelRunner::from_replay("gpt-test", PathBuf::from("fixture.sse")).unwrap();
    let request = sample_tool_enabled_request();
    let built = runner.build_request(&request);
    let tools = built.tools.expect("tools should be present");

    assert!(tools.iter().any(|tool| tool.function.name == "update_plan"));
}
```

In `desktop/src-tauri/src/chat/tools.rs` test module, add:

```rust
#[tokio::test]
async fn scheduled_tool_runner_executes_update_plan() {
    let runner = ScheduledToolRunner::from_current_dir().unwrap();
    let result = runner.execute(
        sample_update_plan_call(),
        sample_tool_context()
    ).await.unwrap();

    assert_eq!(result.output["plan"]["tasks"][0]["title"], "Write failing test");
}
```

**Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p desktop chat::model::tests::build_request_includes_update_plan_tool_when_allowed chat::tools::tests::scheduled_tool_runner_executes_update_plan -- --nocapture
```

Expected: FAIL because `update_plan` is not yet registered

**Step 3: Write minimal implementation**

```rust
BuiltinRegistration::new(
    Builtin::UpdatePlan,
    Arc::new(UpdatePlanTool),
    policy,
)
```

Also extend provider tool definitions:

```rust
to_provider_tool(&UpdatePlanTool)
```

Include:
- `UpdatePlanTool` import wiring
- scheduler registration in `ScheduledToolRunner`
- provider tool definition exposure in `read_only_tool_definitions`
- tool count assertions updated from 3 to 4

**Step 4: Run tests to verify they pass**

Run:

```bash
cargo test -p desktop chat::model::tests::build_request_includes_update_plan_tool_when_allowed chat::tools::tests::scheduled_tool_runner_executes_update_plan -- --nocapture
```

Expected: PASS

**Step 5: Commit**

```bash
git add desktop/src-tauri/src/chat/tools.rs desktop/src-tauri/src/chat/model.rs
git commit -m "feat: expose update plan tool in desktop chat runtime"
```

---

### Task 3: 将 update_plan 成功结果提升为 plan-updated 事件

**Files:**
- Modify: `desktop/src-tauri/src/chat/observer.rs`
- Modify: `desktop/src-tauri/src/chat/events.rs`
- Create: `desktop/src-tauri/src/chat/plan.rs`
- Test: `desktop/src-tauri/src/chat/observer.rs`

**Step 1: Write the failing tests**

Add observer tests covering:

```rust
#[test]
fn map_turn_event_emits_plan_updated_for_update_plan_success() {
    let event = sample_update_plan_completed_event();
    let payload = map_turn_event("turn-1", TurnTargetKind::Agent, "reviewer", &event).unwrap();

    assert_eq!(payload.event_type, "plan-updated");
    assert_eq!(payload.data["tasks"][0]["title"], "Write failing test");
}

#[test]
fn map_turn_event_ignores_invalid_update_plan_payload() {
    let event = sample_invalid_update_plan_completed_event();
    let payload = map_turn_event("turn-1", TurnTargetKind::Agent, "reviewer", &event);

    assert!(payload.is_none());
}
```

**Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p desktop chat::observer::tests::map_turn_event_emits_plan_updated_for_update_plan_success chat::observer::tests::map_turn_event_ignores_invalid_update_plan_payload -- --nocapture
```

Expected: FAIL because `tool-call-completed` currently never becomes `plan-updated`

**Step 3: Write minimal implementation**

```rust
pub struct DesktopPlanSnapshot {
    pub title: String,
    pub description: Option<String>,
    pub tasks: Vec<DesktopPlanTask>,
    pub is_streaming: bool,
    pub source_call_id: String,
}
```

Then in `observer.rs`:

```rust
if tool_name == "update_plan" {
    if let Some(plan) = parse_plan_snapshot(call_id, result) {
        return Some(DesktopTurnEvent {
            turn_id: turn_id.to_string(),
            event_type: "plan-updated".to_string(),
            data: serde_json::to_value(plan).unwrap(),
        });
    }
}
```

Include:
- helper parser for `ToolOutcome::Success(output.plan)`
- validation for `title`, `tasks`, `task.status`
- `sourceCallId` population
- keep existing behavior for non-`update_plan` tools

**Step 4: Run tests to verify they pass**

Run:

```bash
cargo test -p desktop chat::observer::tests::map_turn_event_emits_plan_updated_for_update_plan_success chat::observer::tests::map_turn_event_ignores_invalid_update_plan_payload -- --nocapture
```

Expected: PASS

**Step 5: Commit**

```bash
git add desktop/src-tauri/src/chat/observer.rs desktop/src-tauri/src/chat/events.rs desktop/src-tauri/src/chat/plan.rs
git commit -m "feat: emit plan updated events for update_plan"
```

---

### Task 4: 扩展前端 chat turn 状态以只保留最新计划快照

**Files:**
- Modify: `desktop/lib/chat.ts`
- Modify: `desktop/lib/chat.test.ts`
- Modify: `desktop/app/chat/page.tsx`
- Modify: `desktop/app/chat/page.test.tsx`

**Step 1: Write the failing tests**

In `desktop/app/chat/page.test.tsx`, add:

```tsx
it("keeps only the latest plan snapshot for a turn", async () => {
  render(<ChatPage />);

  await startFirstTurn();
  await act(async () => {
    onTurnEvent?.({
      turnId: "turn-1",
      type: "plan-updated",
      data: firstPlanPayload,
    });
    onTurnEvent?.({
      turnId: "turn-1",
      type: "plan-updated",
      data: secondPlanPayload,
    });
  });

  expect(screen.queryByText("Write failing test")).not.toBeInTheDocument();
  expect(screen.getByText("Implement minimal fix")).toBeInTheDocument();
});
```

Also add a `lib/chat.test.ts` case confirming `subscribe()` forwards `plan-updated` payload unchanged.

**Step 2: Run tests to verify they fail**

Run:

```bash
pnpm --dir desktop exec vitest run app/chat/page.test.tsx lib/chat.test.ts
```

Expected: FAIL because `ChatTurnView` does not store plan snapshots yet

**Step 3: Write minimal implementation**

```ts
type PlanSnapshot = {
  title: string;
  description?: string | null;
  tasks: Array<{ id: string; title: string; status: "pending" | "in_progress" | "completed" }>;
  isStreaming: boolean;
  sourceCallId: string;
};

type ChatTurnView = {
  // existing fields...
  latestPlan: PlanSnapshot | null;
};
```

Then in the reducer:

```ts
case "plan-updated":
  return {
    ...current,
    latestPlan: parsePlanSnapshot(event.data) ?? current.latestPlan,
  };
```

Include:
- default `latestPlan: null` in new turn creation
- event payload parser with runtime guards
- no history array, only latest snapshot overwrite

**Step 4: Run tests to verify they pass**

Run:

```bash
pnpm --dir desktop exec vitest run app/chat/page.test.tsx lib/chat.test.ts
```

Expected: PASS

**Step 5: Commit**

```bash
git add desktop/lib/chat.ts desktop/lib/chat.test.ts desktop/app/chat/page.tsx desktop/app/chat/page.test.tsx
git commit -m "feat: track latest plan snapshot in chat turns"
```

---

### Task 5: 在 chat turn UI 中渲染 Queue 并隐藏 update_plan 普通工具项

**Files:**
- Modify: `desktop/app/chat/page.tsx`
- Modify: `desktop/app/chat/page.test.tsx`
- Modify: `desktop/components/ai/tool-call-item.tsx`
- Test: `desktop/components/ai/plan-queue.test.tsx`

**Step 1: Write the failing tests**

Add chat page assertions:

```tsx
it("renders the queue after the user bubble and before assistant markdown", async () => {
  render(<ChatPage />);

  await startFirstTurn();
  await act(async () => {
    onTurnEvent?.({ turnId: "turn-1", type: "plan-updated", data: completedPlanPayload });
    onTurnEvent?.({ turnId: "turn-1", type: "llm-text-delta", data: { text: "Assistant answer" } });
  });

  const assistantSection = screen.getByText("Assistant answer").closest('[data-slot=\"chat-turn-assistant\"]');
  expect(within(assistantSection!).getByText("Execution Plan")).toBeInTheDocument();
  expect(within(assistantSection!).queryByText("update_plan")).not.toBeInTheDocument();
});
```

**Step 2: Run tests to verify they fail**

Run:

```bash
pnpm --dir desktop exec vitest run app/chat/page.test.tsx components/ai/plan-queue.test.tsx
```

Expected: FAIL because chat page does not render `Queue`

**Step 3: Write minimal implementation**

```tsx
{turn.latestPlan ? <PlanQueue plan={turn.latestPlan} /> : null}
```

And filter tool calls:

```ts
{turn.toolCalls
  .filter((toolCall) => toolCall.name !== "update_plan")
  .map(...)}
```

Place `PlanQueue` immediately before assistant `Streamdown`.

**Step 4: Run tests to verify they pass**

Run:

```bash
pnpm --dir desktop exec vitest run app/chat/page.test.tsx components/ai/plan-queue.test.tsx
```

Expected: PASS

**Step 5: Commit**

```bash
git add desktop/app/chat/page.tsx desktop/app/chat/page.test.tsx desktop/components/ai/plan-queue.tsx desktop/components/ai/plan-queue.test.tsx
git commit -m "feat: render plan queue in chat turns"
```

---

### Task 6: Final verification

**Files:**
- Inspect: `desktop/src-tauri/src/chat/tools.rs`
- Inspect: `desktop/src-tauri/src/chat/model.rs`
- Inspect: `desktop/src-tauri/src/chat/observer.rs`
- Inspect: `desktop/app/chat/page.tsx`
- Inspect: `desktop/components/ai/plan-queue.tsx`

**Step 1: Run Rust verification**

```bash
cargo test -p desktop -- --nocapture
```

Expected: PASS

**Step 2: Run front-end verification**

```bash
pnpm --dir desktop exec vitest run app/chat/page.test.tsx lib/chat.test.ts components/ai/plan-queue.test.tsx components/ai/reasoning.test.tsx components/ai/tool-call-item.test.tsx components/ai/stream-item.test.tsx
```

Expected: PASS

**Step 3: Run workspace sanity checks**

```bash
cargo check -p desktop
git diff --check
```

Expected: all commands succeed with exit code 0

**Step 4: Commit**

```bash
git add -A
git commit -m "chore: verify update plan queue tracking"
```
