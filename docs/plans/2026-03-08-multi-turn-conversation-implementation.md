# Argusx 多轮对话架构设计

## 概述

本文档定义 Argusx 多轮对话的架构边界与核心流程，目标是先建立稳定的 turn 运行时边界，再在其上增加 thread 级历史累积、桌面端会话编排，以及后续的持久化与上下文压缩能力。

这份文档只描述职责、流程和阶段划分，不展开字段、方法、测试或实现细节。

## 目标

### In Scope

- 明确 `turn` crate 与上层会话编排层的职责边界
- 定义单 thread 多 turn 的多轮对话主流程
- 明确 turn 完成后如何沉淀为可复用的历史产物
- 明确桌面端如何基于 turn 运行时继续组织多轮会话
- 给出后续持久化与 context compression 的演进方向

### Out of Scope

- 数据结构字段设计
- API 签名与类型细节
- 数据库存储模型
- checkpoint / undo / 真正的 turn resume
- 多 thread 切换 UI

## 核心判断

### 1. `turn` 保持运行时边界

`turn` 负责单次 turn 的执行、事件流和完成产物，不直接承担完整 session/thread 生命周期编排。

### 2. 会话编排上移

多轮对话的历史管理、活跃 thread 管理和后续持久化，应由 `turn` 之上的 conversation manager 承担。这个 manager 可以位于 desktop chat 后端，或在未来抽成独立的 orchestration 层。

### 3. 先完成单 thread 多 turn

MVP 先聚焦单 thread 内连续多轮对话，不把 thread 切换、多窗口恢复、undo 等能力绑进第一阶段。

### 4. 完成产物先于会话模型

在建立 session/thread 之前，必须先让单次 turn 产生稳定、可追加到历史中的完成产物。否则上层没有可靠的历史输入来源。

## 架构概览

```text
User Input
  ->
Conversation Manager
  ->
Prepare Thread History
  ->
Turn Runtime (`turn`)
  ->
Model Loop + Tool Execution + Event Stream
  ->
Completed Turn Artifact
  ->
Append To Thread History
  ->
Next User Turn Reuses Thread History
```

## 分层职责

### Conversation Manager

- 维护当前对话 thread
- 决定下一轮 turn 使用哪份历史
- 接收 turn 完成产物并追加到 thread 历史
- 派生对话状态给上层 UI 或 IPC 层

### Turn Runtime

- 执行单个 turn
- 产出流式事件
- 在 turn 结束时返回可复用的完成产物
- 保证工具调用、工具结果和最终 assistant 回复被纳入本轮上下文

### History Layer

- 表示“可供下一轮复用的历史”
- 作为 conversation manager 和 turn runtime 之间的边界对象
- 后续可以映射到持久化层或摘要层

## 多轮流程

1. 用户发送新消息。
2. conversation manager 读取当前 thread 历史。
3. manager 将 thread 历史作为输入交给 `turn` 运行时，并附带新的用户消息。
4. `turn` 执行单轮推理、工具调用与事件流输出。
5. turn 结束后返回“完成产物”。
6. manager 将完成产物追加到当前 thread 历史。
7. 下一轮用户消息继续复用该 thread 历史。

## 状态流转

### 对话级

`Idle -> RunningTurn -> AwaitingTurnResult -> ReadyForNextTurn`

### Turn 级

`Created -> Streaming -> ToolExecution -> Finished`

对话级状态只表达编排层视角，turn 级细节仍由 `turn` 运行时维护。

## 演进阶段

### Phase A: Turn Runtime Boundary

先让 `turn` 支持两件事：

- 接收历史输入
- 产出稳定的完成产物

这是多轮对话的第一优先级，也是后续所有上层能力的前提。

### Phase B: In-Memory Conversation Manager

在 desktop chat 后端或单独 orchestration 层引入 conversation manager，用于管理单 thread 多 turn 的历史累积。

### Phase C: Persistence And Compression

当内存态流程稳定后，再增加：

- thread 历史持久化
- 长上下文摘要
- 重启恢复

### Phase D: Advanced Session Features

在持久化边界清晰后，再评估：

- 多 thread 切换
- checkpoint / undo
- 更复杂的恢复策略

## 更新进度

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 多轮对话架构文档 | 完成 | 当前文档已整理为纯架构、流程与进度说明 |
| Priority 1 实现计划 | 完成 | 已单独拆出为 turn 运行时边界实现计划 |
| Turn Runtime Boundary | 完成 | `turn` 已支持历史输入与 completed turn artifact |
| Phase B 实现计划 | 完成 | 已补充 desktop conversation manager 实现计划 |
| In-Memory Conversation Manager | 完成 | desktop backend、Tauri commands 与 chat page 多轮流转已接通 |
| Phase C 实现计划 | 完成 | 已补充 persistence 与 compression implementation plan |
| Persistence / Compression | 完成 | file-backed conversation repository、启动重载与 compression boundary 已落地 |
| Phase D 实现计划 | 完成 | 已补充 advanced session features implementation plan |
| Advanced Session Features | 完成 | thread catalog、checkpoint branch restore 与 explicit restart flows 已落地 |

## 关联文档

- `docs/plans/2026-03-07-chat-turn-connection-design.md`
- `docs/plans/2026-03-08-multi-turn-runtime-boundary-implementation.md`
- `docs/plans/2026-03-08-desktop-conversation-manager-implementation.md`
- `docs/plans/2026-03-08-conversation-persistence-compression-implementation.md`
- `docs/plans/2026-03-08-advanced-session-features-implementation.md`
