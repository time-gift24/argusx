# Agent-Center 设计文档

**版本:** 1.1  
**日期:** 2026-03-03  
**作者:** ArgusX Team

## 执行摘要

本文定义 `agent-center` crate 的可落地设计，用于在现有 `agent-core` / `agent-session` / `agent-tool` 之上提供多 agent 协作能力，并以“先安全、后功能”为上线原则。

**目标能力：**
- Agent 注册与管理：从 `.agents/*.toml` 加载并持久化定义
- Thread 控制面：`spawn_agent` / `wait` / `close_agent`
- 权限边界：深度限制、工具白名单、继承与收窄
- 可靠性：并发配额、幂等、重试、崩溃恢复
- 可观测性：结构化事件、关键指标、可追溯审计

**Go/No-Go 原则（阻断项）：**
在以下能力未完成前，`spawn_agent` 不允许对外启用：
1. 深度/并发 Guard 与拒绝路径
2. `spawn` 幂等键与去重
3. `wait` 超时钳制与无忙轮询
4. 启动时 Thread 状态对账恢复

## 非目标

- 不改动 `agent-turn` 的核心 reducer 协议
- 不在首版引入跨机器分布式调度
- 不在首版支持动态创建全新工具类型（仅复用已注册工具）

## 术语与一致性约束

### 1. Session 与 Thread 的统一语义

为消除歧义，统一约束如下：
- **Thread**：`agent-center` 的并发控制实体
- **Session**：`agent-session` 的会话实体
- **一个 Thread 对应一个独立 Session**（用于历史与状态隔离）
- 父子关系通过 `parent_thread_id` 与 `parent_session_id` 显式记录
- “共享”仅指共享模型适配器、工具运行时与进程资源，不共享会话历史

### 2. 核心不变式（必须满足）

1. `depth(child) = depth(parent) + 1`
2. `depth <= max_depth`，超限必须拒绝 spawn
3. 任一时刻 `running_threads <= max_concurrent`
4. 每个 thread 最终进入且只进入一个终态：`succeeded|failed|cancelled|closed`
5. 同一幂等键的 `spawn` 必须返回同一 `thread_id`
6. 权限只能继承后收窄，不能在子线程放大

## 架构分层

```text
Desktop UI (Tauri + Next.js)
  └── agent (Facade)
      └── agent-center (NEW: thread control plane)
          ├── registry (agent defs)
          ├── scheduler (guards + lifecycle)
          ├── control tools (spawn/wait/close)
          └── persistence (sqlite)
              └── agent-session (SessionRuntime)
                  └── agent-tool (ToolRegistry)
                      └── agent-core (traits/events)
```

## 模块结构

```text
agent-center/
├── src/
│   ├── lib.rs
│   ├── error.rs
│   ├── api/
│   │   └── center.rs
│   ├── core/
│   │   ├── registry.rs
│   │   ├── scheduler.rs
│   │   ├── lifecycle.rs
│   │   └── reconciler.rs
│   ├── permission/
│   │   ├── mod.rs
│   │   ├── context.rs
│   │   ├── ruleset.rs
│   │   ├── guard.rs
│   │   └── evaluator.rs
│   ├── persistence/
│   │   ├── mod.rs
│   │   ├── store.rs
│   │   ├── migrations.rs
│   │   └── models.rs
│   ├── tools/
│   │   ├── mod.rs
│   │   ├── spawn.rs
│   │   ├── wait.rs
│   │   └── close.rs
│   └── config/
│       ├── mod.rs
│       ├── loader.rs
│       ├── validator.rs
│       └── watcher.rs
```

## 与现有工程的最小侵入边界

### 保持不变

- `agent-core::Runtime` 主协议不变
- `agent-turn` 事件驱动执行核心不变
- `agent-tool` 现有工具注册与执行接口不破坏

### 必需新增（最小集合）

1. 在 `agent-center` 内维护 `session_id -> thread_runtime_context` 映射，用于在不改 `agent-core` 协议的前提下做权限/深度判定。
2. `spawn_agent` 必须要求并记录幂等键（默认取当前 tool call id）。
3. `agent` facade 在初始化时注入 `AgentCenter`，并注册 `spawn_agent`/`wait`/`close_agent` 三个工具。

## 核心数据模型

```rust
pub struct AgentDefinition {
    pub name: String,
    pub description: String,
    pub version: String,
    pub prompt: String,
    pub tools: Vec<String>,
    pub permissions: PermissionConfig,
    pub limits: LimitsConfig,
}

pub struct Thread {
    pub thread_id: String,
    pub session_id: String,
    pub parent_thread_id: Option<String>,
    pub parent_session_id: Option<String>,
    pub agent_name: String,
    pub depth: u32,
    pub status: ThreadStatus,
    pub permission_context: PermissionContext,
    pub idempotency_key: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub ended_at_ms: Option<i64>,
}

pub enum ThreadStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Closing,
    Closed,
}
```

## Thread 状态机（强约束）

```text
Pending -> Running
Running -> Succeeded | Failed | Cancelled | Closing
Closing -> Closed | Failed
(任何终态不可回退)
```

状态迁移规则：
- `spawn_agent` 创建时为 `Pending`，首个有效执行事件后置为 `Running`
- `close_agent(force=false)`：`Running -> Closing -> Closed`
- `close_agent(force=true)`：可直接尝试 `Running -> Closed`（记录 `forced=true`）
- 终态线程只能被 `wait` 查询，不能再次 `spawn` 或 `send_input`

## 工具语义合同

### `spawn_agent`

输入：
- `agent`: agent 名称
- `task`: 首条任务输入
- `idempotency_key`: 可选；默认 `tool_call_id`

行为：
1. 校验 agent 存在、权限合法、深度/并发未超限
2. 用 `(parent_thread_id, idempotency_key)` 去重
3. 创建子 session + thread 记录（事务）
4. 返回 `thread_id`, `session_id`, `status`

失败语义：
- 超限：`resource_exhausted`
- 权限拒绝：`permission_denied`
- 参数错误：`invalid_argument`
- 暂态错误：`transient_error`（允许受控重试）

### `wait`

输入：
- `ids: [thread_id]`
- `timeout_ms`（默认 30000，钳制区间 `[1000, 300000]`）
- `mode`：`any`（默认）或 `all`

语义：
- `any`：任意一个到终态即返回
- `all`：全部到终态才返回，否则超时
- 无忙轮询：基于状态订阅/通知或条件变量

返回：
- `statuses: {thread_id -> status}`
- `timed_out: bool`

### `close_agent`

输入：
- `id: thread_id`
- `force: bool = false`
- `grace_ms: u64 = 5000`

语义：
- 幂等：重复关闭返回同一终态快照
- 首先尝试优雅关闭，超时且 `force=true` 时强制关闭
- 关闭后保留 tombstone 记录（默认 24h）用于审计与幂等

## 权限模型

`PermissionContext` 包含：
- `allowed_tools`
- `max_depth`
- `max_concurrent`
- `inherit`（允许继承）

规则：
1. 子线程权限 = 父权限 ∩ 子 agent 声明权限（只能收窄）
2. 明确禁止“子线程开启父线程不可用工具”
3. 若 `depth + 1 == max_depth`，自动禁用再次 `spawn_agent`

## 错误分类与恢复策略

错误分层：
- `InvalidArgument`（不重试）
- `PermissionDenied`（不重试）
- `NotFound`（不重试）
- `ResourceExhausted`（可短暂重试，受预算控制）
- `Transient`（指数退避 + 抖动）
- `Internal`（告警，不自动无限重试）

重试策略：
- 最大重试次数：3
- 退避：`base=200ms`, `factor=2`, `jitter=0~20%`
- 单线程重试预算：10s
- 无幂等键的请求不得自动重试

## 持久化设计（SQLite）

```sql
CREATE TABLE agent_definitions (
  name TEXT PRIMARY KEY,
  description TEXT NOT NULL,
  version TEXT NOT NULL,
  prompt TEXT NOT NULL,
  tools_json TEXT NOT NULL,
  permissions_json TEXT NOT NULL,
  limits_json TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

CREATE TABLE threads (
  thread_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  parent_thread_id TEXT,
  parent_session_id TEXT,
  agent_name TEXT NOT NULL,
  depth INTEGER NOT NULL CHECK(depth >= 0),
  status TEXT NOT NULL,
  idempotency_key TEXT NOT NULL,
  permission_context_json TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL,
  ended_at_ms INTEGER,
  FOREIGN KEY(parent_thread_id) REFERENCES threads(thread_id)
);

CREATE TABLE spawn_dedup (
  parent_thread_id TEXT NOT NULL,
  idempotency_key TEXT NOT NULL,
  thread_id TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  PRIMARY KEY(parent_thread_id, idempotency_key),
  FOREIGN KEY(thread_id) REFERENCES threads(thread_id)
);

CREATE INDEX idx_threads_status ON threads(status);
CREATE INDEX idx_threads_parent ON threads(parent_thread_id);
CREATE INDEX idx_threads_session ON threads(session_id);
```

## 崩溃恢复与启动对账

应用启动时执行 `reconcile()`：
1. 扫描 `status IN (Pending, Running, Closing)` 的线程
2. 对每个线程检查对应 session/turn 是否仍活动
3. 若已无活动执行，则标记为 `Failed` 或 `Closed`（带 `reconcile_reason`）
4. 释放并发配额计数并重建内存索引
5. 输出对账摘要日志与指标

要求：
- 对账过程幂等，可重复执行
- 对账期间禁止新 spawn（短暂写锁）

## 配置与热重载

- 配置源：`.agents/*.toml`
- 热重载采用“校验通过后整体切换 generation”
- 校验失败不影响当前生效配置
- 运行中线程不因热重载被放大权限

## 可观测性

最小指标集：
- `agent_center_threads_running`
- `agent_center_spawn_total{result}`
- `agent_center_wait_latency_ms`
- `agent_center_close_total{result}`
- `agent_center_reconcile_total{result}`

日志字段：
- `thread_id`, `parent_thread_id`, `session_id`, `agent_name`, `depth`, `idempotency_key`, `error_kind`

## 示例配置（修正版）

```toml
[agent]
name = "explorer"
description = "Fast agent specialized for exploring codebases"
version = "1.0.0"
prompt = """
You are a fast agent specialized for exploring codebases.
Use glob, grep, and read tools to quickly find information.
Always be thorough while balancing depth and workload.
"""

tools = ["read", "grep", "glob", "ls"]

[agent.permissions]
inherit = true

[agent.limits]
max_depth = 2
max_concurrent = 3
```

## 集成方式（修正版）

```rust
let center = AgentCenter::builder()
    .model_provider(model_provider)
    .tool_registry(tool_registry.clone())
    .store_path(".agent/center.db")
    .agents_dir(".agents")
    .build()
    .await?;

center.register_builtin_tools(&tool_registry).await?;
```

## 测试矩阵（必须）

1. 单元测试
- 权限收窄与拒绝路径
- 深度/并发 Guard
- 状态机非法迁移拒绝

2. 集成测试
- `spawn -> wait(any) -> close` 主流程
- 幂等 spawn 重放
- `wait(all)` 超时与返回语义

3. 故障测试
- spawn 途中崩溃后启动对账
- DB 写失败回滚一致性
- 热重载配置损坏回退

4. 压力测试
- 高并发 spawn/close 下无配额泄漏
- 长时间运行无线程状态悬挂

## 分期计划（调整后）

### Phase 0（1-2 天，阻断前置）
- 深度/并发 Guard + 拒绝路径
- `spawn` 幂等键与 `spawn_dedup`
- 基础状态机与终态不回退约束

### Phase 1（3-4 天，最小可用）
- AgentRegistry + ConfigLoader + 校验器
- `spawn_agent` / `wait` / `close_agent`
- SQLite 持久化与事务边界
- 启动对账 `reconcile()`

### Phase 2（2-3 天，增强可靠性）
- 热重载 generation 切换
- 指标与结构化日志
- 错误预算与重试参数调优

## 风险清单与缓解

1. **风险：** 子线程递归爆炸  
   **缓解：** max_depth + 在边界层自动禁用 spawn

2. **风险：** 并发配额泄漏  
   **缓解：** RAII reservation + reconcile 兜底回收

3. **风险：** 重试放大故障  
   **缓解：** 仅 transient 可重试 + 预算上限 + jitter

4. **风险：** 热重载引入越权  
   **缓解：** 仅新线程应用新配置，运行线程权限不放大

## 最终结论

该版本将“鲁棒性机制”从增强项提升为首发阻断门槛，明确了：
- 一致的 session/thread 语义
- 完整的状态机与工具合同
- 幂等与恢复策略
- 可执行的持久化与对账流程

在按 Phase 0 + Phase 1 完成并通过测试矩阵前，不应启用多 agent 生产流量。
