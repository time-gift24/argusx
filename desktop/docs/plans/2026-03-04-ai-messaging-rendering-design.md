# AI 消息渲染系统设计文档

**日期：** 2026-03-04  
**状态：** 设计阶段（已按审查意见修订）  
**影响范围：** components/ai, components/ai-elements, features/chat

---

## 一、背景与目标

### 1.1 当前问题

1. **样式分散**
   - MessageResponse、ReasoningContent、TurnProcessSections 各自定义样式
   - Typography 样式重复（`text-[13px] leading-5`、`[&_li]:my-0.5` 等）
   - 行内代码样式在 `streamdown.css` 中硬编码

2. **字体大小不一致**
   - 消息文本：13px
   - 行内代码：14px（text-sm）
   - 紧凑文本：11px

3. **Whitespace 处理不当**
   - 曾经用 `whitespace-pre-wrap` 修复导致普通列表间距异常
   - 纯文本目录树（ASCII）换行显示不稳定

4. **架构不清晰**
   - `ai-elements` 混合基础组件与业务组件
   - 依赖边界不明确

### 1.2 设计目标

1. 统一样式系统，样式集中在 `components/ai/styles.ts`
2. 架构清晰，`components/ai` 可被任意业务层复用
3. 只保留确定性渲染规则，避免启发式“猜测”
4. 类型安全，TypeScript 优先
5. **特殊样式（runtime surface）严格遵循设计图，不允许自由变体**

---

## 二、硬性约束（Non-negotiable）

### 2.1 特殊样式来源唯一

所有特殊样式（代码块容器、过程容器）必须复用同一基础样式 token：`CHAT_STYLES.runtimeSurface.base`。

- 允许差异：仅限 `variant`（例如 `code`, `process`）的少量增量 class
- 禁止：在业务组件中直接写 `bg-*`、`border-*`、`rounded-*` 覆盖 runtime surface
- 禁止：不同组件维护独立“看起来差不多”的容器样式

### 2.2 代码高亮触发规则唯一

只在“确定是代码块”时高亮，规则如下：

- 输入必须是 fenced code block（Markdown AST `code` 节点）
- 且 `language` 在高亮白名单中
- 未声明 language 或 language 不在白名单：**仅用等宽字体展示，不做语法高亮**
- 行内 code（inline code）永不语法高亮
- 纯文本中的“像代码内容”永不自动高亮（禁止启发式识别）

| 内容类型 | 是否高亮 | 说明 |
|---|---|---|
| ```rust ...``` | 是 | fenced + allowlist |
| ```text ...``` | 否 | 明确为 text |
| ```unknown ...``` | 否 | language 不在 allowlist |
| ``` ...```（无语言） | 否 | 不确定语言 |
| `inline code` | 否 | 行内 code 仅基础样式 |
| 普通段落中的代码片段 | 否 | 禁止自动猜测 |

---

## 三、架构设计

### 3.1 目录结构

```text
components/
├── ai/
│   ├── styles.ts
│   ├── index.ts
│   ├── types.ts
│   ├── message.tsx
│   ├── reasoning.tsx
│   ├── plan.tsx
│   ├── tool.tsx
│   ├── terminal.tsx
│   ├── code-block.tsx
│   ├── runtime-code-surface.tsx
│   ├── streamdown-code.tsx
│   ├── streamdown-plugins.ts
│   └── README.md
├── ai-elements/                 # 标记废弃，仅保留迁移期兼容
└── ui/
```

### 3.2 依赖关系

- `components/ai` 只依赖 `components/ui`、`streamdown`、`lib/utils`
- `components/ai` 不依赖 `features/chat`
- `features/chat` 与其他业务层统一依赖 `components/ai`

---

## 四、样式系统设计

### 4.1 Typography Scale

| 名称 | 字体大小 | 行高 | 用途 |
|---|---|---|---|
| `text.base` | 12px | 18px | 消息正文 |
| `text.compact` | 11px | 14px | 标签、紧凑内容 |
| `code.inline` | 12px | 16px | 行内 code |
| `code.block` | 12px | 16px | 代码块 |

### 4.2 统一样式配置（高复用）

**文件：** `components/ai/styles.ts`

```typescript
export const CHAT_STYLES = {
  text: {
    base: "text-[12px] leading-[18px] whitespace-normal",
    compact: "text-[11px] leading-[14px]",
    muted: "text-[12px] leading-[18px] text-muted-foreground",
  },

  code: {
    inline: "rounded bg-muted px-1.5 py-0.5 font-mono text-[12px] leading-[16px]",
    block: "font-mono text-[12px] leading-[16px] whitespace-pre-wrap",
  },

  runtimeSurface: {
    // 所有特殊容器必须复用该 token
    base: "llm-chat-runtime-surface rounded-xl border",
    variant: {
      code: "bg-[var(--chat-runtime-surface-bg)] border-[var(--chat-runtime-surface-border)]",
      process: "bg-[var(--chat-runtime-surface-bg)] border-[var(--chat-runtime-surface-border)]",
    },
    header: "flex items-center justify-between px-3 py-2 border-b border-[var(--chat-runtime-surface-border)]",
    body: "px-3 py-2",
  },

  spacing: {
    list: "[&_li]:my-0.5 [&_ol]:my-1 [&_ul]:my-1",
  },
} as const;

export const CODE_HIGHLIGHT_ALLOWLIST = [
  "rust",
  "typescript",
  "javascript",
  "tsx",
  "jsx",
  "python",
  "go",
  "java",
  "kotlin",
  "swift",
  "bash",
  "shell",
  "shell-session",
  "json",
  "yaml",
  "toml",
  "sql",
  "html",
  "css",
] as const;
```

### 4.3 主题变量（对齐设计图的暗色代码面板）

**文件：** `app/styles/theme.css`

```css
@theme {
  --chat-runtime-surface-bg: #2f2f31;
  --chat-runtime-surface-border: #3a3a3d;
  --chat-runtime-surface-radius: 0.75rem;
  --chat-runtime-surface-text: #f2f2f3;
  --chat-runtime-surface-label: #b9b9bb;

  --chat-runtime-code-font-size: 12px;
  --chat-runtime-code-line-height: 16px;
  --chat-runtime-code-padding-x: 0.75rem;
  --chat-runtime-code-padding-y: 0.5rem;
}
```

> 说明：颜色以设计图视觉为准，变量值可微调，但只能在该变量层调整，不允许组件层私自改色。

---

## 五、组件规范

### 5.1 代码块组件

模式：左上语言标签 + 右侧复制按钮 + header/body 分隔。

实现约束：
- 外层：`CHAT_STYLES.runtimeSurface.base + CHAT_STYLES.runtimeSurface.variant.code`
- header：`CHAT_STYLES.runtimeSurface.header`
- 内容：`<pre className={CHAT_STYLES.code.block}>`
- 仅当 fenced + allowlist 时启用语法高亮
- 无 language / 非 allowlist：走“纯等宽文本”渲染器

### 5.2 过程组件（Reasoning/Plan/Tool/Terminal）

实现约束：
- 默认折叠 `defaultOpen={false}`
- 容器复用：`CHAT_STYLES.runtimeSurface.base + CHAT_STYLES.runtimeSurface.variant.process`
- 文本统一用 `CHAT_STYLES.text.base`
- 禁止在过程组件引入独立 surface 色板

---

## 六、Whitespace 与 Markdown 策略

### 6.1 CSS 作用域规则

```css
@layer components {
  .chat-text {
    white-space: normal;
  }

  /* 仅 block code 保留空白 */
  .chat-text pre {
    white-space: pre-wrap;
  }

  /* inline code 保持正常流式换行 */
  .chat-text :not(pre) > code {
    white-space: normal;
  }
}
```

### 6.2 Prompt 规范（辅助，不作为正确性前提）

System Prompt 继续要求：代码/目录树/终端输出使用 fenced code block。

但前端逻辑不得依赖“模型一定遵守 prompt”作为唯一保证；即使模型输出不规范，也不能把普通文本误判为可高亮代码。

---

## 七、迁移计划

### 7.1 迁移范围

从 `components/ai-elements/*` 迁移：
- `message.tsx`
- `reasoning.tsx`
- `plan.tsx`
- `tool.tsx`
- `terminal.tsx`
- `code-block.tsx`
- `runtime-code-surface.tsx`
- `streamdown-code.tsx`
- `streamdown-plugins.ts`

### 7.2 迁移原则

1. 先迁移 `styles.ts` 与 `streamdown-plugins.ts`（统一入口）
2. 再迁移组件实现，删除组件内重复样式
3. 最后改业务层 import

### 7.3 废弃策略

`ai-elements` 对应文件标注 `@deprecated`，保留一个版本周期后移除。

---

## 八、测试策略

### 8.1 正向场景

- 普通 Markdown 文本渲染正常
- inline code 有统一基础样式
- fenced + allowlist language 的代码块出现语法高亮
- 代码块头部样式（语言标签、复制按钮）符合设计图
- 过程组件折叠/展开正常

### 8.2 反向场景（必须覆盖）

- fenced 但无 language：不高亮
- fenced + `text`：不高亮
- fenced + unknown language：不高亮
- inline code：不高亮
- 普通段落出现 `fn main()`：不高亮

### 8.3 回归检查点

- 全部特殊容器是否复用 `runtimeSurface.base`
- 是否仍有 `streamdown.css` 中硬编码 code 样式
- Whitespace 是否只作用在 `pre`，未污染 inline code

---

## 九、风险与缓解

1. **迁移回归风险**
   - 缓解：先引入高亮判定单测，再迁移 UI

2. **样式漂移风险**
   - 缓解：通过单一 token（`runtimeSurface.base`）和禁止组件层自定义色板

3. **模型输出不规范风险**
   - 缓解：前端采用确定性判定，不依赖启发式识别

---

## 十、成功标准

1. 所有特殊样式渲染只经由 `runtimeSurface` token 输出
2. 仅 fenced + allowlist 代码块高亮，零误高亮
3. 迁移后无视觉回归，无明显性能下降
4. 文档与测试均覆盖正向与反向场景

---

## 十一、后续优化边界

- 可扩展 allowlist，但必须走评审
- 可微调主题变量，但必须保持与设计图同一视觉语言
- **不做自动检测目录树/代码启发式高亮**（与本设计目标冲突）

---

## 附录：参考资源

- [Tailwind CSS v4 文档](https://tailwindcss.com/docs)
- [Streamdown 文档](https://streamdown.dev)
- [shadcn/ui 设计系统](https://ui.shadcn.com)
