# Argusx Domain Notes

## Session -> Thread -> Turn

Argusx 当前的会话模型按 `Session -> Thread -> Turn` 三层拆分。
检视代码或设计时，先按这三个层级判断职责边界，再看具体实现是否越界。
`docs/plans/2026-03-08-session-thread-turn-*` 删除后，这里就是这套模型的权威摘要。

### Session

`Session` 是用户级隔离和跨线程上下文的容器，不负责单轮执行。

职责：
- 标识一个用户/工作区下的会话边界
- 持有跨线程共享的默认配置，例如默认模型、系统提示词
- 管理 thread 列表、当前 active thread、初始化恢复流程
- 对外暴露统一入口，例如 `create_thread`、`switch_thread`、`send_message`

不该承担的职责：
- 不重建一套 turn 状态机
- 不直接实现模型调用或工具执行
- 不把 UI 视角状态当成持久化真相源

当前代码中的核心对象：
- 持久化：`SessionRecord`
- 运行时：`SessionManager`、`SessionRuntime`

### Thread

`Thread` 是多轮对话的聚合根，负责把多个 `Turn` 串成一条连续对话流。

职责：
- 拥有有序的 turn 历史
- 保证同一 thread 在任意时刻最多只有一个 active turn
- 为下一轮 turn 重建 prior messages
- 管理 thread 级生命周期，例如 `Open` / `Archived`
- 承载“前台/后台”的运行语义，但这类状态是运行时派生信息，不是持久化真相

不该承担的职责：
- 不重复实现 `TurnDriver` 内部的流式状态推进
- 不把 `BackgroundProcessing` 之类 UI 视角直接落库
- 不保存整段历史的重复 transcript；每个 turn 只保存自己的增量消息

当前代码中的核心对象：
- 持久化：`ThreadRecord`
- 运行时：`ThreadRuntime`、`ActiveTurnRuntime`

### Turn

`Turn` 是单次用户输入触发的一次完整执行，包含模型调用、工具调用、权限审批和最终收尾。

职责：
- 接收本轮 user message 和 prior messages
- 驱动 LLM 流式输出
- 驱动 tool loop 和 permission flow
- 产生 `TurnEvent` 事件流
- 在正常完成时产出 `TurnOutcome`

`Turn` 是唯一应该真正理解单轮执行细节的层。
`Session` 和 `Thread` 只能消费它暴露出来的事件、transcript 和 outcome，不应该重复造一套并行协议。

当前代码中的核心对象：
- 输入：`TurnSeed`
- 执行器：`TurnDriver`
- 事件：`TurnEvent`
- 输出：`TurnOutcome`
- 持久化：`TurnRecord`

## 代码对象映射表

下面这张表用来把领域概念直接映射到当前代码里的核心类型。
检视时如果发现某个类型在做表格之外的事情，通常就值得重点怀疑它是否越界了。

| 领域概念 | 主要代码对象 | 角色说明 |
|----------|--------------|----------|
| Session 持久化实体 | `SessionRecord` | 落库存储 session 级稳定配置 |
| Session 应用服务 | `SessionManager` | 对外提供 session/thread 入口，协调 store、runtime、turn |
| Session 运行时状态 | `SessionRuntime` | 只保存内存态，例如 active thread 和 thread runtime |
| Thread 持久化实体 | `ThreadRecord` | 落库存储 thread 标题、生命周期、`last_turn_number` |
| Thread 运行时状态 | `ThreadRuntime` | 保存 thread 当前运行态，例如 active turn |
| Active turn 运行时句柄 | `ActiveTurnRuntime` | 保存当前 turn 的 reservation、controller、permission 等内存态 |
| Turn 持久化实体 | `TurnRecord` | 落库存储单轮输入、状态、增量 transcript、输出和时间戳 |
| Turn 启动输入 | `TurnSeed` | 把 prior messages 和当前 user message 交给 `TurnDriver` |
| Turn 单轮执行器 | `TurnDriver` | 真正驱动 LLM、tool loop、permission flow |
| Turn 事件协议 | `TurnEvent` | 向上游暴露单轮执行过程中的流式事件 |
| Turn 完成结果 | `TurnOutcome` | 正常完成时的统一收尾结果，供 session/thread 落库 |
| Session 持久化仓储 | `ThreadStore` | 负责 session/thread/turn 的 SQLite 读写 |
| Desktop 入口状态 | `DesktopSessionState` | 持有 `SessionManager` 和 desktop 侧运行依赖 |
| Desktop 事件桥 | `spawn_session_event_bridge` | 把 session/thread/turn 事件桥接到前端 UI |

### 推荐的阅读顺序

如果你要快速检视当前实现，按下面顺序看会更高效：

1. 先看 `AGENTS.md` 里的职责边界和不变量
2. 再看 `session/src/types.rs`，确认持久化模型长什么样
3. 再看 `session/src/thread.rs`，确认 thread runtime 和历史回放边界
4. 再看 `session/src/manager.rs`，确认 orchestration 是否越界
5. 最后看 `turn/src/*`，确认单轮执行细节是否被正确封装

## 运行时与持久化边界

要把“稳定事实”和“运行时状态”分开看。

稳定事实，应该持久化：
- session 默认配置
- thread 标题、生命周期、`last_turn_number`
- turn 的 `user_input`、`status`、`transcript`、`final_output`、时间戳

运行时状态，只存在内存：
- 当前 active thread
- 当前 active turn 的 controller
- 是否正在等待权限
- desktop 是否正在前台展示某个 thread

恢复语义：
- 应用重启后，只加载已经落盘的 session/thread/turn 历史
- `Running` / `WaitingPermission` 这类未完成 turn 在启动时统一标记为 `Interrupted`
- 不恢复未完成 turn 的真实执行，只恢复历史可见性

## 历史如何进入下一轮上下文

这是这套设计最关键的一条链路：

1. `Thread` 从已持久化的 turn 历史中挑出可回放的 turn
2. 把每个 turn 的增量 transcript 按顺序展开成 prior messages
3. 用这些 prior messages + 当前 user message 组成新的 `TurnSeed`
4. `TurnDriver` 基于这个 seed 执行下一轮

检视时要重点确认：
- 每个 turn 持久化的是“本轮增量”，不是“完整历史副本”
- 回放时不会重复消息，也不会漏掉失败 turn 已经产生的可见内容
- `Thread` 只负责拼接历史，不负责解释 LLM/tool 执行细节

## 当前关键不变量

1. 同一 `thread` 任意时刻只能有一个 active turn
2. active turn 的占坑必须先发生，再允许任何 `await`
3. `insert_turn` 和推进 `last_turn_number` 必须原子完成
4. 每个 turn 的 transcript 只保存本轮新增消息
5. failed turn 也要尽量落下已经对 UI 可见的增量内容
6. 切换 active thread 只影响 UI，不应取消后台运行中的 turn
7. desktop 事件桥即使 lagged，也不能永久退出

## 代码检视时的常见误区

### 误区 1：在 Thread 层再造一套 Turn 状态机

如果一个设计让 `Thread` 自己开始定义细粒度的 streaming/tool/permission 状态推进，通常说明边界已经错了。
这些语义应该来自 `TurnEvent`，而不是由 `Thread` 二次发明。

### 误区 2：把 UI 状态当成领域状态

例如“当前页面正在看哪个 thread”、“后台是否在跑”这类信息，通常是运行时派生状态。
它们可以影响展示，但不应该直接成为持久化真相。

### 误区 3：每轮都保存完整历史 transcript

这会导致：
- 存储随 turn 数平方增长
- 历史回放时重复消息
- 后续迁移成本越来越高

正确做法是：
- `TurnOutcome` 可以带完整 transcript 方便运行时收尾
- `Thread`/`Session` 落库时只切出当前 turn 的增量片段

### 误区 4：让 desktop 入口层持有全局大锁

Tauri command 层如果把整个 `SessionManager` 再包一层全局 async mutex，会把不必要的串行瓶颈带进 UI。
真正需要保护的是 thread 内的不变量，而不是把所有命令入口都串起来。

## 快速心智模型

- `Session`：用户级容器和入口
- `Thread`：多轮对话聚合根
- `Turn`：单轮执行引擎

可以把它理解成：
- `Session` 管理“有哪些对话”
- `Thread` 管理“这一条对话的连续历史”
- `Turn` 管理“这一次输入到底怎么执行完”

## 简版设计总结

- `Session` 负责用户级容器、默认配置和 thread 入口，不拥有单轮执行状态机。
- `Thread` 是多轮对话聚合根，负责有序历史、单 active turn 约束，以及把已落盘 transcript 回放成下一轮 `prior messages`。
- `Turn` 是唯一的单轮执行引擎：输入是 `TurnSeed`，运行时通过 `TurnEvent` 暴露过程，完成时通过 `TurnOutcome` 交回最终 transcript 和输出。
- 稳定事实只落 `sessions / threads / turns` 和每轮增量 transcript；前后台、审批等待、controller 这类状态只保存在运行时内存。
- 切换 active thread 只影响 UI 焦点，不取消后台 turn；应用重启只恢复历史可见性，并把 `Running` / `WaitingPermission` 统一收敛成 `Interrupted`。
