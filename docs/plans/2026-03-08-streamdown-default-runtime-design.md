# Streamdown Default Runtime Design

## Goal

让 `desktop` 里所有当前运行时使用的 `Streamdown` 都退回原生默认行为，去掉额外的自定义样式、组件覆盖、控制按钮、图标、翻译和插件注入，同时保留现有自定义层代码作为历史参考并标记为 `@deprecated`。

## Current State

当前 `desktop` 对 `Streamdown` 做了三层定制：

- 运行时接入层会统一传入自定义 `className`、`components`、`controls`、`icons`、`plugins`、`translations`、`shikiTheme`
- 视觉层通过 `app/globals.css` 中的 `.ai-streamdown` 规则重写默认排版、代码块、Mermaid、操作按钮和展开交互
- 组件层通过 `streamdown-code.tsx` 和 `streamdown-config.ts` 定义了自定义 code/mermaid 包装和共享配置

这些定制目前被 `chat`、`Reasoning`、`AI Elements reasoning` 以及 `dev/streamdown` 示例页共同复用。

## Chosen Approach

采用“运行时全部退回默认 + 自定义层保留但废弃”的方案。

- 所有当前运行时 `Streamdown` 调用点改成最小用法，只保留 `children`，以及确实需要的 `isAnimating`
- 不再传入任何共享自定义 props，因此 `.ai-streamdown` 不再挂载到运行时 DOM 上
- 保留现有自定义模块和样式块，但明确标注 `@deprecated`，说明仅用于历史参考，运行时不再依赖

这样可以最快让你看到原生 `Streamdown` 的真实效果，同时不破坏后续回滚路径。

## Scope

运行时移除自定义的使用点：

- `desktop/app/chat/page.tsx`
- `desktop/components/ai/reasoning.tsx`
- `desktop/components/ai-elements/reasoning.tsx`
- `desktop/app/dev/streamdown/streamdown-playground.tsx`

保留但废弃的自定义层：

- `desktop/components/ai/streamdown-config.ts`
- `desktop/components/ai/streamdown-code.tsx`
- `desktop/app/globals.css` 中 `.ai-streamdown` 自定义样式块

## Non-Goals

- 不删除 `@streamdown/code` / `@streamdown/mermaid` / `@streamdown/math` / `@streamdown/cjk` 依赖
- 不清理所有与旧自定义层相关的测试，只会把断言改成和“默认运行时”一致
- 不顺手重做 `StreamItem`、`Reasoning`、`ToolCallItem` 的布局结构

## Expected Result

- 聊天页助手正文、Reasoning 内容、dev playground 都显示原生 `Streamdown` 风格
- 当前仓库中仍然能找到旧的自定义接入代码，但这些文件/导出已经标记为废弃
- 回归测试验证运行时不再依赖旧的共享自定义 props
