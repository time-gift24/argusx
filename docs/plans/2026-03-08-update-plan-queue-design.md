# update_plan Queue 追踪设计

## 概述

本文档定义桌面 chat 在前端对 `update_plan` 的可视化追踪方案，以及后端将 `update_plan` 提升为一等事件的集成方式。

目标是让一轮对话中的 `update_plan` 不再只作为普通 tool result 被压缩成文本摘要，而是能以结构化计划快照的形式显示在前端，并在同一轮内随着多次 `update_plan` 调用持续对齐最新状态。

## 目标

- 让 desktop chat runtime 正式暴露 `update_plan`
- 在一轮对话中支持多次 `update_plan` 调用
- 前端仅展示该轮最新的一份计划快照
- 使用 `ai-elements` 的 `Queue` 组件承载计划显示
- 将 `Queue` 放在每轮助手输出顶部，位于用户消息之后、正文之前
- 当计划全部 `completed` 后仍保留完成态显示

## 非目标

- 不替换现有 `PromptComposer`
- 不接入 `PromptInput`、`ModelSelector`、`Attachments`
- 不保留 `update_plan` 的历史版本演进
- 不在 V1 中强制模型策略层必须持续调用 `update_plan` 直到全部完成
- 不在本次范围内实现计划状态的 SQLite 持久化

## 当前问题

当前桌面 chat 的工具层只注册了 `read/glob/grep`，`update_plan` 虽然在 [tool](/Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/tool/src/lib.rs) 中存在，但并未接入 desktop chat runtime。

同时，前端只消费通用的 `tool-call-completed` 事件，在 [page.tsx](/Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/app/chat/page.tsx) 中将所有工具结果都压成 `ToolCallItem`。这会导致 `update_plan` 的结构化 `plan.tasks` 丢失，无法在 UI 上稳定展示“剩余任务”和“当前进度”。

## 方案选择

### 方案 A：一等事件流（推荐）

- 后端正式注册 `UpdatePlanTool`
- provider tool definitions 也暴露 `update_plan`
- observer 识别 `update_plan` 成功结果，并额外发出 `plan-updated` 事件
- 前端只消费结构化的 `plan-updated`，不再自己解析普通 tool output

优点：

- 前后端语义清晰
- 前端不依赖工具输出细节
- 后续易于扩展为持久化或历史回放

缺点：

- 需要扩展 desktop turn event 契约

### 方案 B：前端解析通用 tool result

- 后端不改事件协议
- 前端在 `tool-call-completed` 中识别 `name=update_plan`
- 从 `result.output.plan` 直接解析 `tasks`

优点：

- 改动较少

缺点：

- 前端与工具输出 schema 强耦合
- 协议脆弱，不利于后续维护

### 方案 C：后端持久化计划状态

- 每次 `update_plan` 写入数据库
- 前端从 turn state 或 SQLite 读取计划

优点：

- 恢复能力最好

缺点：

- 对 V1 过重
- 明显超出当前 UI 集成范围

本设计采用方案 A。

## 后端设计

### Tool 注册

desktop chat 的 [tools.rs](/Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/src-tauri/src/chat/tools.rs) 需要把 `UpdatePlanTool` 注册到 `ToolScheduler` 中，使其与 `read/glob/grep` 一起成为本轮可调用 builtin tools。

### Provider tool definitions

[model.rs](/Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/src-tauri/src/chat/model.rs) 当前只向 provider 暴露 `read/glob/grep` 的 tool definitions。这里也要补上 `update_plan`，否则模型不会生成该 tool call。

### 事件提升

observer 在收到 `TurnEvent::ToolCallCompleted` 时：

- 若工具不是 `update_plan`，保持现有行为
- 若工具是 `update_plan` 且结果成功，则解析 `ToolResult.output.plan`
- 若解析成功，则额外 emit 一个新的 `DesktopTurnEvent`

建议新增事件类型：

- `plan-updated`

建议 payload 结构：

```json
{
  "title": "Execution Plan",
  "description": "Starting execution",
  "tasks": [
    { "id": "task-1", "title": "Write failing test", "status": "in_progress" },
    { "id": "task-2", "title": "Implement minimal fix", "status": "pending" }
  ],
  "isStreaming": true,
  "sourceCallId": "call-123"
}
```

其中：

- `tasks` 直接沿用 `update_plan` 现有结构化输出
- `isStreaming` 用于区分“还有剩余步骤”与“已全部完成”
- `sourceCallId` 方便前端关联本次快照来源

### “保障”边界

V1 中，后端的保障边界定义为：

- `update_plan` 对模型可见且可调用
- 每次有效调用都会被校验
- 每次有效调用都会被转换成结构化计划快照
- 快照会持续流式发送给前端

V1 不负责强制模型“必须持续调用到全部完成”。这属于后续 agent loop / orchestration 的策略问题，不在本次集成范围内。

## 前端状态设计

### ChatTurnView 扩展

[page.tsx](/Users/wanyaozhong/projects/argusx/.worktrees/chat-turn-connection/desktop/app/chat/page.tsx) 中的 `ChatTurnView` 需要新增：

- `latestPlan: null | PlanSnapshot`

其中 `PlanSnapshot` 建议定义为：

- `title`
- `description`
- `tasks`
- `isStreaming`
- `sourceCallId`

### 事件归并

`reduceTurnEventForTurn` 新增 `plan-updated` 分支：

- 收到后直接覆盖 `latestPlan`
- 不保留旧版本
- 若 payload 非法，则忽略本次事件并保留上一版 `latestPlan`

## UI 设计

### Queue 位置

每轮渲染顺序调整为：

1. 用户消息
2. `Queue`（如果该轮存在 `latestPlan`）
3. 助手正文 `Streamdown`
4. `reasoning`
5. 其他 tool calls
6. turn 级错误信息

`Queue` 的位置固定在助手区域顶部，也就是用户消息之后、正文之前。

### 展示策略

- 同一轮里多次 `update_plan` 只显示最新快照
- 从未调用过 `update_plan` 的轮次不显示 `Queue`
- 计划全部 `completed` 后继续保留完成态
- turn 失败/取消时保留最后一版 plan，不自动清空

### 与 ToolCallItem 的关系

`update_plan` 不应该在普通 `ToolCallItem` 列表里重复显示，否则同一语义会在 UI 上重复一层。

因此建议：

- `update_plan` 仍存在于后端 tool call 流里
- 但前端渲染普通 tool calls 时跳过 `update_plan`
- 计划相关可视化统一由 `Queue` 负责

### Queue 组件策略

当前仓库还没有 `queue` 组件文件，因此需要先执行：

```bash
npx ai-elements@latest add queue
```

建议新增一个薄适配层组件，例如 `PlanQueue`：

- 输入：`PlanSnapshot`
- 输出：`Queue` 组件树
- 职责：把后端 `tasks` 映射到 AI Elements 所需数据结构和状态展示

这样可以避免把 plan mapping 逻辑散落在 chat page 中。

## 错误处理

- `update_plan` 执行失败：保留普通 `ToolCallItem` 错误态，不更新 `latestPlan`
- `plan-updated` payload 非法：前端忽略本次更新，保留上一版计划
- turn 失败但计划未完成：`Queue` 保留最后快照，turn 错误独立显示
- turn 取消：`Queue` 仍显示最后快照，不额外伪造完成态

## 测试策略

### Rust

- `ScheduledToolRunner` 已注册 `UpdatePlanTool`
- provider tool definitions 暴露 `update_plan`
- observer 遇到 `update_plan` 成功结果时发出 `plan-updated`
- observer 遇到普通工具结果或非法 plan 结果时不发 `plan-updated`

### Frontend

- `plan-updated` 会覆盖同一轮旧计划
- `Queue` 显示在用户消息之后、正文之前
- 全部 `completed` 后仍保留完成态
- `update_plan` 不再重复显示成普通 `ToolCallItem`

### 集成

- 一轮内多次 `update_plan` 时，只保留最新快照
- turn 失败或取消时，最后一版计划仍可见

## 结果

V1 完成后，desktop chat 将能把 `update_plan` 从“普通工具输出”提升为“结构化计划追踪流”。用户在每一轮中都可以直接看到当前最新的执行计划、剩余任务以及最终完成态，而无需从工具原始输出中手动辨认。
