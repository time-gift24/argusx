# Session / Thread / Turn 重构设计（V2）

日期: 2026-03-08

## 目标

将当前的单 Turn 执行能力扩展为 `Session -> Thread -> Turn` 三层模型，并满足以下要求：

- `Thread` 管理多轮对话，每一轮历史参与下一轮模型上下文
- 切换 `thread` 时，当前运行中的 `turn` 在后台继续执行
- 应用重启后不恢复运行中的 `turn`，但要加载已经落盘的 `thread`/`turn` 历史
- 最大化复用现有 `turn` crate，避免在 `thread` 层重复实现工具调度、权限审批、取消、超时控制

## 非目标

- 不支持应用重启后恢复一个尚未完成的 `turn`
- 不在 `thread` 层再发明一套与 `TurnEvent` 平行的细粒度事件协议
- 不把“前台/后台”这类 UI 视角状态直接持久化到数据库
- 不在本文档中包含实施任务拆分或实现步骤

## 分层职责

保留你确认的职责划分，但明确“实体”和“运行时服务”要分开：

| 层级 | 职责 |
|------|------|
| `Session` | 用户级隔离、跨线程共享配置、生命周期管理 |
| `Thread` | 多轮对话编排、历史聚合、审批/认证协调、运行时状态管理 |
| `Turn` | 单次交互执行、工具调用追踪、权限暂停、取消、超时、事件流 |

补充说明：

- `Session` / `Thread` / `TurnRecord` 是领域模型或持久化模型
- `SessionManager` 是应用服务，不等同于 `Session` 实体
- `TurnDriver` 是执行引擎，不等同于 `TurnRecord`

## 先回顾 Turn 已有能力

在引入 `Thread` 之前，先明确现有 `turn` crate 已经负责了什么。

### 1. Turn 已经是一个完整的单轮执行引擎

现有 `TurnDriver` 已经支持：

- 启动一次模型交互并驱动整个单轮生命周期
- 在同一轮内执行多步 tool loop
- 将工具调用结果按顺序回灌到下一步 LLM 请求
- 处理工具权限审批 (`WaitingForPermission`)
- 处理中途取消 (`Cancel`)
- 处理工具超时、模型启动超时、流空闲超时、整轮 deadline
- 通过 `FinalStepPolicy` 和 `max_steps` 保证单轮有明确终止边界

### 2. Turn 已经暴露了运行时控制面

现有 `turn` 的控制边界是：

- `TurnDriver::spawn(...) -> (TurnHandle, JoinHandle<Result<..., TurnError>>)`
- `TurnHandle::next_event()` 用于消费 `TurnEvent`
- `TurnHandle::cancel()` 用于取消运行中的 turn
- `TurnHandle::resolve_permission()` 用于恢复审批暂停的 turn
- `TurnObserver` 可作为事件旁路，用于埋点、持久化、桥接 UI 事件

这意味着：

- `Thread` 不应该持有 `TurnDriver`
- `Thread` 应该持有 `TurnHandle + JoinHandle` 这样的运行时句柄
- `Thread` 的主要职责是“编排 turn”，不是“重写 turn”

### 3. Turn 已经有内部状态机

现有 `TurnState` 已经覆盖了单轮执行态：

- `Ready`
- `StreamingLlm`
- `WaitingTools`
- `WaitingForPermission`
- `Completed`
- `Cancelled`
- `Failed`

因此：

- `Thread` 不应复制一套细粒度的 turn 状态机
- `Thread` 只需要维护“是否存在活动 turn、它属于哪个 thread、是否在等待用户审批”这类编排层信息

### 4. Turn 已经定义了细粒度事件协议

现有 `TurnEvent` 已经覆盖：

- `TurnStarted`
- `LlmTextDelta`
- `LlmReasoningDelta`
- `ToolCallPrepared`
- `ToolCallCompleted`
- `ToolCallPermissionRequested`
- `ToolCallPermissionResolved`
- `StepFinished`
- `TurnFinished`

因此：

- `Thread` 层不应把这些事件重新降级成 `TurnMessage`
- 前端看到的 thread 级事件应以 `thread_id` 包装现有 `TurnEvent`
- `ThreadEvent::TurnEvent { thread_id, turn_id, event }` 比“重新定义一套 `TurnProgress`”更合适

### 5. Turn 的现有边界还缺两块能力

为了真正支持多轮 thread，`turn` 还缺两块关键能力：

1. `Turn` 当前只能从“当前用户输入”启动，不能直接带入前序轮次历史
2. `Turn` 当前会发出 `LlmTextDelta`，但没有稳定暴露“最终可持久化 transcript”作为单轮产物

这两个缺口如果不补，`thread` 层就会被迫自己重建 transcript 和 prompt，最终还是会重复造轮子。

## 核心设计原则

### 1. 单轮执行真相只保留一份

`turn` 是单轮执行的唯一真相源：

- 工具并发
- 审批暂停
- 取消
- 超时
- tool loop
- finish reason

全部留在 `turn`。

### 2. 多轮历史真相只保留一份

参与下一轮上下文的历史，不再拆成：

- `assistant_response`
- `tool_calls`
- `turn_progress`
- `thread_history`

四套平行模型。

设计上只保留一份“可回放到模型上下文中的持久化消息序列”作为真相源。

### 3. 持久化状态和运行时状态分离

以下信息必须是运行时派生态，而不是数据库真值：

- 当前 thread 是否在前台
- 当前运行 turn 是否在后台继续执行
- 当前审批弹窗是否显示在 UI 上

数据库只保存稳定事实，不保存 UI 视角。

### 4. 线程切换不改变 turn 的执行语义

切换 thread 只改变：

- `session.active_thread_id`
- 前端事件订阅和展示目标

它不改变 turn 本身的生命周期。

## 选型结论

本次采用下面这套结构，而不是旧文档里的“Thread 持有 TurnDriver + 自己定义状态机 + 自己定义事件协议”：

### 方案 A: Thread 自己重建 turn 状态机

优点：
- 不用改 `turn`

缺点：
- 状态重复
- 事件重复
- prompt/history 重复
- 工具审批逻辑容易出现双写和漂移

### 方案 B: Thread 负责编排，Turn 继续负责单轮执行

优点：
- 与现有 `turn` 边界最一致
- 运行时逻辑最少重复
- 历史与 prompt 组装职责更清晰
- 后台执行可直接复用 `TurnHandle + JoinHandle`

缺点：
- 需要对 `turn` 做少量边界增强

### 方案 C: 全量 actor 化（SessionActor / ThreadActor / TurnActor）

优点：
- 并发边界最清晰

缺点：
- 当前阶段偏重
- Tauri 集成成本更高
- 对现有项目是过早抽象

**结论：采用方案 B。**

## 领域模型与运行时模型

### Session

`Session` 表示用户级隔离边界和跨线程共享上下文。

建议职责：

- 拥有 `session_id`
- 管理 `active_thread_id`
- 管理跨线程共享配置：模型选择、系统提示词、工具授权策略、工作目录/工作区上下文
- 管理 thread 生命周期：创建、归档、删除、列表

建议数据模型：

```rust
pub struct SessionRecord {
    pub id: String,
    pub user_id: Option<String>,
    pub default_model: String,
    pub system_prompt: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

运行时：

```rust
pub struct SessionRuntime {
    pub active_thread_id: Option<Uuid>,
    pub threads: HashMap<Uuid, ThreadRuntime>,
}
```

### Thread

`Thread` 是多轮对话的聚合根。

它负责：

- 聚合该 thread 下的所有 `TurnRecord`
- 维护是否存在活动 turn
- 对接审批流程和前端通知
- 构造下一轮 turn 的历史上下文

`Thread` 不负责：

- 自己执行模型调用
- 自己调度工具
- 自己管理 tool timeout / cancel / permission protocol

建议持久化模型：

```rust
pub enum ThreadLifecycle {
    Open,
    Archived,
}

pub struct ThreadRecord {
    pub id: Uuid,
    pub session_id: String,
    pub title: Option<String>,
    pub lifecycle: ThreadLifecycle,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_turn_number: u32,
}
```

建议运行时模型：

```rust
pub struct ThreadRuntime {
    pub thread_id: Uuid,
    pub active_turn: Option<ActiveTurnRuntime>,
}

pub struct ActiveTurnRuntime {
    pub turn_id: Uuid,
    pub turn_number: u32,
    pub handle: TurnHandle,
    pub task: JoinHandle<Result<TurnOutcome, TurnError>>,
    pub waiting_permission: Option<PermissionRequest>,
}
```

这里的关键点：

- “后台执行”不是一个持久化状态字段
- 是否后台运行由 `thread_id != session.active_thread_id` 派生得到
- `waiting_permission` 是运行时状态，不直接作为 thread 生命周期持久化

### Turn

`Turn` 是 thread 中的一次不可变交互记录。

建议持久化模型：

```rust
pub enum TurnStatus {
    Running,
    WaitingPermission,
    Completed,
    Cancelled,
    Failed,
    Interrupted,
}

pub struct TurnRecord {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub turn_number: u32,
    pub user_input: String,
    pub status: TurnStatus,
    pub finish_reason: Option<String>,
    pub transcript: Vec<PersistedMessage>,
    pub final_output: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}
```

其中：

- `transcript` 是参与下一轮 prompt replay 的真相源
- `final_output` 是便于 UI 列表展示和搜索的冗余索引字段
- 如果未来需要更强查询能力，再从 `transcript` 提炼 tool trace 索引，而不是一开始就维护第二套 `ToolCallRecord` 真值结构

### PersistedMessage

为了让历史直接参与下一轮模型上下文，持久化消息模型必须接近 `turn` 的真实语义，而不是只保留摘要。

```rust
pub enum PersistedMessage {
    User {
        content: String,
    },
    AssistantText {
        content: String,
    },
    AssistantToolCalls {
        content: Option<String>,
        calls: Vec<PersistedToolCall>,
    },
    ToolResult {
        call_id: String,
        tool_name: String,
        content: String,
        is_error: bool,
    },
    SystemNote {
        content: String,
    },
}
```

这份结构是“下一轮上下文回放”的唯一真值，不再额外维护 `assistant_response + tool_calls` 两套平行结构。

## 对 Turn 边界的必要增强

如果坚持完全不改 `turn`，`thread` 层就必须自己做 transcript 构造和历史注入，重复度会很高。

因此设计上建议对 `turn` 增加两个最小能力。

### 增强 1: 支持带历史启动 Turn

建议不要再让 `TurnContext` 只携带 `user_message`，而是允许注入前序历史。

可选形态之一：

```rust
pub struct TurnSeed {
    pub session_id: String,
    pub turn_id: String,
    pub prior_messages: Vec<TurnMessage>,
    pub user_message: String,
}
```

这样 `TurnDriver` 在首轮发起模型请求前，可以先把：

- 历史消息
- 当前用户输入

放入内部 transcript，再统一驱动后续 tool loop。

### 增强 2: Turn 完成时产出最终 transcript

为了避免 `thread` 层根据 `LlmTextDelta` 再拼一次 transcript，`turn` 需要在完成时稳定暴露单轮结果。

建议引入：

```rust
pub struct TurnOutcome {
    pub turn_id: String,
    pub finish_reason: TurnFinishReason,
    pub transcript: Vec<TurnMessage>,
    pub final_output: Option<String>,
}
```

这样：

- UI 仍然通过 `TurnEvent` 获得流式体验
- 持久化层通过 `TurnOutcome` 获得单轮最终产物
- `thread` 层不必自行“复刻 turn transcript builder”

## 总体架构

```text
Frontend (Tauri)
    |
    | commands + emitted events
    v
SessionManager (application service)
    |
    | owns
    +--> SessionRecord / SessionRuntime
    |
    +--> ThreadStore (sessions / threads / turns persistence)
    |
    +--> ThreadRuntimeMap
            |
            +--> ActiveTurnRuntime
                    |
                    +--> TurnHandle
                    +--> JoinHandle<Result<TurnOutcome, TurnError>>
                    +--> TurnObserver bridge

turn crate
    |
    +--> TurnDriver
    +--> TurnEvent
    +--> TurnState
    +--> TurnHandle
    +--> TurnOutcome
```

## 状态设计

### 持久化状态

建议只持久化稳定生命周期：

- `ThreadLifecycle`: `Open | Archived`
- `TurnStatus`: `Running | WaitingPermission | Completed | Cancelled | Failed | Interrupted`

### 运行时展示状态

以下状态由运行时推导，不直接入库：

- `Idle`
- `RunningForeground`
- `RunningBackground`
- `WaitingPermissionForeground`
- `WaitingPermissionBackground`

推导规则：

- 有 `active_turn` 且 `thread_id == session.active_thread_id` -> `Foreground`
- 有 `active_turn` 且 `thread_id != session.active_thread_id` -> `Background`
- 无 `active_turn` -> `Idle`

这比把 `BackgroundProcessing` 写进数据库更稳定。

## 事件设计

### Thread 不再重新定义 Turn 过程事件

建议 thread 级事件只做两件事：

- 声明 thread 生命周期变化
- 用 `thread_id` 包装现有 `TurnEvent`

建议事件模型：

```rust
pub enum ThreadEvent {
    ThreadCreated { thread_id: Uuid },
    ThreadActivated { thread_id: Uuid },
    ThreadUpdated { thread_id: Uuid },
    ThreadArchived { thread_id: Uuid },
    TurnEvent {
        thread_id: Uuid,
        turn_id: Uuid,
        event: TurnEvent,
    },
}
```

这样前端可以：

- 直接复用现有 `TurnEvent` 语义
- 用 `thread_id` 做路由和展示
- 不需要维护第二套事件映射逻辑

## 核心数据流

### 1. 创建 Thread

1. `SessionManager` 创建 `ThreadRecord`
2. 持久化到 store
3. 在 `SessionRuntime` 中注册空的 `ThreadRuntime`
4. 设置为 `active_thread_id`
5. 发出 `ThreadCreated` / `ThreadActivated`

### 2. 在 Thread 中启动新一轮 Turn

1. 从 store 读取该 thread 已完成的历史 turns
2. 将这些历史 turn 的 `transcript` 按顺序扁平化为 `prior_messages`
3. 用 `prior_messages + user_message` 创建 `TurnSeed`
4. `TurnDriver::spawn(...)` 返回 `TurnHandle + JoinHandle`
5. 将其放入 `ThreadRuntime.active_turn`
6. 插入一条 `TurnRecord(status = Running)`
7. 通过 observer / handle 把 `TurnEvent` 包装成 `ThreadEvent::TurnEvent` 发给前端
8. 当任务完成后，用 `TurnOutcome` 回填最终 `TurnRecord`

### 3. 切换 Thread

1. 更新 `session.active_thread_id`
2. 不中断旧 thread 的 `active_turn`
3. 旧 thread 自动从 `RunningForeground` 变成 `RunningBackground`
4. 新 thread 如果无运行任务则显示已有历史；如果本身也有运行任务，则显示其前台运行态

关键点：

- 切换 thread 不触碰 `TurnHandle`
- 不需要显式调用“start background task”
- 后台执行只是 UI 视角变化，不是新的执行机制

### 4. 审批/认证流程

1. `TurnEvent::ToolCallPermissionRequested` 到达
2. `ThreadRuntime.active_turn.waiting_permission = Some(request)`
3. 前端按 `thread_id` 展示审批入口
4. 用户审批后调用 `TurnHandle::resolve_permission(...)`
5. turn 继续运行

关键点：

- 审批协议仍然属于 `turn`
- `thread` 只负责把审批请求和对应 thread 关联起来

### 5. 应用重启

1. 加载 `SessionRecord / ThreadRecord / TurnRecord`
2. 所有 `status in (Running, WaitingPermission)` 的 turn 统一标记为 `Interrupted`
3. 不恢复任何 `ActiveTurnRuntime`
4. 前端展示历史 thread 列表
5. 用户再次发送消息时，仅使用稳定落盘的历史 turn 构造上下文

这满足你的要求：

- 切换 thread 时后台继续跑
- 重启应用后只加载落盘历史，不恢复中途执行

## 历史参与下一轮上下文的规则

### 1. 只有稳定历史参与 prompt replay

默认参与下一轮上下文的 turn：

- `Completed`
- `Cancelled`
- `Failed`

默认不自动参与的 turn：

- `Interrupted`
- 尚未完成但只写入了部分 checkpoint 的 turn

原因：

- 被中断的 partial assistant text 往往语义不完整
- 自动塞回上下文会污染后续推理

### 2. 上下文构造顺序

对某个 thread 构造下一轮 `prior_messages` 时：

1. 按 `turn_number` 升序遍历历史 turn
2. 对每个可回放 turn，按 transcript 内消息顺序追加
3. 最后追加本轮用户输入

这保证：

- thread 是多轮对话
- turn 仍然是单轮执行单元
- prompt replay 与真实交互顺序一致

## 持久化建议

建议至少保留以下三张表：

### `sessions`

保存用户级隔离和跨线程共享配置。

### `threads`

保存 thread 元数据：

- `id`
- `session_id`
- `title`
- `lifecycle`
- `created_at`
- `updated_at`
- `last_turn_number`

### `turns`

保存 turn 级稳定事实：

- `id`
- `thread_id`
- `turn_number`
- `status`
- `finish_reason`
- `user_input`
- `final_output`
- `transcript_json`
- `started_at`
- `finished_at`

设计上建议：

- `transcript_json` 直接存可回放消息序列
- 不再单独维护 `tool_calls` 作为另一套真值表
- 如果将来确实要做统计分析，再从 transcript 派生索引表

## 错误恢复策略

| 场景 | 处理策略 |
|------|----------|
| thread 切换时 turn 仍在运行 | 不处理执行引擎，仅改变活动 thread 指针 |
| 应用关闭或崩溃时 turn 未完成 | 启动后将该 turn 标记为 `Interrupted` |
| tool 权限请求发生在后台 thread | 保留请求并通过 thread 级事件通知 UI |
| 历史读取失败 | 不阻塞应用启动，但该 thread 标记为不可用并记录日志 |
| turn 持久化最终结果失败 | 运行结果仍返回 UI，但 thread 标记为 `degraded` 并提示重试落盘 |

## 为什么这个版本比旧设计更好

### 消除了旧设计中的重复

旧设计的问题：

- `Thread` 持有 `TurnDriver`
- `ThreadState` 混合运行态、后台态、UI 态
- `ThreadEvent` 重新定义了 turn 过程语义
- `TurnRecord + ToolCallRecord + assistant_response` 与真正 prompt 历史并不一致

新设计的改进：

- `Thread` 持有 `TurnHandle + JoinHandle`，与现有 `turn` API 对齐
- “后台执行”变成派生视图，不再是持久化状态
- thread 事件只包装 `TurnEvent`
- 历史只保留一份“可回放 transcript”真相源
- 明确承认 `turn` 需要最小增强，避免在 session/thread 层偷偷复制逻辑

## 最终结论

这次重构的核心不是“在 Turn 上面包一层 Thread”，而是重新划清边界：

- `Turn` 继续做单轮执行引擎
- `Thread` 成为多轮对话聚合根
- `Session` 负责用户级隔离和跨线程共享上下文
- 持久化真相以“可回放 transcript”为中心
- 前后台切换是运行时派生状态，不是新的执行机制

这能满足：

- 多 thread 管理
- thread 切换后台继续跑
- 重启后加载历史
- 历史进入下一轮上下文
- 尽量不重复实现现有 turn 能力
