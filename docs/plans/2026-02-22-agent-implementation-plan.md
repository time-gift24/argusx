# Agent Crate 实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 重构现有代码并创建新的 `agent` crate，提供统一的 Facade API

**Architecture:** 分离关注点 - agent-session 专注会话元数据，agent-turn 专注对话引擎，agent 做组合

**Tech Stack:** Rust, tokio, async-trait, serde

---

## 概览

本计划分三个阶段：
1. **Phase 1**: 重构 agent-session（提取 SessionManager，简化存储）
2. **Phase 2**: 修改 agent-turn（移除对 session 的依赖）
3. **Phase 3**: 创建 agent crate（Facade）

---

## Phase 1: 重构 agent-session

### Task 1: 提取 SessionManager

**Files:**
- Create: `agent-session/src/manager.rs`
- Modify: `agent-session/src/lib.rs`
- Modify: `agent-session/src/session_runtime.rs`
- Test: `agent-session/tests/manager.rs`

**Step 1: 创建 manager.rs，定义 SessionManager 结构**

```rust
// agent-session/src/manager.rs
use std::collections::HashMap;
use std::sync::Arc;

use agent_core::{new_id, SessionId, SessionInfo, SessionStatus, TurnSummary};
use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::storage::{SessionFilter, SessionStore};

#[derive(Clone)]
struct SessionState {
    info: SessionInfo,
    turns: Vec<TurnSummary>,
}

pub struct SessionManager<S: SessionStore> {
    store: Arc<S>,
    sessions: Arc<RwLock<HashMap<SessionId, SessionState>>>,
}

impl<S: SessionStore> SessionManager<S> {
    pub fn new(store: Arc<S>) -> Self {
        Self {
            store,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_session(
        &self,
        user_id: Option<String>,
        title: Option<String>,
    ) -> Result<SessionId> {
        let session_id = new_id();
        let title = title.unwrap_or_else(|| format!("Session {}", &session_id[..8]));

        let mut info = SessionInfo::new(session_id.clone(), title);
        info.user_id = user_id;

        self.store.create(&info).await?;

        let state = SessionState {
            info,
            turns: Vec::new(),
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), state);

        Ok(session_id)
    }

    pub async fn list_sessions(&self, filter: SessionFilter) -> Result<Vec<SessionInfo>> {
        self.store.list(filter).await
    }

    pub async fn get_session(&self, session_id: &SessionId) -> Result<Option<SessionInfo>> {
        // 先检查内存
        {
            let sessions = self.sessions.read().await;
            if let Some(state) = sessions.get(session_id) {
                return Ok(Some(state.info.clone()));
            }
        }
        // 再查存储
        self.store.get(session_id).await
    }

    pub async fn delete_session(&self, session_id: &SessionId) -> Result<()> {
        {
            let mut sessions = self.sessions.write().await;
            sessions.remove(session_id);
        }
        self.store.delete(session_id).await
    }

    pub async fn save_turn_summary(
        &self,
        session_id: &SessionId,
        summary: TurnSummary,
    ) -> Result<()> {
        self.store.save_turn_summary(session_id, &summary).await?;

        let mut sessions = self.sessions.write().await;
        if let Some(state) = sessions.get_mut(session_id) {
            state.turns.push(summary);
        }

        Ok(())
    }

    pub async fn list_turn_summaries(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<TurnSummary>> {
        // 先检查内存
        {
            let sessions = self.sessions.read().await;
            if let Some(state) = sessions.get(session_id) {
                return Ok(state.turns.clone());
            }
        }
        // 再查存储
        self.store.list_turn_summaries(session_id).await
    }

    pub async fn ensure_session_loaded(&self, session_id: &SessionId) -> Result<SessionState> {
        {
            let sessions = self.sessions.read().await;
            if let Some(state) = sessions.get(session_id) {
                return Ok(state.clone());
            }
        }

        if let Some(info) = self.store.get(session_id).await? {
            let turns = self.store.list_turn_summaries(session_id).await?;
            let state = SessionState { info, turns };
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), state.clone());
            return Ok(state);
        }

        anyhow::bail!("Session not found: {}", session_id)
    }
}
```

**Step 2: 修改 lib.rs 导出 SessionManager**

```rust
// agent-session/src/lib.rs
pub mod session_runtime;
pub mod storage;
pub mod manager;  // 新增

pub use session_runtime::SessionRuntime;
pub use storage::{FileSessionStore, FileTurnCheckpointStore, SessionFilter, SessionStore};
pub use manager::SessionManager;
```

**Step 3: 更新 session_runtime.rs 使用 SessionManager**

将现有的会话管理逻辑移到 SessionManager，然后让 SessionRuntime 组合 SessionManager + TurnRuntime。

（此处略过详细代码，核心是分离关注点）

**Step 4: 运行测试**

Run: `cargo test -p agent-session --lib`
Expected: PASS

**Step 5: Commit**

```bash
git add agent-session/src/manager.rs agent-session/src/lib.rs agent-session/src/session_runtime.rs
git commit -m "refactor(agent-session): extract SessionManager from SessionRuntime

- Create SessionManager for pure session metadata management
- SessionRuntime now composes SessionManager + TurnRuntime
- Add ensure_session_loaded for session recovery"
```

---

### Task 2: 简化 FileSessionStore

**Files:**
- Modify: `agent-session/src/storage.rs`

**Step 1: 移除 transcript 相关方法**

从 FileSessionStore 移除：
- `save_turn_transcript`
- `load_turn_transcript`
- `checked_turn_transcript_path`

保留：
- `save_turn_context` - 移到 agent-turn 的 CheckpointStore

**Step 2: 运行测试**

Run: `cargo test -p agent-session --lib`
Expected: PASS (部分测试需要删除或修改)

**Step 3: Commit**

```bash
git commit -m "refactor(agent-session): simplify FileSessionStore

- Remove transcript storage (moved to agent crate)
- Keep only session metadata and turn summary
- Update storage structure for future extensibility"
```

---

### Task 3: 更新 agent-session-cli

**Files:**
- Modify: `agent-session-cli/src/main.rs`
- Modify: `agent-session-cli/src/mock.rs`

**Step 1: 更新 mock.rs 使用 SessionManager**

将 `build_runtime` 改为返回 SessionManager 或组合类型。

**Step 2: 运行 CLI 测试**

Run: `cargo test -p agent-session-cli`
Expected: PASS

**Step 3: Commit**

```bash
git commit -m "refactor(agent-session-cli): update to use SessionManager"
```

---

## Phase 2: 修改 agent-turn

### Task 4: 移除 TurnRuntime 对 session 的依赖

**Files:**
- Modify: `agent-turn/src/runtime_impl.rs`

**Step 1: 简化 TurnRuntime**

移除以下依赖：
- 不再需要 SessionManager
- CheckpointStore 成为必需而非可选

```rust
// 修改后
pub struct TurnRuntime<L, T>
where
    L: agent_core::LanguageModel + 'static,
    T: ToolExecutor + 'static,
{
    model: Arc<L>,
    tools: Arc<T>,
    checkpoint_store: Arc<dyn CheckpointStore>,  // 改为必需
    config: TurnEngineConfig,
    turns: Arc<RwLock<HashMap<String, TurnControl>>>,
}

impl<L, T> TurnRuntime<L, T> {
    pub fn new(
        model: Arc<L>,
        tools: Arc<T>,
        checkpoint_store: Arc<dyn CheckpointStore>,
        config: TurnEngineConfig,
    ) -> Self {
        // ...
    }
}
```

**Step 2: 运行测试**

Run: `cargo test -p agent-turn --lib`
Expected: PASS (如有编译错误需要修复)

**Step 3: Commit**

```bash
git commit -m "refactor(agent-turn): make CheckpointStore required

- TurnRuntime no longer depends on SessionManager
- CheckpointStore is now required for checkpoint functionality"
```

---

### Task 5: 创建 CheckpointStore 实现

**Files:**
- Create: `agent-turn/src/checkpoint.rs`

**Step 1: 实现 FileCheckpointStore**

```rust
// agent-turn/src/checkpoint.rs
use std::path::PathBuf;
use std::sync::Arc;

use agent_core::{AgentError, CheckpointStore, TranscriptItem};
use anyhow::Result;
use async_trait::async_trait;
use tokio::fs;

pub struct FileCheckpointStore {
    base_path: PathBuf,
}

impl FileCheckpointStore {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    fn checked_turn_path(&self, turn_id: &str) -> Result<PathBuf> {
        Ok(self.base_path.join(turn_id))
    }

    fn checked_transcript_path(&self, turn_id: &str) -> Result<PathBuf> {
        Ok(self.checked_turn_path(turn_id)?.join("transcript.jsonl"))
    }
}

#[async_trait]
impl CheckpointStore for FileCheckpointStore {
    async fn append_items(
        &self,
        turn_id: &str,
        items: &[TranscriptItem],
    ) -> Result<(), AgentError> {
        let path = self.checked_transcript_path(turn_id)?;

        if !fs::try_exists(path.parent().unwrap()).await? {
            fs::create_dir_all(path.parent().unwrap()).await?;
        }

        // Append to existing file
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;

        for item in items {
            let line = serde_json::to_string(item)?;
            use tokio::io::AsyncWriteExt;
            file.write_all(line.as_bytes()).await?;
            file.write_all(b"\n").await?;
        }

        Ok(())
    }

    async fn load_items(&self, turn_id: &str) -> Result<Vec<TranscriptItem>, AgentError> {
        let path = self.checked_transcript_path(turn_id)?;

        if !fs::try_exists(&path).await? {
            return Ok(Vec::new());
        }

        let raw = fs::read_to_string(&path).await?;
        let mut items = Vec::new();

        for line in raw.lines().filter(|l| !l.trim().is_empty()) {
            let item: TranscriptItem = serde_json::from_str(line)
                .map_err(|e| AgentError::Internal { message: e.to_string() })?;
            items.push(item);
        }

        Ok(items)
    }

    async fn snapshot(&self, turn_id: &str, items: &[TranscriptItem]) -> Result<(), AgentError> {
        let path = self.checked_transcript_path(turn_id)?;
        let parent = path.parent().unwrap();

        fs::create_dir_all(parent).await?;

        let mut content = String::new();
        for item in items {
            content.push_str(&serde_json::to_string(item)?);
            content.push('\n');
        }

        fs::write(&path, content).await?;
        Ok(())
    }
}
```

**Step 2: 导出新模块**

```rust
// agent-turn/src/lib.rs
pub mod adapters;
pub mod checkpoint;  // 新增
pub mod effect;
pub mod engine;
pub mod journal;
pub mod projection;
pub mod reducer;
pub mod runtime_impl;
pub mod state;
pub mod transition;

pub use checkpoint::FileCheckpointStore;
```

**Step 3: 运行测试**

Run: `cargo test -p agent-turn --lib`
Expected: PASS

**Step 4: Commit**

```bash
git add agent-turn/src/checkpoint.rs agent-turn/src/lib.rs
git commit -m "feat(agent-turn): add FileCheckpointStore for transcript persistence"
```

---

## Phase 3: 创建 agent crate

### Task 6: 创建 agent crate 骨架

**Files:**
- Create: `agent/Cargo.toml`
- Create: `agent/src/lib.rs`
- Modify: `Cargo.toml` (workspace)

**Step 1: 创建 Cargo.toml**

```toml
[package]
name = "agent"
version = "0.1.0"
edition = "2021"
description = "Unified agent runtime for desktop and CLI"

[dependencies]
agent-core = { path = "../agent-core" }
agent-session = { path = "../agent-session" }
agent-turn = { path = "../agent-turn" }
thiserror = { workspace = true }
anyhow = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
async-trait = { workspace = true }
tokio = { workspace = true }
tokio-stream = "0.1"
tracing = { workspace = true }
futures = { workspace = true }
```

**Step 2: 创建 lib.rs 骨架**

```rust
pub mod builder;
pub mod response;

pub use builder::{Agent, AgentBuilder};
pub use response::ChatResponse;
```

**Step 3: 创建 builder.rs**

```rust
use std::path::PathBuf;
use std::sync::Arc;

use agent_core::{AgentError, LanguageModel, Runtime, ToolExecutor};
use agent_session::{FileSessionStore, SessionFilter, SessionManager};
use agent_turn::{FileCheckpointStore, TurnEngineConfig, TurnRuntime};

pub struct Agent<L, T>
where
    L: LanguageModel + Send + Sync + 'static,
    T: ToolExecutor + Send + Sync + 'static,
{
    session_manager: SessionManager<Arc<dyn agent_session::SessionStore>>,
    turn_runtime: Arc<TurnRuntime<L, T>>,
    checkpoint_store: Arc<dyn agent_core::CheckpointStore>,
}

pub struct AgentBuilder<L, T> {
    model: Option<Arc<L>>,
    tools: Option<Arc<T>>,
    store_path: Option<PathBuf>,
    max_parallel_tools: usize,
}

impl<L, T> Default for AgentBuilder<L, T> {
    fn default() -> Self {
        Self {
            model: None,
            tools: None,
            store_path: None,
            max_parallel_tools: 4,
        }
    }
}

impl<L, T> AgentBuilder<L, T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_model(mut self, model: Arc<L>) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_tools(mut self, tools: Arc<T>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn with_store_path(mut self, path: PathBuf) -> Self {
        self.store_path = Some(path);
        self
    }

    pub fn with_max_parallel_tools(mut self, n: usize) -> Self {
        self.max_parallel_tools = n;
        self
    }

    pub fn build(self) -> Result<Agent<L, T>, AgentError> {
        let model = self.model.ok_or(AgentError::Internal {
            message: "model is required".to_string(),
        })?;
        let tools = self.tools.ok_or(AgentError::Internal {
            message: "tools is required".to_string(),
        })?;
        let store_path = self.store_path.ok_or(AgentError::Internal {
            message: "store_path is required".to_string(),
        })?;

        let sessions_path = store_path.join("sessions");
        let checkpoints_path = store_path.join("checkpoints");

        let session_store: Arc<dyn agent_session::SessionStore> =
            Arc::new(FileSessionStore::new(sessions_path));
        let session_manager = SessionManager::new(session_store);

        let checkpoint_store: Arc<dyn agent_core::CheckpointStore> =
            Arc::new(FileCheckpointStore::new(checkpoints_path));

        let config = TurnEngineConfig {
            max_parallel_tools: self.max_parallel_tools,
            ..TurnEngineConfig::default()
        };

        let turn_runtime = Arc::new(TurnRuntime::new(
            model,
            tools,
            checkpoint_store.clone(),
            config,
        ));

        Ok(Agent {
            session_manager,
            turn_runtime,
            checkpoint_store,
        })
    }
}
```

**Step 4: 创建 response.rs**

```rust
use agent_core::{ToolCall, Usage};

#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub message: String,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
}
```

**Step 5: 运行编译**

Run: `cargo build -p agent`
Expected: SUCCESS (可能有少量错误需要修复)

**Step 6: Commit**

```bash
git add agent/
git commit -m "feat(agent): create agent crate skeleton

- Add AgentBuilder for easy initialization
- Add ChatResponse type
- Basic structure ready for API implementation"
```

---

### Task 7: 实现 Agent 对话 API

**Files:**
- Modify: `agent/src/lib.rs`
- Modify: `agent/src/builder.rs`

**Step 1: 添加会话管理 API**

在 builder.rs 的 Agent 结构体中添加：

```rust
impl<L, T> Agent<L, T>
where
    L: LanguageModel + Send + Sync + 'static,
    T: ToolExecutor + Send + Sync + 'static,
{
    pub async fn create_session(
        &self,
        title: Option<String>,
    ) -> Result<agent_core::SessionId, AgentError> {
        self.session_manager
            .create_session(None, title)
            .await
            .map_err(|e| AgentError::Internal {
                message: e.to_string(),
            })
    }

    pub async fn list_sessions(
        &self,
        filter: SessionFilter,
    ) -> Result<Vec<agent_core::SessionInfo>, AgentError> {
        self.session_manager
            .list_sessions(filter)
            .await
            .map_err(|e| AgentError::Internal {
                message: e.to_string(),
            })
    }

    pub async fn get_session(
        &self,
        session_id: &agent_core::SessionId,
    ) -> Result<Option<agent_core::SessionInfo>, AgentError> {
        self.session_manager
            .get_session(session_id)
            .await
            .map_err(|e| AgentError::Internal {
                message: e.to_string(),
            })
    }

    pub async fn delete_session(
        &self,
        session_id: &agent_core::SessionId,
    ) -> Result<(), AgentError> {
        self.session_manager
            .delete_session(session_id)
            .await
            .map_err(|e| AgentError::Internal {
                message: e.to_string(),
            })
    }
}
```

**Step 2: 添加 chat API**

```rust
impl<L, T> Agent<L, T>
where
    L: LanguageModel + Send + Sync + 'static,
    T: ToolExecutor + Send + Sync + 'static,
{
    pub async fn chat(
        &self,
        session_id: &agent_core::SessionId,
        message: &str,
    ) -> Result<ChatResponse, AgentError> {
        let streams = self.chat_stream(session_id, message).await?;

        // Collect events and build response
        // (详细实现略)
        todo!()
    }

    pub async fn chat_stream(
        &self,
        session_id: &agent_core::SessionId,
        message: &str,
    ) -> Result<agent_core::RuntimeStreams, AgentError> {
        // 1. Ensure session exists
        let _state = self.session_manager
            .ensure_session_loaded(session_id)
            .await
            .map_err(|e| AgentError::Internal {
                message: e.to_string(),
            })?;

        // 2. Load transcript from checkpoint store
        // (从 checkpoin store 加载历史 transcript)

        // 3. Build request
        let request = agent_core::TurnRequest {
            meta: agent_core::SessionMeta::new(
                session_id.clone(),
                agent_core::new_id(),
            ),
            initial_input: agent_core::InputEnvelope::user_text(message),
            transcript: Vec::new(), // TODO: load from checkpoint
        };

        // 4. Run turn
        self.turn_runtime.run_turn(request).await
    }
}
```

**Step 3: 完善 chat 实现**

需要处理：
- 加载历史 transcript
- 收集 RunStreamEvent::TurnDone
- 保存 checkpoint
- 保存 turn summary

**Step 4: 运行编译**

Run: `cargo build -p agent`
Expected: SUCCESS

**Step 5: Commit**

```bash
git commit -m "feat(agent): implement chat API

- Add session management methods (create, list, get, delete)
- Add chat() and chat_stream() methods
- Wire up SessionManager + TurnRuntime"
```

---

### Task 8: 添加集成测试

**Files:**
- Create: `agent/tests/chat.rs`

**Step 1: 写测试**

```rust
use agent::{Agent, AgentBuilder};
use agent_core::{LanguageModel, ModelOutputEvent, ModelRequest, Runtime};
use async_trait::async_trait;
use futures::stream;
use std::sync::Arc;

struct MockModel;

#[async_trait]
impl LanguageModel for MockModel {
    fn model_name(&self) -> &str {
        "mock"
    }

    async fn stream(
        &self,
        _request: ModelRequest,
    ) -> Result<agent_core::ModelEventStream, agent_core::AgentError> {
        Ok(Box::pin(stream::once(async {
            Ok(ModelOutputEvent::Completed { usage: None })
        })))
    }
}

struct MockTools;

#[async_trait]
impl agent_turn::ToolExecutor for MockTools {
    async fn execute_tool(
        &self,
        _call: agent_core::ToolCall,
        _epoch: u64,
    ) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({"result": "ok"}))
    }
}

#[tokio::test]
async fn test_create_session() {
    let agent = AgentBuilder::new()
        .with_model(Arc::new(MockModel))
        .with_tools(Arc::new(MockTools))
        .with_store_path tempfile::tempdir().unwrap().path().to_path_buf())
        .build()
        .unwrap();

    let session_id = agent.create_session(Some("Test".into())).await.unwrap();
    assert!(!session_id.is_empty());
}
```

**Step 2: 运行测试**

Run: `cargo test -p agent`
Expected: PASS

**Step 3: Commit**

```bash
git add agent/tests/
git commit -m "test(agent): add integration tests"
```

---

## 总结

完成以上 8 个 Task 后，将有：
- 重构后的 `agent-session`（纯会话管理）
- 独立的 `agent-turn`（纯对话引擎）
- 全新的 `agent` crate（Facade API）

---

**Plan complete and saved to `docs/plans/2026-02-22-agent-implementation-plan.md`. Two execution options:**

1. **Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

2. **Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach?**
