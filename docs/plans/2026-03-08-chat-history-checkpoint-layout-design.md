# Chat 多轮历史与 Checkpoint 布局设计

## 概述

本文档定义桌面 `/chat` 页面新的消息布局，目标是去掉当前“整轮卡片”样式，改为保留多轮历史、使用 `Checkpoint` 组件分隔每轮对话，并将输入框改成底部悬浮层。

## 目标

- 保留多轮历史，而不是只显示最新一轮
- 每轮对话顶部使用 `Checkpoint` 分隔，文案为 `第 N 轮`
- 用户消息显示在右侧，使用系统一致的轻底色
- 助手正文保持无背景、无卡片，直接作为正文流渲染
- 输入框悬浮在 chat 页面底部，不随消息滚动
- 滚动容器为悬浮输入框预留安全底部空间，避免遮挡最后一轮内容

## 非目标

- 不重写现有 turn 事件协议
- 不替换现有 `Streamdown`、`Reasoning`、`ToolCallItem` 组件
- 不修改 provider settings 或 chat backend 协议
- 不引入新的消息编辑、重试、分支会话能力

## 当前问题

当前 [page.tsx](/Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/app/chat/page.tsx) 将整轮内容包裹在一个带边框和底色的卡片中，同时 `PromptComposer` 仍处在正常文档流中。

这导致两个问题：

- 助手正文被限制在卡片视觉语言里，不符合“正文就是正文”的预期
- 消息区滚动时，输入框会被内容继续往下挤，而不是固定悬浮在页面底部

## 组件策略

### Checkpoint

使用 `ai-elements` 的 `Checkpoint` 组件作为每轮对话的显式分隔符。当前仓库还没有该组件文件，因此需要先执行：

```bash
npx ai-elements@latest add checkpoint
```

该组件只负责“第 N 轮”的结构分隔，不承担消息卡片职责。

### 现有渲染组件保留

- 助手正文继续使用 [Streamdown](/Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/app/chat/page.tsx)
- reasoning 继续使用 [Reasoning](/Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/components/ai/reasoning.tsx)
- tool 调用继续使用 [ToolCallItem](/Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/components/ai/tool-call-item.tsx)

这样可以只替换布局，不推翻当前 turn 渲染逻辑。

## 视图结构

每一轮 turn 统一渲染为：

1. `Checkpoint`，文案为 `第 N 轮`
2. 用户消息
3. 助手正文
4. reasoning
5. tool 调用
6. 错误信息（如果该轮失败）

### 用户消息

- 右侧对齐
- 使用轻底色气泡
- 保持圆角和适度内边距
- 不增加重边框，和系统现有 `muted/card` 调性一致

### 助手正文

- 左侧正文流
- 不包卡片
- 不加背景色
- `Streamdown` 直接占据消息正文宽度
- 若还未收到正文 token，仅显示轻量占位提示

### reasoning / tool

- 仍附着在当前轮的助手正文之后
- 复用现有组件
- 不额外包裹整轮外层容器

## 数据结构调整

当前页面状态只维护一个 `ChatViewState`。新的布局需要改为维护 `ChatTurnView[]`。

每个 turn 至少包含：

- `turnId`
- `index`
- `prompt`
- `assistantText`
- `reasoningText`
- `toolCalls`
- `error`
- `status`

### turn 生命周期

- 用户提交时立即创建一条新 turn，并先写入用户 prompt
- `start_turn` 成功后回填真实 `turnId`
- 后续流式事件按 `turnId` 归并到对应 turn
- 如果用户在上一轮进行中再次提交，新 turn 创建后旧 turn 可继续保留在历史中，但停止接收后续渲染

## 滚动与悬浮输入框

### 页面结构

- 外层仍为纵向 flex 容器
- 中间消息区域单独滚动
- 底部 composer 作为绝对定位或 sticky 悬浮层，固定在 chat 页面底部

### 安全留白

消息滚动容器底部必须增加动态 padding，值大于 composer 实际高度。

原因：

- composer 半透明悬浮时会覆盖视觉底部
- 若底部留白不足，最后一轮的 assistant/tool 内容会被压在 composer 下方

实现上推荐通过 ref 测量 composer 容器高度，而不是写死常量。

### 自动滚动

- 当用户位于底部附近时，新内容到来自动滚动到底部
- 当用户主动向上查看历史时，不强制抢回滚动位置

## 视觉规则

- 删除当前整轮卡片的边框、底色和阴影
- `Checkpoint` 作为唯一轮次分隔视觉
- 用户消息可以有底色
- 助手正文严格保持透明背景
- composer 使用轻透明背景和少量 blur，使其悬浮但不厚重

## 测试策略

### 前端

- 多次提交后保留多轮历史
- 每轮之间出现 `第 N 轮` checkpoint
- 用户消息在右侧显示气泡样式
- 助手正文不再包裹在整轮卡片中
- composer 在滚动时保持悬浮
- 消息区底部有足够留白，不被 composer 遮挡

### 回归

- 现有 turn 事件仍能正确驱动正文、reasoning、tool 渲染
- cancel turn 行为不回归
- root route `/` 仍直接渲染 chat 页面

## 结果

改造完成后，`/chat` 将从“单轮卡片”视图切换为“多轮历史正文流”视图：轮次由 `Checkpoint` 分隔，用户消息右侧展示，助手输出保持系统正文风格，底部输入框悬浮且不再被滚动内容挤走。
