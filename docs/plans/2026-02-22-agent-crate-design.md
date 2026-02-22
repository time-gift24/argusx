# Agent Crate 设计文档

**日期**: 2026-02-22

## 目标

重构现有代码并创建新的 `agent` crate，提供统一的 Facade API，让 desktop 和 CLI 都能方便地使用。

## 背景

当前项目结构：
- **agent-core**: 核心接口定义（Runtime, LanguageModel, Session 等）
- **agent-turn**: 单轮对话运行时（TurnEngine + TurnRuntime）
- **agent-session**: 会话管理 + 持久化 + 调用 TurnRuntime

现有问题：
1. agent-session 包含了 Runtime 实现，职责不够单一
2. 没有统一的入口 API，调用方需要自己组合
3. 未来需要支持更高级的存储方式（用户喜好、近期活动）

## 设计方案

### 1. 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                         agent                                │
│  (Facade: 组合 SessionManager + TurnRuntime)                │
│  - chat() / chat_stream()                                   │
│  - 会话管理                                                  │
└─────────────────────────────────────────────────────────────┘
         │                                    │
         ▼                                    ▼
┌─────────────────────┐          ┌─────────────────────────┐
│   agent-session     │          │      agent-turn          │
│  (会话元数据管理)    │          │   (单轮对话引擎)          │
│                     │          │                          │
│ - SessionManager    │          │ - TurnRuntime<L, T>     │
│ - SessionStore      │          │ - CheckpointStore        │
│ - FileSessionStore  │          │                         │
└─────────────────────┘          └─────────────────────────┘
         │                                    │
         └──────────────┬─────────────────────┘
                        ▼
              ┌─────────────────────┐
              │     agent-core      │
              │ (核心接口与类型定义)  │
              └─────────────────────┘
```

### 2. agent-session（纯会话管理）

#### 职责
- 会话元数据管理（创建/删除/列表）
- TurnSummary 持久化
- 可插拔的 SessionStore

#### 核心结构

```rust
// src/manager.rs
pub struct SessionManager<S: SessionStore> {
    store: Arc<S>,
}

impl<S: SessionStore> SessionManager<S> {
    pub async fn create_session(
        &self,
        user_id: Option<String>,
        title: Option<String>,
    ) -> Result<SessionId>;

    pub async fn list_sessions(&self, filter: SessionFilter) -> Result<Vec<SessionInfo>>;

    pub async fn get_session(&self, session_id: &SessionId) -> Result<Option<SessionInfo>>;

    pub async fn delete_session(&self, session_id: &SessionId) -> Result<()>;

    pub async fn save_turn_summary(
        &self,
        session_id: &SessionId,
        summary: &TurnSummary,
    ) -> Result<()>;

    pub async fn list_turn_summaries(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<TurnSummary>>;
}
```

#### SessionStore Trait

```rust
// src/storage.rs
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn create(&self, info: &SessionInfo) -> Result<()>;
    async fn get(&self, session_id: &str) -> Result<Option<SessionInfo>>;
    async fn update(&self, info: &SessionInfo) -> Result<()>;
    async fn delete(&self, session_id: &str) -> Result<()>;
    async fn list(&self, filter: SessionFilter) -> Result<Vec<SessionInfo>>;
}
```

#### FileSessionStore 简化

只保留会话相关存储：

```
data/
└── sessions/
    └── {session_id}/
        └── metadata.json      # SessionInfo
        └── turns/
            └── {turn_id}/
                └── summary.json  # TurnSummary
```

移除：
- `context.json` (turn context)
- `transcript.jsonl` (完整对话记录) - 移到 agent crate 管理

### 3. agent-turn（纯对话引擎）

#### 职责
- 单轮对话的完整运行
- 工具调用执行
- Checkpoint 恢复

#### 核心结构

```rust
// src/runtime_impl.rs
pub struct TurnRuntime<L, T>
where
    L: agent_core::LanguageModel + 'static,
    T: ToolExecutor + 'static,
{
    model: Arc<L>,
    tools: Arc<T>,
    checkpoint_store: Option<Arc<dyn CheckpointStore>>,
    config: TurnEngineConfig,
    turns: Arc<RwLock<HashMap<String, TurnControl>>>,
}
```

实现 `Runtime` trait：
- `run_turn(request: TurnRequest) -> Result<RuntimeStreams>`
- `inject_input(turn_id: &str, input: InputEnvelope) -> Result<()>`
- `cancel_turn(turn_id: &str, reason: Option<String>) -> Result<()>`

**关键变化**：TurnRuntime 不再依赖 SessionManager，独立运作。

#### CheckpointStore

```
data/
└── checkpoints/
    └── {turn_id}/
        └── transcript.jsonl  # 完整的 transcript items
```

### 4. agent（Facade）

#### 职责
- 组合 SessionManager + TurnRuntime
- 提供简单的 chat API
- 协调 transcript 加载与保存

#### 核心结构

```rust
// src/lib.rs
pub struct Agent<L, T>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
    T: ToolExecutor + Send + Sync + 'static,
{
    session_manager: SessionManager<Arc<dyn SessionStore>>,
    turn_runtime: Arc<TurnRuntime<L, T>>,
    checkpoint_store: Arc<dyn CheckpointStore>,
}

pub struct AgentBuilder<L, T> {
    model: Option<Arc<L>>,
    tools: Option<Arc<T>>,
    store_path: Option<PathBuf>,
    max_parallel_tools: usize,
}

impl<L, T> AgentBuilder<L, T> {
    pub fn new() -> Self;
    pub fn with_model(mut self, model: Arc<L>) -> Self;
    pub fn with_tools(mut self, tools: Arc<T>) -> Self;
    pub fn with_store_path(mut self, path: PathBuf) -> Self;
    pub fn with_max_parallel_tools(mut self, n: usize) -> Self;
    pub fn build(self) -> Result<Agent<L, T>>;
}
```

#### API

```rust
impl<L, T> Agent<L, T> {
    // 会话管理
    pub async fn create_session(&self, title: Option<String>) -> Result<SessionId, AgentError>;
    pub async fn list_sessions(&self, filter: SessionFilter) -> Result<Vec<SessionInfo>, AgentError>;
    pub async fn get_session(&self, session_id: &SessionId) -> Result<Option<SessionInfo>, AgentError>;
    pub async fn delete_session(&self, session_id: &SessionId) -> Result<(), AgentError>;

    // 对话
    pub async fn chat(&self, session_id: &SessionId, message: &str) -> Result<ChatResponse, AgentError>;

    pub async fn chat_stream(
        &self,
        session_id: &SessionId,
        message: &str,
    ) -> Result<RuntimeStreams, AgentError>;

    // Turn 控制
    pub async fn inject_input(&self, turn_id: &str, input: InputEnvelope) -> Result<(), AgentError>;
    pub async fn cancel_turn(&self, turn_id: &str, reason: Option<String>) -> Result<(), AgentError>;
}
```

#### ChatResponse

```rust
pub struct ChatResponse {
    pub message: String,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
}
```

#### 使用示例

```rust
use agent::{Agent, AgentBuilder};
use bigmodel_api::BigModel;

#[tokio::main]
async fn main() -> Result<()> {
    let agent = AgentBuilder::new()
        .with_model(Arc::new(BigModel::new()))
        .with_tools(Arc::new(MyTools))
        .with_store_path("./data".into())
        .build()?;

    let session_id = agent.create_session(Some("My Chat".into())).await?;

    let response = agent.chat(&session_id, "帮我写一个 hello world").await?;
    println!("{}", response.message);

    Ok(())
}
```

## 实现步骤

见 `docs/plans/2026-02-22-agent-implementation-plan.md`

## 风险与限制

1. **向后兼容性**: 重构可能影响现有 agent-session-cli 的使用方式
2. **存储迁移**: 现有用户数据需要迁移到新的目录结构
3. **错误处理**: 需要统一错误类型

## 未来扩展

1. **数据库存储**: 实现 `DatabaseSessionStore`，支持用户喜好、近期活动
2. **多模型支持**: Agent 支持配置多个模型
3. **插件系统**: 动态加载工具
