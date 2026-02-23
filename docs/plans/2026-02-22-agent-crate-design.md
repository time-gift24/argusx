# Agent Crate 设计文档

**日期**: 2026-02-22

## 目标

重构现有代码并创建新的 `agent` crate，提供统一的 Facade API，让 desktop 和 CLI 都能方便调用。

## 背景与问题

当前项目结构：
- **agent-core**: 核心接口定义（`Runtime`、`LanguageModel`、`Session` 等）
- **agent-turn**: 单轮对话运行时（`TurnEngine` + `TurnRuntime`）
- **agent-session**: 会话管理 + 持久化 + 组合 `TurnRuntime`

当前主要问题：
1. `agent-session` 中混入 Runtime 协调逻辑，职责边界不清晰
2. 缺少统一入口，调用方需要自己拼装 session + runtime + checkpoint
3. 现有存储契约没有把「会话元数据」和「turn 摘要索引」表达完整
4. transcript 脱离 session 后，缺少 `session_id -> latest_turn_id` 的确定性恢复路径

## 设计原则

1. 单一职责：`agent-turn` 只处理单轮执行；`agent-session` 只处理会话元数据和 turn 索引；`agent` 做 Facade 编排。
2. 契约完整：Store trait 必须覆盖 manager 真实需要的行为，避免「接口能编译但语义不闭合」。
3. 恢复可预测：任意 `chat(session_id, ...)` 都能确定性找到上轮 transcript。
4. 并发语义明确：同一 session 同时只允许 1 个 active turn（v1 单进程保证）。

## 设计方案

### 1. 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                           agent                              │
│  (Facade: 组合 SessionManager + TurnRuntime + checkpoint)    │
│  - chat() / chat_stream()                                    │
│  - 会话管理 API                                               │
│  - transcript 恢复与 turn 收尾持久化                          │
└─────────────────────────────────────────────────────────────┘
         │                                    │
         ▼                                    ▼
┌─────────────────────┐          ┌─────────────────────────┐
│   agent-session     │          │      agent-turn          │
│  (会话元数据 + 摘要索引) │       │   (纯单轮执行引擎)        │
│ - SessionManager    │          │ - TurnRuntime<L, T>     │
│ - SessionStore      │          │ - Runtime impl           │
│ - FileSessionStore  │          │ - CheckpointStore        │
└─────────────────────┘          └─────────────────────────┘
         │                                    │
         └──────────────┬─────────────────────┘
                        ▼
              ┌─────────────────────┐
              │     agent-core      │
              │ (核心接口与类型定义)  │
              └─────────────────────┘
```

### 2. agent-session（纯会话管理 + turn 摘要索引）

#### 职责
- 会话元数据管理（创建、删除、列表、状态更新）
- `TurnSummary` 持久化
- 提供 `latest_turn_id` 索引，支持 transcript 恢复

#### 数据模型修订

在 `SessionInfo` 增加：

```rust
pub struct SessionInfo {
    // existing fields...
    pub latest_turn_id: Option<TurnId>,
}
```

#### Store 契约（修订）

```rust
#[async_trait]
pub trait SessionMetaStore: Send + Sync {
    async fn create_session(&self, info: &SessionInfo) -> Result<()>;
    async fn get_session(&self, session_id: &SessionId) -> Result<Option<SessionInfo>>;
    async fn update_session(&self, info: &SessionInfo) -> Result<()>;
    async fn delete_session(&self, session_id: &SessionId) -> Result<()>;
    async fn list_sessions(&self, filter: SessionFilter) -> Result<Vec<SessionInfo>>;
}

#[async_trait]
pub trait TurnSummaryStore: Send + Sync {
    async fn save_turn_summary(&self, session_id: &SessionId, summary: &TurnSummary) -> Result<()>;
    async fn list_turn_summaries(&self, session_id: &SessionId) -> Result<Vec<TurnSummary>>;
    async fn latest_turn_id(&self, session_id: &SessionId) -> Result<Option<TurnId>>;
}

pub trait SessionStore: SessionMetaStore + TurnSummaryStore {}
impl<T> SessionStore for T where T: SessionMetaStore + TurnSummaryStore {}
```

#### SessionManager（修订）

`SessionManager` 不再做泛型 + 双层 `Arc`，直接持有 trait object：

```rust
pub struct SessionManager {
    store: Arc<dyn SessionStore>,
}

impl SessionManager {
    pub fn new(store: Arc<dyn SessionStore>) -> Self;

    pub async fn create_session(
        &self,
        user_id: Option<String>,
        title: Option<String>,
    ) -> Result<SessionId>;

    pub async fn list_sessions(&self, filter: SessionFilter) -> Result<Vec<SessionInfo>>;
    pub async fn get_session(&self, session_id: &SessionId) -> Result<Option<SessionInfo>>;
    pub async fn delete_session(&self, session_id: &SessionId) -> Result<()>;

    pub async fn mark_session_active(&self, session_id: &SessionId, turn_id: &TurnId) -> Result<()>;
    pub async fn mark_session_idle(&self, session_id: &SessionId, turn_id: &TurnId) -> Result<()>;

    pub async fn save_turn_summary(&self, session_id: &SessionId, summary: &TurnSummary) -> Result<()>;
    pub async fn list_turn_summaries(&self, session_id: &SessionId) -> Result<Vec<TurnSummary>>;
    pub async fn latest_turn_id(&self, session_id: &SessionId) -> Result<Option<TurnId>>;
}
```

#### FileSessionStore 目录（修订）

```
data/
└── sessions/
    └── {session_id}/
        ├── metadata.json              # SessionInfo (含 latest_turn_id)
        └── turns/
            └── {turn_id}/
                └── summary.json       # TurnSummary
```

移除：
- `context.json`
- `transcript.jsonl`

### 3. agent-turn（纯单轮引擎）

#### 职责
- 单轮对话的完整运行
- 工具调用执行
- turn transcript checkpoint

#### 核心结构（修订）

`CheckpointStore` 改为必需依赖，避免 “运行成功但无 checkpoint” 的隐式行为。

```rust
pub struct TurnRuntime<L, T>
where
    L: agent_core::LanguageModel + 'static,
    T: ToolExecutor + 'static,
{
    model: Arc<L>,
    tools: Arc<T>,
    checkpoint_store: Arc<dyn CheckpointStore>,
    config: TurnEngineConfig,
    turns: Arc<RwLock<HashMap<String, TurnControl>>>,
}

impl<L, T> TurnRuntime<L, T> {
    pub fn new(
        model: Arc<L>,
        tools: Arc<T>,
        checkpoint_store: Arc<dyn CheckpointStore>,
        config: TurnEngineConfig,
    ) -> Self;
}
```

#### Checkpoint 目录

```
data/
└── checkpoints/
    └── {turn_id}/
        └── transcript.jsonl
```

### 4. agent（Facade）

#### 职责
- 组合 `SessionManager + TurnRuntime + CheckpointStore`
- 提供简洁的会话与对话 API
- 协调 transcript 恢复和 turn 收尾持久化

#### 核心结构（修订）

```rust
pub struct Agent<L, T>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
    T: ToolExecutor + Send + Sync + 'static,
{
    session_manager: Arc<SessionManager>,
    turn_runtime: Arc<TurnRuntime<L, T>>,
    checkpoint_store: Arc<dyn CheckpointStore>,
}

pub struct AgentBuilder<L, T> {
    model: Option<Arc<L>>,
    tools: Option<Arc<T>>,
    session_store: Option<Arc<dyn SessionStore>>,
    checkpoint_store: Option<Arc<dyn CheckpointStore>>,
    store_path: Option<PathBuf>,
    max_parallel_tools: usize,
}
```

Builder 规则：
1. 若显式传入 `session_store/checkpoint_store`，优先使用
2. 否则要求 `store_path`，并默认创建 `FileSessionStore` + `FileCheckpointStore`

#### API（修订）

```rust
impl<L, T> Agent<L, T> {
    // 会话管理
    pub async fn create_session(
        &self,
        user_id: Option<String>,
        title: Option<String>,
    ) -> Result<SessionId, AgentError>;
    pub async fn list_sessions(&self, filter: SessionFilter) -> Result<Vec<SessionInfo>, AgentError>;
    pub async fn get_session(&self, session_id: &SessionId) -> Result<Option<SessionInfo>, AgentError>;
    pub async fn delete_session(&self, session_id: &SessionId) -> Result<(), AgentError>;

    // 对话
    pub async fn chat(&self, session_id: &SessionId, message: &str) -> Result<ChatResponse, AgentError>;
    pub async fn chat_stream(&self, session_id: &SessionId, message: &str) -> Result<RuntimeStreams, AgentError>;

    // Turn 控制
    pub async fn inject_input(&self, turn_id: &str, input: InputEnvelope) -> Result<(), AgentError>;
    pub async fn cancel_turn(&self, turn_id: &str, reason: Option<String>) -> Result<(), AgentError>;
}
```

#### chat_stream 生命周期（关键闭环）

1. 校验并加载 session
2. `latest_turn_id = session_manager.latest_turn_id(session_id)`
3. 若存在 `latest_turn_id`，`checkpoint_store.load_items(latest_turn_id)` 作为 `TurnRequest.transcript`
4. 创建新 `turn_id`，调用 `mark_session_active(session_id, turn_id)`
5. 调 `turn_runtime.run_turn(request)`
6. 后台任务消费 `run` 事件，遇到 `TurnDone/TurnFailed`：
   - 生成并保存 `TurnSummary`
   - 更新 `SessionInfo.latest_turn_id = Some(turn_id)`（失败 turn 仍可保留用于审计）
   - `mark_session_idle(session_id, turn_id)`
7. 如果上游异常中断，也必须执行 `mark_session_idle`（防止 session 卡住）

#### ChatResponse

```rust
pub struct ChatResponse {
    pub message: String,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
}
```

### 5. 错误与并发语义

建议在 Facade 层定义可区分的错误：
- `SessionNotFound`
- `SessionBusy { session_id, active_turn_id }`
- `Storage` / `Internal`

并发规则（v1）：
1. 同一 `session_id` 同时只能有一个 active turn
2. 不同 session 可以并发执行
3. `inject_input/cancel_turn` 仅作用于 active turn

## 存储迁移策略

从旧结构迁移到新结构：
1. 保留 `sessions/{session_id}/turns/{turn_id}/summary.json`
2. 将 `sessions/{session_id}/turns/{turn_id}/transcript.jsonl` 迁移到 `checkpoints/{turn_id}/transcript.jsonl`
3. 回填 `SessionInfo.latest_turn_id`（按 `started_at` 最大值）

迁移失败策略：
- 单个 turn 迁移失败不阻断整体；记录告警并继续
- 若 `latest_turn_id` 找不到 checkpoint，运行时回退为空 transcript 并输出 warning

## 实现步骤

见 `docs/plans/2026-02-22-agent-implementation-plan.md`。
该实现计划需要同步更新：重点对齐本文的 Store trait 分层、`SessionManager` 类型签名和 `chat_stream` 生命周期闭环。

## 风险与限制

1. **向后兼容性**：`agent-session-cli` 的构造方式会变化，需要过渡层或升级说明
2. **多进程并发**：v1 的 session busy 控制仅保证单进程，跨进程需额外锁策略
3. **错误分类**：需要统一 core/runtime/facade 三层错误映射，避免信息丢失

## 未来扩展

1. **数据库存储**：实现 `DatabaseSessionStore` / `DatabaseCheckpointStore`
2. **多模型路由**：按 session 或 turn 维度路由模型
3. **插件系统**：动态注册工具执行器
