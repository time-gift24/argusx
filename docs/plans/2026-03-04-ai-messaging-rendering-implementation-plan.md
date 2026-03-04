# AI Messaging Rendering Deterministic Surface Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 让 AI 消息渲染满足两条强约束：所有特殊样式只复用统一 runtime surface，且仅 fenced + allowlist 的确定代码块启用语法高亮。

**Architecture:** 在 `desktop/components/ai` 建立样式与高亮策略中心（token + policy），聊天渲染层只消费 policy 结果，不做启发式猜测。`runtime-markdown-block` 负责判定，`runtime-code-surface` 负责统一 UI；未知语言/无语言/inline code 全部走非高亮路径。

**Tech Stack:** Next.js 16, React 19, TypeScript, Streamdown, Shiki, Tailwind CSS v4, Vitest, Testing Library

---

**Execution rules:**
- 严格按 `@test-driven-development` 执行（先写失败测试再实现）
- 每个任务结束执行 `@verification-before-completion`
- 每 1 个任务提交 1 次 commit，避免大批量改动

**Execution mode (locked by user): Option 2 - Parallel Session**
- 在独立并行会话执行本计划（`superpowers:executing-plans`），不中断等待用户逐阶段确认
- 自动推进策略：阶段 Gate 通过后直接进入下一阶段，直到最终 PR
- 阶段划分：
1. Phase 1 = Task 1 + Task 2
2. Phase 2 = Task 3 + Task 4
3. Phase 3 = Task 5 + Task 6

**Mandatory phase review loop (after each phase)**
1. 记录阶段前后提交：
   - `BASE_SHA=<phase-start-sha>`
   - `HEAD_SHA=$(git rev-parse HEAD)`
2. 运行 `codex-code-review`（等价于 `superpowers:code-reviewer`）审查 `BASE_SHA..HEAD_SHA`
3. 修复所有 Critical/Important 问题，Minor 只在不阻塞时顺手修复
4. 重新执行本阶段涉及的测试与 lint，直到通过
5. 提交修复并自动进入下一阶段（不需要用户反馈）

### Task 1: 建立确定性高亮策略模块

**Files:**
- Create: `desktop/components/ai/highlight-policy.ts`
- Create: `desktop/components/ai/highlight-policy.test.ts`
- Test: `desktop/components/ai/highlight-policy.test.ts`

**Step 1: Write the failing test**

```ts
import { describe, expect, it } from "vitest";
import { shouldHighlightFence } from "./highlight-policy";

describe("shouldHighlightFence", () => {
  it("returns true for fenced + allowlisted language", () => {
    expect(shouldHighlightFence({ isFenced: true, language: "rust" })).toBe(true);
  });

  it("returns false when language is empty", () => {
    expect(shouldHighlightFence({ isFenced: true, language: "" })).toBe(false);
  });

  it("returns false for text and unknown languages", () => {
    expect(shouldHighlightFence({ isFenced: true, language: "text" })).toBe(false);
    expect(shouldHighlightFence({ isFenced: true, language: "foo-lang" })).toBe(false);
  });

  it("returns false when not fenced", () => {
    expect(shouldHighlightFence({ isFenced: false, language: "rust" })).toBe(false);
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm --dir desktop exec vitest run components/ai/highlight-policy.test.ts`  
Expected: FAIL with `Cannot find module './highlight-policy'`.

**Step 3: Write minimal implementation**

```ts
const CODE_HIGHLIGHT_ALLOWLIST = new Set([
  "rust",
  "typescript",
  "javascript",
  "tsx",
  "jsx",
  "python",
  "go",
  "java",
  "bash",
  "shell",
  "shell-session",
  "json",
  "yaml",
  "toml",
  "sql",
  "html",
  "css",
]);

export function shouldHighlightFence(input: {
  isFenced: boolean;
  language?: string;
}): boolean {
  if (!input.isFenced) return false;
  const lang = (input.language ?? "").trim().toLowerCase();
  if (!lang || lang === "text") return false;
  return CODE_HIGHLIGHT_ALLOWLIST.has(lang);
}
```

**Step 4: Run test to verify it passes**

Run: `pnpm --dir desktop exec vitest run components/ai/highlight-policy.test.ts`  
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/components/ai/highlight-policy.ts desktop/components/ai/highlight-policy.test.ts
git commit -m "test(chat): add deterministic code highlight policy"
```

### Task 2: 在 RuntimeMarkdownBlock 接入策略并补全反向测试

**Files:**
- Modify: `desktop/components/features/chat/runtime-markdown-block.tsx`
- Modify: `desktop/components/features/chat/runtime-code-surface.tsx`
- Modify: `desktop/components/features/chat/runtime-markdown-block.test.tsx`
- Test: `desktop/components/features/chat/runtime-markdown-block.test.tsx`

**Step 1: Write the failing test**

```tsx
it("does not syntax-highlight fenced code when language is missing", () => {
  const { container } = renderMarkdown("```\nfn main() {}\n```");
  expect(container.querySelector('[data-highlighted="false"]')).toBeTruthy();
});

it("does not syntax-highlight fenced text language", () => {
  const { container } = renderMarkdown("```text\nfn main() {}\n```");
  expect(container.querySelector('[data-highlighted="false"]')).toBeTruthy();
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm --dir desktop exec vitest run components/features/chat/runtime-markdown-block.test.tsx`  
Expected: FAIL on new assertions.

**Step 3: Write minimal implementation**

```tsx
// runtime-markdown-block.tsx
const highlighted = shouldHighlightFence({
  isFenced: true,
  language: parsed.language,
});

return (
  <RuntimeCodeSurface
    code={parsed.code}
    isIncomplete={props.isIncomplete}
    language={parsed.language}
    highlighted={highlighted}
  />
);
```

```tsx
// runtime-code-surface.tsx
export interface RuntimeCodeSurfaceProps {
  // ...existing props
  highlighted?: boolean;
}

// code mode branch
if (!highlighted) {
  return (
    <div className="llm-chat-code-surface llm-chat-runtime-surface" data-highlighted="false">
      <pre><code>{code}</code></pre>
    </div>
  );
}

return <CodeBlock ... data-highlighted="true" />;
```

**Step 4: Run test to verify it passes**

Run: `pnpm --dir desktop exec vitest run components/features/chat/runtime-markdown-block.test.tsx`  
Expected: PASS（含反向用例）。

**Step 5: Commit**

```bash
git add desktop/components/features/chat/runtime-markdown-block.tsx desktop/components/features/chat/runtime-code-surface.tsx desktop/components/features/chat/runtime-markdown-block.test.tsx
git commit -m "feat(chat): apply deterministic highlight gating for fenced code"
```

### Phase 1 Review Gate (auto-continue)

**Scope:** Task 1 + Task 2

1. Run review:
   - `BASE_SHA=<phase-1-start-sha>`
   - `HEAD_SHA=$(git rev-parse HEAD)`
   - 使用 `codex-code-review` 审查 `BASE_SHA..HEAD_SHA`
2. Fix review findings:
   - 必须修复 Critical/Important
   - 修复后新增 commit（例如：`fix(chat): resolve phase-1 review findings`）
3. Re-verify:
   - `pnpm --dir desktop exec vitest run components/ai/highlight-policy.test.ts`
   - `pnpm --dir desktop exec vitest run components/features/chat/runtime-markdown-block.test.tsx`
4. Auto-continue:
   - 上述检查通过后，直接进入 Phase 2（Task 3）

### Task 3: 抽取并落地统一 runtime surface 样式 token

**Files:**
- Create: `desktop/components/ai/styles.ts`
- Create: `desktop/components/ai/types.ts`
- Create: `desktop/components/ai/index.ts`
- Modify: `desktop/components/features/chat/runtime-code-surface.tsx`
- Modify: `desktop/components/features/chat/runtime-process-section.tsx`
- Test: `desktop/components/features/chat/runtime-process-section.test.tsx`

**Step 1: Write the failing test**

```tsx
it("reuses runtime surface token for process and code containers", () => {
  const { container } = render(<RuntimeProcessSection ... />);
  expect(container.querySelectorAll(".llm-chat-runtime-surface").length).toBeGreaterThan(0);
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm --dir desktop exec vitest run components/features/chat/runtime-process-section.test.tsx`  
Expected: FAIL（尚未统一 token 引用）。

**Step 3: Write minimal implementation**

```ts
// desktop/components/ai/styles.ts
export const CHAT_STYLES = {
  runtimeSurface: {
    base: "llm-chat-runtime-surface rounded-xl border",
    variant: {
      code: "bg-[var(--chat-runtime-surface-bg)] border-[var(--chat-runtime-surface-border)]",
      process: "bg-[var(--chat-runtime-surface-bg)] border-[var(--chat-runtime-surface-border)]",
    },
  },
} as const;
```

```tsx
// runtime-code-surface.tsx / runtime-process-section.tsx
className={cn(CHAT_STYLES.runtimeSurface.base, CHAT_STYLES.runtimeSurface.variant.code)}
```

**Step 4: Run test to verify it passes**

Run: `pnpm --dir desktop exec vitest run components/features/chat/runtime-process-section.test.tsx`  
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/components/ai/styles.ts desktop/components/ai/types.ts desktop/components/ai/index.ts desktop/components/features/chat/runtime-code-surface.tsx desktop/components/features/chat/runtime-process-section.tsx desktop/components/features/chat/runtime-process-section.test.tsx
git commit -m "refactor(chat): centralize runtime surface styles in components/ai"
```

### Task 4: 修正 whitespace 作用域并迁移 Streamdown 代码组件

**Files:**
- Create: `desktop/components/ai/streamdown-code.tsx`
- Create: `desktop/components/ai/streamdown-plugins.ts`
- Modify: `desktop/components/features/chat/turn-process-sections.tsx`
- Modify: `desktop/components/features/chat/runtime-markdown-block.test.tsx`
- Modify: `desktop/app/styles/streamdown.css`
- Test: `desktop/components/features/chat/turn-process-sections.test.tsx`

**Step 1: Write the failing test**

```tsx
it("keeps inline code non-highlighted while preserving block code rendering", () => {
  const { container } = render(/* turn process markdown with inline + fenced */);
  expect(container.querySelector(".llm-chat-markdown :not(pre) > code")).toBeTruthy();
  expect(container.querySelector('[data-highlighted="true"]')).toBeTruthy();
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm --dir desktop exec vitest run components/features/chat/turn-process-sections.test.tsx`  
Expected: FAIL on inline-code scope assertion.

**Step 3: Write minimal implementation**

```css
/* desktop/app/styles/streamdown.css */
.chat-text pre { white-space: pre-wrap; }
.chat-text :not(pre) > code { white-space: normal; }
```

```tsx
// use new imports
import { STREAMDOWN_PLUGINS } from "@/components/ai/streamdown-plugins";
import { StreamdownCode } from "@/components/ai/streamdown-code";
```

**Step 4: Run test to verify it passes**

Run: `pnpm --dir desktop exec vitest run components/features/chat/turn-process-sections.test.tsx`  
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/components/ai/streamdown-code.tsx desktop/components/ai/streamdown-plugins.ts desktop/components/features/chat/turn-process-sections.tsx desktop/components/features/chat/turn-process-sections.test.tsx desktop/app/styles/streamdown.css
git commit -m "fix(chat): scope whitespace rules and migrate streamdown code renderer"
```

### Phase 2 Review Gate (auto-continue)

**Scope:** Task 3 + Task 4

1. Run review:
   - `BASE_SHA=<phase-2-start-sha>`
   - `HEAD_SHA=$(git rev-parse HEAD)`
   - 使用 `codex-code-review` 审查 `BASE_SHA..HEAD_SHA`
2. Fix review findings:
   - 必须修复 Critical/Important
   - 修复后新增 commit（例如：`fix(chat): resolve phase-2 review findings`）
3. Re-verify:
   - `pnpm --dir desktop exec vitest run components/features/chat/runtime-process-section.test.tsx`
   - `pnpm --dir desktop exec vitest run components/features/chat/turn-process-sections.test.tsx`
4. Auto-continue:
   - 上述检查通过后，直接进入 Phase 3（Task 5）

### Task 5: 迁移核心 UI 组件到 components/ai 并更新聊天入口引用

**Files:**
- Create: `desktop/components/ai/message.tsx`
- Create: `desktop/components/ai/reasoning.tsx`
- Create: `desktop/components/ai/plan.tsx`
- Create: `desktop/components/ai/tool.tsx`
- Create: `desktop/components/ai/terminal.tsx`
- Create: `desktop/components/ai/code-block.tsx`
- Modify: `desktop/components/features/chat/conversation-view.tsx`
- Modify: `desktop/components/features/chat/agent-turn-card.tsx`
- Modify: `desktop/components/features/chat/turn-process-sections.tsx`
- Test: `desktop/components/features/chat/conversation-view-scroll.test.tsx`
- Test: `desktop/components/features/chat/turn-process-sections.test.tsx`

**Step 1: Write the failing test**

```tsx
it("imports MessageResponse from components/ai and renders assistant message", () => {
  // smoke render for conversation view
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm --dir desktop exec vitest run components/features/chat/conversation-view-scroll.test.tsx components/features/chat/turn-process-sections.test.tsx`  
Expected: FAIL before import migration.

**Step 3: Write minimal implementation**

```tsx
// old
import { Message, MessageResponse } from "@/components/ai-elements/message";

// new
import { Message, MessageResponse } from "@/components/ai/message";
```

**Step 4: Run test to verify it passes**

Run: `pnpm --dir desktop exec vitest run components/features/chat/conversation-view-scroll.test.tsx components/features/chat/turn-process-sections.test.tsx`  
Expected: PASS.

**Step 5: Commit**

```bash
git add desktop/components/ai/message.tsx desktop/components/ai/reasoning.tsx desktop/components/ai/plan.tsx desktop/components/ai/tool.tsx desktop/components/ai/terminal.tsx desktop/components/ai/code-block.tsx desktop/components/features/chat/conversation-view.tsx desktop/components/features/chat/agent-turn-card.tsx desktop/components/features/chat/turn-process-sections.tsx
git commit -m "refactor(chat): migrate core rendering components to components/ai"
```

### Task 6: 废弃 ai-elements 对应模块并做最终验证

**Files:**
- Modify: `desktop/components/ai-elements/message.tsx`
- Modify: `desktop/components/ai-elements/reasoning.tsx`
- Modify: `desktop/components/ai-elements/plan.tsx`
- Modify: `desktop/components/ai-elements/tool.tsx`
- Modify: `desktop/components/ai-elements/terminal.tsx`
- Modify: `desktop/components/ai-elements/code-block.tsx`
- Modify: `desktop/app/styles/theme.css`
- Create: `desktop/components/ai/README.md`

**Step 1: Write the failing test**

```tsx
it("keeps runtime surface visual token consistent in light and dark themes", () => {
  // assert css vars for runtime surface are present and consumed
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm --dir desktop exec vitest run components/features/chat/runtime-markdown-block.test.tsx`  
Expected: FAIL on missing final token wiring/deprecation compatibility.

**Step 3: Write minimal implementation**

```tsx
/**
 * @deprecated 已迁移到 components/ai/message.tsx
 */
export { Message, MessageResponse } from "@/components/ai/message";
```

```css
/* theme.css runtime surface values align to design */
--chat-runtime-surface-bg: #2f2f31;
--chat-runtime-surface-border: #3a3a3d;
--chat-runtime-surface-text: #f2f2f3;
```

**Step 4: Run full verification**

Run:
- `pnpm --dir desktop exec vitest run components/features/chat/runtime-markdown-block.test.tsx`
- `pnpm --dir desktop exec vitest run components/features/chat/turn-process-sections.test.tsx`
- `pnpm --dir desktop lint`

Expected: 目标用例 PASS，lint 无新增错误。

**Step 5: Commit**

```bash
git add desktop/components/ai-elements/message.tsx desktop/components/ai-elements/reasoning.tsx desktop/components/ai-elements/plan.tsx desktop/components/ai-elements/tool.tsx desktop/components/ai-elements/terminal.tsx desktop/components/ai-elements/code-block.tsx desktop/app/styles/theme.css desktop/components/ai/README.md
git commit -m "chore(chat): deprecate ai-elements wrappers and finalize deterministic rendering rules"
```

### Phase 3 Review Gate (final before PR)

**Scope:** Task 5 + Task 6

1. Run review:
   - `BASE_SHA=<phase-3-start-sha>`
   - `HEAD_SHA=$(git rev-parse HEAD)`
   - 使用 `codex-code-review` 审查 `BASE_SHA..HEAD_SHA`
2. Fix review findings:
   - 必须修复 Critical/Important
   - 修复后新增 commit（例如：`fix(chat): resolve phase-3 review findings`）
3. Re-verify:
   - `pnpm --dir desktop exec vitest run components/features/chat/conversation-view-scroll.test.tsx`
   - `pnpm --dir desktop exec vitest run components/features/chat/turn-process-sections.test.tsx`
   - `pnpm --dir desktop exec vitest run components/features/chat/runtime-markdown-block.test.tsx`
   - `pnpm --dir desktop lint`

---

## Final verification gate (must pass before merge)

1. `pnpm --dir desktop exec vitest run components/features/chat/runtime-markdown-block.test.tsx`
2. `pnpm --dir desktop exec vitest run components/features/chat/turn-process-sections.test.tsx`
3. `pnpm --dir desktop exec vitest run components/features/chat/runtime-process-section.test.tsx`
4. `pnpm --dir desktop lint`

If any command fails, stop and fix before merging.

## Final PR Procedure (required)

1. Create/confirm feature branch（必须使用 `codex/` 前缀）:
   - `git checkout -b codex/ai-messaging-deterministic-rendering`（若分支已存在则跳过）
2. Ensure clean state:
   - `git status --short` 为空
3. Push branch:
   - `git push -u origin codex/ai-messaging-deterministic-rendering`
4. Prepare PR body file:
   - `cat > docs/plans/2026-03-04-ai-messaging-rendering-pr.md <<'EOF'`
   - 写入变更摘要、阶段性 `codex-code-review` 结论、测试证据
   - `EOF`
5. Create PR:
   - `gh pr create --title "feat(chat): deterministic AI messaging rendering surfaces" --body-file docs/plans/2026-03-04-ai-messaging-rendering-pr.md`
6. PR body 必须包含：
   - 设计约束落实清单（统一 runtime surface / 仅确定代码块高亮）
   - 三个阶段的 `codex-code-review` 结论与修复摘要
   - 最终测试命令与通过结果
