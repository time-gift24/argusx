# Chat History Checkpoint Layout Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将桌面 chat 页面从单轮卡片布局改为多轮历史布局，使用 `Checkpoint` 分隔每一轮，对用户消息和助手正文采用不同视觉语言，并让输入框悬浮在页面底部。

**Architecture:** 保留现有 turn 事件与渲染组件，只重构前端 view state 和页面布局。每次提交创建一个新的 turn 视图项，用 `Checkpoint` 组件分隔，并让消息区与 composer 分离为“独立滚动区 + 底部悬浮层”。

**Tech Stack:** Next.js 16, React 19, Vitest, shadcn/ui, ai-elements `checkpoint`, existing Streamdown/Reasoning/ToolCallItem

---

### Task 1: 安装并接入 Checkpoint 组件

**Files:**
- Create: `desktop/components/ai-elements/checkpoint.tsx`
- Modify: `desktop/package.json`
- Test: `desktop/components/settings/provider-settings-dialog.test.tsx`

**Step 1: Add the component with the official generator**

Run:

```bash
cd /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop
npx ai-elements@latest add checkpoint
```

Expected: `desktop/components/ai-elements/checkpoint.tsx` appears

**Step 2: Verify the generated file exists**

Run:

```bash
test -f /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/components/ai-elements/checkpoint.tsx
```

Expected: exit code `0`

**Step 3: Commit**

```bash
git add /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/components/ai-elements/checkpoint.tsx /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/package.json /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/pnpm-lock.yaml
git commit -m "feat: add checkpoint component for chat turn separators"
```

---

### Task 2: 将 chat 页面状态改为多轮历史

**Files:**
- Modify: `desktop/app/chat/page.tsx`
- Test: `desktop/app/chat/page.test.tsx`

**Step 1: Write the failing test**

Add a test that submits twice and asserts:

- page shows `第 1 轮`
- page shows `第 2 轮`
- first round content is still visible after second submission

**Step 2: Run test to verify it fails**

Run:

```bash
pnpm --dir /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop exec vitest run app/chat/page.test.tsx
```

Expected: FAIL because the page only keeps a single latest turn

**Step 3: Write minimal implementation**

Refactor page state from single `ChatViewState` to `ChatTurnView[]`, and:

- create a new turn view immediately on submit
- keep previous turns in history
- route streamed events by `turnId`
- keep cancel behavior for the active running turn

**Step 4: Run test to verify it passes**

Run:

```bash
pnpm --dir /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop exec vitest run app/chat/page.test.tsx
```

Expected: PASS

**Step 5: Commit**

```bash
git add /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/app/chat/page.tsx /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/app/chat/page.test.tsx
git commit -m "feat: preserve chat history across turns"
```

---

### Task 3: 用 Checkpoint + 正文流替换整轮卡片

**Files:**
- Modify: `desktop/app/chat/page.tsx`
- Test: `desktop/app/chat/page.test.tsx`

**Step 1: Write the failing test**

Add assertions that:

- each turn renders a `Checkpoint` label like `第 1 轮`
- user prompt appears in a right-aligned message block
- assistant `Streamdown` content is rendered without the old outer card shell

**Step 2: Run test to verify it fails**

Run:

```bash
pnpm --dir /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop exec vitest run app/chat/page.test.tsx
```

Expected: FAIL because the old `Latest Turn` card still exists

**Step 3: Write minimal implementation**

Update the page render tree to:

- render `Checkpoint` before each turn
- show user prompt on the right with a light background
- render assistant `Streamdown` as plain page content without the old card container
- keep `Reasoning` and `ToolCallItem` directly under assistant content

**Step 4: Run test to verify it passes**

Run:

```bash
pnpm --dir /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop exec vitest run app/chat/page.test.tsx
```

Expected: PASS

**Step 5: Commit**

```bash
git add /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/app/chat/page.tsx /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/app/chat/page.test.tsx
git commit -m "feat: render chat turns with checkpoints and inline assistant body"
```

---

### Task 4: 让 composer 悬浮到底部并补足滚动留白

**Files:**
- Modify: `desktop/app/chat/page.tsx`
- Possibly Modify: `desktop/components/ai/prompt-composer.tsx`
- Test: `desktop/app/chat/page.test.tsx`

**Step 1: Write the failing test**

Add a test that asserts:

- composer is rendered in a dedicated bottom overlay container
- scroll region has extra bottom padding/safe space
- page no longer places composer in the same vertical flow as message history

**Step 2: Run test to verify it fails**

Run:

```bash
pnpm --dir /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop exec vitest run app/chat/page.test.tsx
```

Expected: FAIL because composer still participates in normal layout flow

**Step 3: Write minimal implementation**

Implement:

- bottom overlay composer container
- translucent background and light blur
- measured composer height via ref
- dynamic bottom padding in the scroll region

**Step 4: Run test to verify it passes**

Run:

```bash
pnpm --dir /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop exec vitest run app/chat/page.test.tsx
```

Expected: PASS

**Step 5: Commit**

```bash
git add /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/app/chat/page.tsx /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/app/chat/page.test.tsx /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/components/ai/prompt-composer.tsx
git commit -m "feat: float chat composer above the scroll region"
```

---

### Task 5: 回归验证与 root route 校验

**Files:**
- Modify: `desktop/app/page.test.tsx`
- Modify: `desktop/components/layouts/app-layout.test.tsx`

**Step 1: Write any missing regression assertions**

Ensure tests still cover:

- `/` renders the chat workspace directly
- provider settings button remains visible in the header
- new layout does not reintroduce the retired placeholder

**Step 2: Run targeted frontend tests**

Run:

```bash
pnpm --dir /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop exec vitest run app/page.test.tsx app/chat/page.test.tsx components/layouts/app-layout.test.tsx lib/chat.test.ts
```

Expected: PASS

**Step 3: Run Rust regression**

Run:

```bash
cargo test -p desktop -- --nocapture
```

Expected: PASS

**Step 4: Run static validation**

Run:

```bash
cargo check -p desktop
git diff --check
```

Expected: PASS and no diff-check output

**Step 5: Commit**

```bash
git add /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/app/page.test.tsx /Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/components/layouts/app-layout.test.tsx
git commit -m "test: cover checkpoint chat layout regression"
```
