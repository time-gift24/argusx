# Session 管理设计方案

## 1. 背景

基于 OpenCode 和 Codex 的调研结果，设计 agent-session crate。

### 调研结论

| 方面 | OpenCode | Codex |
|------|----------|-------|
| 存储 | SQLite | JSONL 文件 |
| Turn 边界 | finish != "tool-calls" | turn_context 事件 |
| 历史传递 | 完整 session 历史 | 从 turn_context 起 |
| Fork 支持 | ✅ | ❌ |

## 2. 模块划分

```
agent-core     → 公共类型定义
agent-session → Session 运行时
agent-turn    → Turn 运行时 (现有)
```

### 依赖关系

```
agent-core (无依赖)
    ↑
agent-session (依赖 agent-core, agent-turn)
    ↑
agent-turn (依赖 agent-core)
```

## 3. 核心数据结构

### 3.1 公共类型 (agent-core)

```rust
// agent-core/src/session.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,   // 有活跃 turn
    Idle,     // 无活跃 turn
    Archived, // 已归档
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: SessionId,
    pub user_id: Option<String>,
    pub parent_id: Option<SessionId>,  // 支持 fork
    pub title: String,
    pub status: SessionStatus,
    pub created_at: i64,
    pub updated_at: i64,
    pub archived_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnSummary {
    pub turn_id: TurnId,
    pub epoch: u64,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub status: TurnStatus,           // Done, Failed, Cancelled
    pub final_message: Option<String>,
    pub tool_calls_count: u32,
    pub usage: Usage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnStatus {
    Running,
    Done,
    Failed,
    Cancelled,
}
```

### 3.2 TurnContext (Turn 边界标记)

```rust
// 每次新 turn 开始时创建，类似 Codex 的 turn_context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnContext {
    pub turn_id: TurnId,
    pub session_id: SessionId,
    pub epoch: u64,
    pub model_config: ModelConfig,
    pub started_at: i64,
}

impl TurnContext {
    pub fn new(session_id: SessionId, model_config: ModelConfig) -> Self {
        Self {
            turn_id: TurnId(new_id()),
            session_id,
            epoch: 0,
            model_config,
            started_at: now(),
        }
    }
}
```

### 3.3 SessionState (内部状态)

```rust
// agent-session/src/session_state.rs

struct SessionState {
    info: SessionInfo,
    turns: Vec<TurnSummary>,           // 历史 turns
    current_turn: Option<TurnInfo>,   // 当前活跃 turn
    checkpoint_store: Option<Arc<dyn CheckpointStore>>,
}

struct TurnInfo {
    context: TurnContext,
    started_at: i64,
    event_tx: mpsc::UnboundedSender<RuntimeEvent>,  // 用于 inject/cancel
}
```

## 4. 存储设计

### 4.1 文件结构

```
sessions/
├── manifest.json          # session 索引
└── {session_id}/
    ├── metadata.json      # SessionInfo
    ├── turns/
    │   └── {turn_id}/
    │       ├── context.json   # TurnContext
    │       ├── summary.json   # TurnSummary
    │       └── transcript.jsonl  # TranscriptItem 列表
    └── checkpoint.json    # 最新 checkpoint
```

### 4.2 Storage Trait

```rust
pub trait SessionStore: Send + Sync {
    async fn create(&self, info: &SessionInfo) -> Result<()>;
    async fn get(&self, session_id: &str) -> Result<Option<SessionInfo>>;
    async fn update(&self, info: &SessionInfo) -> Result<()>;
    async fn delete(&self, session_id: &str) -> Result<()>;
    async fn list(&self, filter: SessionFilter) -> Result<Vec<SessionInfo>>;
}

pub struct FileSessionStore { /* 实现 */ }
pub struct SqliteSessionStore { /* 实现 */ }
```

## 5. API 设计

### 5.1 SessionRuntime Trait

```rust
#[async_trait]
pub trait SessionRuntime: Send + Sync {
    // ===== Session 管理 =====
    async fn create_session(&self, user_id: Option<String>, title: Option<String>) -> SessionId;
    async fn list_sessions(&self, filter: SessionFilter) -> Vec<SessionInfo>;
    async fn get_session(&self, session_id: &str) -> Option<SessionInfo>;
    async fn delete_session(&self, session_id: &str) -> Result<()>;

    // ===== Turn 操作 =====
    async fn run_turn(&self, session_id: &str, input: InputEnvelope) -> Result<RuntimeStreams>;
    async fn inject_input(&self, turn_id: &str, input: InputEnvelope) -> Result<()>;
    async fn cancel_turn(&self, turn_id: &str, reason: Option<String>) -> Result<()>;

    // ===== 恢复 =====
    async fn restore_session(&self, session_id: &str) -> Result<SessionInfo>;
}
```

### 5.2 SessionFilter

```rust
#[derive(Debug, Clone, Default)]
pub struct SessionFilter {
    pub user_id: Option<String>,
    pub status: Option<SessionStatus>,
    pub from_date: Option<i64>,
    pub to_date: Option<i64>,
    pub limit: Option<usize>,
}
```

## 6. 核心流程

### 6.1 创建 Session 并运行 Turn

```
用户: run_turn(session_id, input)
    │
    ▼
SessionRuntime::run_turn("s1", input)
    │
    ├─► 检查 session "s1" 是否存在
    │
    ├─► 检查是否有活跃 turn → 有则返回错误
    │
    ├─► 创建 TurnContext
    │       turn_id = new_id()
    │       epoch = 0
    │
    ├─► 保存 TurnInfo 到 SessionState.current_turn
    │
    ├─► 调用 TurnRuntime::run_turn(request)
    │       └── TurnEngine 处理事件
    │
    ├─► Turn 完成回调:
    │       ├─► 创建 TurnSummary
    │       ├─► 添加到 SessionState.turns
    │       ├─► 清空 current_turn
    │       ├─► 更新 SessionInfo.status = Idle
    │       └─► 持久化 checkpoint
    │
    └─► 返回 RuntimeStreams
```

### 6.2 Turn 间历史传递

```rust
// TurnRuntime::run_turn 时传入
struct TurnRequest {
    pub meta: SessionMeta {
        session_id,
        turn_id,  // 新 turn 的 turn_id
    },
    pub initial_input: InputEnvelope,
    pub transcript: Vec<TranscriptItem>,  // 上一个 turn 的完整 transcript
}
```

## 7. 实现计划

### 阶段 1: 基础类型 (agent-core)

- [ ] 添加 `SessionId`, `TurnId`, `SessionStatus` 等类型
- [ ] 添加 `SessionInfo`, `TurnSummary`, `TurnContext` 结构
- [ ] 添加 `TurnStatus` 枚举

### 阶段 2: 存储抽象 (agent-session)

- [ ] 定义 `SessionStore` trait
- [ ] 实现 `FileSessionStore`
- [ ] 实现 checkpoint 持久化

### 阶段 3: Runtime 实现 (agent-session)

- [ ] 实现 `SessionRuntime` struct
- [ ] 实现 session 生命周期管理
- [ ] 集成 `TurnRuntime`
- [ ] 实现 turn 边界处理

### 阶段 4: API 完善

- [ ] 实现 list/get/delete session
- [ ] 实现 fork session (可选)
- [ ] 添加测试

## 8. 参考

- OpenCode: `.vendor/opencode/packages/opencode/src/session/`
- Codex: `.codex/sessions/YYYY/MM/DD/`
- 现有实现: `agent-turn/`
