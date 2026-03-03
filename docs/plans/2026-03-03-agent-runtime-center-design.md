# Agent Runtime Center Design

## 1. 背景

当前 desktop 后端在 `AppState` 内持有单个 `SessionRuntime`，前端会话通过 `frontend_to_backend_session` 映射到后端 session。该模型在普通失败场景可工作，但对“进程级崩溃”隔离能力不足：一旦 runtime/worker 进程异常，正在执行的 turn 会中断，且缺乏统一池化调度与自动替换能力。

## 2. 目标

1. 新增 crate `agent-runtime-center`，作为运行时 agent 管理中心（池化 + 调度 + 恢复）。
2. 采用混合池策略：常驻共享池 + 按需隔离池。
3. 发生 worker 级崩溃后自动替换，并从最近 checkpoint 自动重试一次。
4. 若重试仍失败，turn 标记为 `failed`，并返回清晰失败原因（崩溃原因 + 重试失败原因）。
5. `subagent` 调用视为 tool 执行，且强制走 `IsolatedPool`。

## 3. 非目标

1. 本阶段不改造前端交互模型（保持现有 `start_agent_turn`/stream 事件形态）。
2. 不引入无限次自动重试（仅一次自动重试）。
3. 不在本设计中变更业务 prompt/tool 参数语义（重试元信息仅在后端执行上下文传播）。

## 4. 已确认决策

1. 主目标是“进程级异常隔离 + 自动替换”。
2. 崩溃恢复策略：从最近 checkpoint 自动重试一次；重试失败则标记 `failed` 并说明原因。
3. 池策略：混合池（常驻 + 短时突发扩容）。
4. crate 名称固定为 `agent-runtime-center`。
5. 交互策略采用 A（规则引擎 + 最小交互）：高风险/长任务自动分流，提权时才请求用户确认。
6. `subagent` 调用本质是 tool 调用，且固定走 `IsolatedPool`。

## 5. 方案对比

### 方案 A（采用）：混合池 + 自动恢复

- `SharedPool`（in-proc）处理普通任务，`IsolatedPool`（subprocess）处理高风险/长任务/subagent。
- 崩溃后替换 worker 并执行一次 checkpoint 恢复重试。

优点：

1. 兼顾低延迟与崩溃隔离。
2. 能在现有架构上渐进迁移，风险可控。
3. 支持后续扩展更细粒度调度策略。

缺点：

1. 调度、可观测、恢复链路复杂度高于单池。

### 方案 B：全隔离进程池

优点：隔离最强。  
缺点：所有任务都承担进程与 IPC 成本，短任务吞吐与时延变差。

### 方案 C：单 runtime 加崩溃重建

优点：改动最小。  
缺点：不是真正的池，扩展性与隔离能力不足。

## 6. 推荐架构

```text
Frontend
  -> start_agent_turn
Desktop(Tauri)
  -> AgentRuntimeCenter
      -> Classifier
      -> Scheduler
      -> SharedPool(min=2, in-proc)
      -> IsolatedPool(max=8, subprocess, burst)
      -> RecoveryCoordinator
      -> SessionAffinityMap
      -> Telemetry
```

关键语义：

1. 默认路由到 `SharedPool`，命中规则时路由到 `IsolatedPool`。
2. `subagent` 无条件进入 `IsolatedPool`。
3. worker 崩溃时拉起替代 worker，并触发恢复重试流程。
4. 流式输出面向前端保持同一 turn 语义，不泄漏内部重试实现细节。

## 7. 组件设计

### 7.1 Classifier

输入：

1. `TurnRequestMeta`（权限策略、工具类型、历史统计、`is_subagent`）。
2. 环境约束（当前池负载、策略阈值）。

输出：

- `RoutingDecision::{Shared, Isolated}` + `reason`。

优先级：

1. `is_subagent == true` -> `Isolated`
2. `high_risk == true` -> `Isolated`
3. `long_task == true` -> `Isolated`
4. otherwise -> `Shared`

### 7.2 Scheduler

1. 维护队列和容量。
2. 常驻 `SharedPool` 2 个 worker。
3. `IsolatedPool` 可突发扩容到 8，空闲回收。
4. 超阈值时回压（排队或拒绝策略可配置，默认排队）。

### 7.3 Pool Abstraction

统一 `WorkerHandle` 接口：

1. `run_turn(request) -> RuntimeStreams`
2. `cancel_turn(turn_id)`
3. `inject_input(turn_id, input)`
4. `health()`

实现：

1. `SharedPoolWorker`：进程内 runtime 实例。
2. `IsolatedPoolWorker`：子进程 runtime（IPC 流式转发）。

### 7.4 RecoveryCoordinator

职责：

1. 检测 worker 崩溃或不可恢复中断。
2. 拉起替代 worker。
3. 从 checkpoint 恢复并自动重试一次。
4. 汇总失败原因并形成统一错误返回。

### 7.5 SessionAffinity

1. 非 subagent turn 尽量粘在同 worker，减少上下文抖动。
2. subagent 默认无粘性，以隔离优先。
3. affinity worker 不健康时可迁移。

### 7.6 Telemetry

核心指标：

1. `runtime_center_queue_depth`
2. `runtime_center_pool_in_use{pool=shared|isolated}`
3. `runtime_center_worker_crash_total`
4. `runtime_center_retry_total{result=success|failed}`
5. `runtime_center_turn_latency_ms_p95`

## 8. 路由与判定规则

### 8.1 高风险判定

命中任一即可：

1. 请求 `sandbox_permissions=require_escalated`。
2. 包含写/删/外部网络等高风险工具调用意图。
3. 同类任务历史 crash 率超过阈值。

### 8.2 长任务判定

命中任一即可：

1. 历史 `p95` 耗时超过阈值（如 90s）。
2. 预计工具调用次数超过阈值（如 8）。

### 8.3 用户交互策略（A）

1. 默认自动分流，无额外交互噪音。
2. 仅当请求提权时进行用户确认。

## 9. 崩溃恢复与一次重试

状态流：

```text
Queued -> Running -> Done
Queued -> Running -> Recovering -> Running(retry) -> Done
Queued -> Running -> Recovering -> Failed
Queued -> Running -> Cancelled
```

恢复流程：

1. 运行中检测到 worker 崩溃。
2. 标记 turn `Recovering`，记录 `crash_cause`。
3. 拉起替代 worker。
4. 以最近 checkpoint 恢复并重试一次（`retry_attempt=1`）。
5. 若成功，正常完成。
6. 若失败，`failed`，错误体包含：
   - `crash_cause`
   - `retry_cause`

## 10. Subagent 作为 Tool 的语义

1. `subagent` 通过标准 tool 路径调用（例如 `subagent.execute`）。
2. 首次与重试使用同一份业务参数（args 不改写）。
3. 重试元信息通过执行上下文传递，不注入业务参数：
   - `retry_attempt`
   - `retry_reason`
   - `parent_turn_id`
   - `tool_call_id`（稳定关联）
4. `subagent` 固定路由 `IsolatedPool`。
5. subagent 重试后仍失败，返回结构化 tool error 给父 turn；父 turn 默认 `fail-fast`。

## 11. Desktop 集成改造

`desktop/src-tauri/src/lib.rs` 改造方向：

1. `AppState.runtime` 从 `SessionRuntime` 替换为 `AgentRuntimeCenter` 句柄。
2. `start_agent_turn` 改为调用 center 的 `start_turn`，其余命令保持语义兼容：
   - `cancel_agent_turn`
   - `restore_turn_checkpoint`
   - `inject_input`（若后续接入）
3. 保留对前端的统一 stream envelope 机制，确保事件序列单调。

## 12. 加载优化

1. 常驻小规模共享池，避免每次 turn 冷启动。
2. 隔离池按需拉起并带空闲回收，降低常驻资源成本。
3. 可选 warmup：提前预热高频模型客户端/工具运行时。
4. 基于历史统计进行路由前置判定，减少错误池命中。

## 13. 错误模型

新增统一错误类别（示意）：

1. `WorkerCrashed`
2. `CheckpointNotFound`
3. `RetryFailed`
4. `PoolExhausted`
5. `DispatchRejected`

turn 最终对前端仍映射为 `failed/cancelled/done`，并在 `failed` 附加结构化 `details`。

## 14. 测试与验收

### 14.1 单元测试

1. classifier 路由优先级（含 `subagent` 强制隔离）。
2. scheduler 扩缩容与队列行为。
3. recovery coordinator 一次重试边界行为。

### 14.2 集成测试

1. worker 崩溃后自动替换并恢复成功。
2. worker 崩溃后重试失败，最终 `failed` 且原因完整。
3. subagent tool 调用始终走 `IsolatedPool`。
4. 提权请求触发确认流程，其余自动分流。

### 14.3 端到端验收

1. 前端无需改交互即可接收连续事件流。
2. 对同一 session 连续 turn，普通任务延迟无明显回归。
3. 压测下隔离池可突发扩容并在空闲后回收。

## 15. 风险与缓解

1. IPC 与流式事件重排序风险：保留中心统一序列号分配。
2. checkpoint 一致性风险：恢复前校验 checkpoint 完整性与 turn 归属。
3. 扩缩容抖动风险：引入最小存活时间与冷却窗口。
4. 复杂度上升：通过模块边界和可观测指标控制维护成本。

## 16. 交付物

1. 新 crate：`agent-runtime-center`（中心调度与池化恢复能力）。
2. desktop 接入 center 的 runtime 调用链。
3. 混合池策略（常驻 2 + 突发到 8）。
4. 崩溃恢复一次重试机制（checkpoint-based）。
5. subagent tool 的隔离执行与重试上下文传播。

---

Created: 2026-03-03  
Status: Approved
