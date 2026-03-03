# Codex Subagent 派发与可靠性设计梳理

## 1. 目标与范围

本文聚焦 `.vendor/codex` 中 **thread-spawn 协作子代理（subagent）** 的实现机制，回答两个问题：

1. subagent 是如何被派发、调度和管理的？
2. 代码里做了哪些可靠性（稳定性/可恢复性/可观测性）设计？

不覆盖 UI 展示细节与模型提示词策略，仅覆盖核心 Rust 控制面与关键协议事件。

---

## 2. 总体架构（控制面）

主链路可概括为：

`Model function call` -> `ToolRouter` -> `MultiAgentHandler` -> `AgentControl` -> `ThreadManager` -> `Codex::spawn`

关键入口：

- 工具规格注册（仅 `Feature::Collab` 开启时注入）：
  - `.vendor/codex/codex-rs/core/src/tools/spec.rs:1600`
- 工具调用路由：
  - `.vendor/codex/codex-rs/core/src/tools/router.rs:143`
- 多代理处理器：
  - `.vendor/codex/codex-rs/core/src/tools/handlers/multi_agents.rs:52`
- 会话级代理控制器：
  - `.vendor/codex/codex-rs/core/src/agent/control.rs:45`
- 线程生成与注册：
  - `.vendor/codex/codex-rs/core/src/thread_manager.rs:428`
  - `.vendor/codex/codex-rs/core/src/thread_manager.rs:520`

---

## 3. subagent 派发流程（按工具语义）

### 3.1 `spawn_agent`

1. 解析参数（`message/items/agent_type`），校验输入互斥关系与非空。
2. 计算 `child_depth`，超深度直接拒绝。
3. 发送 `CollabAgentSpawnBeginEvent`。
4. 复制父 turn 的模型、provider、reasoning、sandbox、cwd、developer/base 指令等配置，应用角色配置。
5. 强制子代理 `approval_policy = Never`，并在达到下一层深度边界时关闭其 `Collab` 能力。
6. 调用 `AgentControl::spawn_agent(...)` 真正创建线程并投递首条输入。
7. 发送 `CollabAgentSpawnEndEvent`（含 `new_thread_id`、昵称、角色、状态）。

关键代码：

- `.vendor/codex/codex-rs/core/src/tools/handlers/multi_agents.rs:114`
- `.vendor/codex/codex-rs/core/src/tools/handlers/multi_agents.rs:884`
- `.vendor/codex/codex-rs/core/src/tools/handlers/multi_agents.rs:931`
- `.vendor/codex/codex-rs/core/src/agent/control.rs:55`

### 3.2 `send_input`

1. 解析 `id`，可选 `interrupt`。
2. `interrupt=true` 时先发 `Op::Interrupt`。
3. 发送 `CollabAgentInteractionBeginEvent`。
4. 调用 `AgentControl::send_input`（底层 `Op::UserInput`）。
5. 获取最新状态并发送 `CollabAgentInteractionEndEvent`。

关键代码：

- `.vendor/codex/codex-rs/core/src/tools/handlers/multi_agents.rs:235`
- `.vendor/codex/codex-rs/core/src/agent/control.rs:172`

### 3.3 `resume_agent`

1. 仅当目标 agent `NotFound` 时尝试从 rollout 恢复。
2. 恢复时重建 `ThreadSpawn` 来源，并尝试从 sqlite 回填 `agent_nickname/agent_role`。
3. 恢复后重新注册线程并通知创建事件。

关键代码：

- `.vendor/codex/codex-rs/core/src/tools/handlers/multi_agents.rs:325`
- `.vendor/codex/codex-rs/core/src/agent/control.rs:105`

### 3.4 `wait`

1. 校验 `ids` 非空，`timeout_ms > 0`。
2. timeout 会被 clamp 到 `[10s, 300s]`，默认 `30s`。
3. 先看初始状态是否已 final，否则并发订阅状态流，等待首个 final 或超时。
4. 返回 `status` 映射和 `timed_out` 标记，并发出 `CollabWaitingEndEvent`。

关键代码：

- `.vendor/codex/codex-rs/core/src/tools/handlers/multi_agents.rs:42`
- `.vendor/codex/codex-rs/core/src/tools/handlers/multi_agents.rs:472`

### 3.5 `close_agent`

1. 先订阅当前状态。
2. 若未关停则发送 `Op::Shutdown`。
3. 发送 `CollabCloseEndEvent` 并返回关闭前状态。

关键代码：

- `.vendor/codex/codex-rs/core/src/tools/handlers/multi_agents.rs:665`
- `.vendor/codex/codex-rs/core/src/agent/control.rs:200`

---

## 4. 可靠性设计（重点）

### 4.1 并发与资源上限控制

- 会话级共享 `Guards` 控制 subagent 并发数（默认最大线程数 6）。
- `SpawnReservation` 使用 RAII：占位后若 spawn 中途失败，`Drop` 自动归还配额，避免泄漏。

代码位置：

- `.vendor/codex/codex-rs/core/src/agent/guards.rs:21`
- `.vendor/codex/codex-rs/core/src/agent/guards.rs:149`
- `.vendor/codex/codex-rs/core/src/config/mod.rs:117`
- `.vendor/codex/codex-rs/core/src/config/mod.rs:1761`

### 4.2 递归深度保护

- `agents.max_depth` 默认 1。
- `spawn/resume` 前做深度检查，超限拒绝。
- 额外防护：对子代理配置，如果再下一层会超限，则禁用其 `Collab`，防止无限派发。

代码位置：

- `.vendor/codex/codex-rs/core/src/agent/guards.rs:42`
- `.vendor/codex/codex-rs/core/src/tools/handlers/multi_agents.rs:130`
- `.vendor/codex/codex-rs/core/src/tools/handlers/multi_agents.rs:931`
- `.vendor/codex/codex-rs/core/src/codex.rs:325`

### 4.3 `wait` 防忙轮询与 CPU 保护

- 对超短 `timeout_ms` 做最小值钳制（10 秒），避免 orchestrator 频繁短轮询导致 CPU 空转。

代码位置：

- `.vendor/codex/codex-rs/core/src/tools/handlers/multi_agents.rs:504`

### 4.4 线程生命周期清理

- `send_input` 若遇到 `InternalAgentDied`，会移除线程并释放 slot。
- `shutdown_agent` 无论结果如何都尝试 remove + release，防止僵尸占位。

代码位置：

- `.vendor/codex/codex-rs/core/src/agent/control.rs:187`
- `.vendor/codex/codex-rs/core/src/agent/control.rs:201`

### 4.5 恢复能力与身份连续性

- 恢复从 rollout 文件定位线程。
- `agent_nickname/agent_role` 进入 `SessionMeta`、rollout、sqlite schema，resume 时可回填。

代码位置：

- `.vendor/codex/codex-rs/core/src/agent/control.rs:149`
- `.vendor/codex/codex-rs/core/src/rollout/recorder.rs:387`
- `.vendor/codex/codex-rs/state/migrations/0013_threads_agent_nickname.sql:1`
- `.vendor/codex/codex-rs/core/src/rollout/metadata.rs:50`

### 4.6 父线程完成通知（无需显式 wait）

- child 完成后由 watcher 注入 `<subagent_notification>` 到 parent 历史（user message）。
- 该消息被识别为 session prefix，不会误判成新的用户意图边界。

代码位置：

- `.vendor/codex/codex-rs/core/src/agent/control.rs:258`
- `.vendor/codex/codex-rs/core/src/session_prefix.rs:11`
- `.vendor/codex/codex-rs/core/src/event_mapping.rs:47`
- `.vendor/codex/codex-rs/core/src/context_manager/history.rs:552`

### 4.7 错误隔离与对模型可恢复反馈

- `ToolRouter` 对非 Fatal 错误统一回传 `FunctionCallOutput(success=false)`，避免直接终止整轮。
- `multi_agents` 内部将底层错误映射成稳定的“对模型可读”错误语义（例如 `not found`、`is closed`）。

代码位置：

- `.vendor/codex/codex-rs/core/src/tools/router.rs:183`
- `.vendor/codex/codex-rs/core/src/tools/handlers/multi_agents.rs:796`

### 4.8 事件持久化策略

- 协作工具 `*End` 事件持久化为 `Extended`，`*Begin` 不持久化。
- 取舍：保留结果与状态证据，减少日志噪音与体积。

代码位置：

- `.vendor/codex/codex-rs/core/src/rollout/policy.rs:119`
- `.vendor/codex/codex-rs/core/src/rollout/policy.rs:168`

### 4.9 委托线程取消与审批回流

- `codex_delegate` 在取消时执行 `Interrupt + Shutdown + drain`，避免后台悬挂。
- 子线程审批请求（exec/apply_patch/request_user_input）回流到父会话处理，取消时默认 `Abort` / 空响应，防阻塞。

代码位置：

- `.vendor/codex/codex-rs/core/src/codex_delegate.rs:272`
- `.vendor/codex/codex-rs/core/src/codex_delegate.rs:336`
- `.vendor/codex/codex-rs/core/src/codex_delegate.rs:438`

---

## 5. 测试证据（已存在）

关键测试覆盖点：

- collab 工具注册可见性：
  - `.vendor/codex/codex-rs/core/src/tools/spec.rs:1865`
- `wait` 参数校验、最小超时钳制、超时与 final 状态返回：
  - `.vendor/codex/codex-rs/core/src/tools/handlers/multi_agents.rs:1606`
  - `.vendor/codex/codex-rs/core/src/tools/handlers/multi_agents.rs:1752`
  - `.vendor/codex/codex-rs/core/src/tools/handlers/multi_agents.rs:1787`
- 并发上限与 slot 回收：
  - `.vendor/codex/codex-rs/core/src/agent/control.rs:677`
  - `.vendor/codex/codex-rs/core/src/agent/control.rs:837`
- parent 自动收到 subagent 完成通知（即使未 `wait`）：
  - `.vendor/codex/codex-rs/core/tests/suite/subagent_notifications.rs:173`
- subagent header 注入（`x-openai-subagent`）：
  - `.vendor/codex/codex-rs/codex-api/src/requests/headers.rs:13`
  - `.vendor/codex/codex-rs/core/tests/responses_headers.rs:27`

---

## 6. 现状边界与注意点

1. `wait` 语义是“返回至少一个 final 或超时”，不是“等待全部完成”；上层编排需循环调用。
2. `close_agent` 返回的是关闭前快照状态，调用后线程会被移除，后续状态通常为 `NotFound`。
3. 深度默认值较保守（1），若要多层协作需显式提高 `agents.max_depth`。
4. 多代理能力本身受 feature gate 控制；生产环境是否可用取决于配置。

---

## 7. 快速结论

`.vendor/codex` 的 subagent 设计是一个以 `ToolRouter + MultiAgentHandler + AgentControl` 为核心的会话级控制面：  
在能力边界（threads/depth）、生命周期清理、状态传播、恢复与错误隔离上都有明确工程化实现，且有对应测试覆盖关键行为。
