# Session Management Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 创建 agent-session crate，实现 Session 运行时管理，支持多 turn 会话、持久化和历史传递。

**Architecture:** 在 agent-core 添加公共类型，agent-session 实现 SessionRuntime，复用 agent-turn 的 TurnRuntime。

**Tech Stack:** Rust, tokio, serde, 文件存储

---

## 阶段 1: 基础类型 (agent-core)

### Task 1: 添加 Session 相关类型到 agent-core

**Files:**
- Modify: `agent-core/src/lib.rs`
- Create: `agent-core/src/session.rs`

**Step 1: 创建基础类型文件**

```bash
touch agent-core/src/session.rs
```

**Step 2: 添加基础类型定义**

```rust
// agent-core/src/session.rs

use serde::{Deserialize, Serialize};

pub type SessionId = String;
pub type TurnId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Idle,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnStatus {
    Running,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: SessionId,
    pub user_id: Option<String>,
    pub parent_id: Option<SessionId>,
    pub title: String,
    pub status: SessionStatus,
    pub created_at: i64,
    pub updated_at: i64,
    pub archived_at: Option<i64>,
}

impl SessionInfo {
    pub fn new(session_id: SessionId, title: String) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            session_id,
            user_id: None,
            parent_id: None,
            title,
            status: SessionStatus::Idle,
            created_at: now,
            updated_at: now,
            archived_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnSummary {
    pub turn_id: TurnId,
    pub epoch: u64,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub status: TurnStatus,
    pub final_message: Option<String>,
    pub tool_calls_count: u32,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnContext {
    pub turn_id: TurnId,
    pub session_id: SessionId,
    pub epoch: u64,
    pub started_at: i64,
}

impl TurnContext {
    pub fn new(session_id: SessionId) -> Self {
        Self {
            turn_id: crate::new_id(),
            session_id,
            epoch: 0,
            started_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub fn next_epoch(&self) -> Self {
        Self {
            turn_id: crate::new_id(),
            session_id: self.session_id.clone(),
            epoch: self.epoch + 1,
            started_at: chrono::Utc::now().timestamp_millis(),
        }
    }
}
```

**Step 3: 在 lib.rs 中导出**

```rust
// agent-core/src/lib.rs
pub mod session;

pub use session::*;
```

**Step 4: 添加 chrono 依赖**

```bash
# agent-core/Cargo.toml 添加
chrono = { workspace = true }
```

**Step 5: 运行测试验证**

```bash
cargo check -p agent-core
```

**Step 6: Commit**

```bash
git add agent-core/src/session.rs agent-core/src/lib.rs agent-core/Cargo.toml
git commit -m "feat(agent-core): add session types

- Add SessionId, TurnId, SessionStatus, TurnStatus
- Add SessionInfo, TurnSummary, TurnContext
- Export from agent-core"
```

---

## 阶段 2: 存储抽象 (agent-session)

### Task 2: 创建 agent-session crate

**Files:**
- Create: `agent-session/Cargo.toml`
- Create: `agent-session/src/lib.rs`

**Step 1: 创建 Cargo.toml**

```toml
# agent-session/Cargo.toml
[package]
name = "agent-session"
version = "0.1.0"
edition = "2021"

[dependencies]
agent-core = { path = "../agent-core" }
agent-turn = { path = "../agent-turn" }
thiserror = { workspace = true }
anyhow = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
async-trait = { workspace = true }
tokio = { workspace = true }
chrono = { workspace = true }
uuid = { version = "1", features = ["v4", "serde"] }
tracing = { workspace = true }
futures = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

**Step 2: 添加到 workspace**

```bash
# Cargo.toml members 添加
"agent-session",
```

**Step 3: 创建基础 lib.rs**

```rust
// agent-session/src/lib.rs
pub mod storage;
pub mod session_runtime;

pub use storage::{SessionStore, FileSessionStore, SessionFilter};
pub use session_runtime::SessionRuntime;
```

**Step 4: 验证编译**

```bash
cargo check -p agent-session
```

**Step 5: Commit**

```bash
git add agent-session/
git commit -m "feat(agent-session): create crate structure"
```

### Task 3: 实现 FileSessionStore

**Files:**
- Create: `agent-session/src/storage.rs`

**Step 1: 编写测试**

```rust
// agent-session/src/storage.rs

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_and_get_session() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileSessionStore::new(temp_dir.path().to_path_buf());

        let info = SessionInfo::new("s1".into(), "Test Session".into());
        store.create(&info).await.unwrap();

        let retrieved = store.get("s1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test Session");
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileSessionStore::new(temp_dir.path().to_path_buf());

        for i in 1..=3 {
            let info = SessionInfo::new(format!("s{}", i), format!("Session {}", i));
            store.create(&info).await.unwrap();
        }

        let sessions = store.list(SessionFilter::default()).await.unwrap();
        assert_eq!(sessions.len(), 3);
    }

    #[tokio::test]
    async fn test_delete_session() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileSessionStore::new(temp_dir.path().to_path_buf());

        let info = SessionInfo::new("s1".into(), "Test".into());
        store.create(&info).await.unwrap();

        store.delete("s1").await.unwrap();

        let retrieved = store.get("s1").await.unwrap();
        assert!(retrieved.is_none());
    }
}
```

**Step 2: 运行测试验证失败**

```bash
cargo test -p agent-session storage::tests -- --nocapture 2>&1 | head -30
# Expected: compilation error - FileSessionStore not defined
```

**Step 3: 实现 FileSessionStore**

```rust
// agent-session/src/storage.rs

use agent_core::{SessionId, SessionInfo, SessionStatus};
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct SessionFilter {
    pub user_id: Option<String>,
    pub status: Option<SessionStatus>,
    pub limit: Option<usize>,
}

impl Default for SessionFilter {
    fn default() -> Self {
        Self {
            user_id: None,
            status: None,
            limit: Some(100),
        }
    }
}

#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn create(&self, info: &SessionInfo) -> Result<()>;
    async fn get(&self, session_id: &SessionId) -> Result<Option<SessionInfo>>;
    async fn update(&self, info: &SessionInfo) -> Result<()>;
    async fn delete(&self, session_id: &SessionId) -> Result<()>;
    async fn list(&self, filter: SessionFilter) -> Result<Vec<SessionInfo>>;
}

pub struct FileSessionStore {
    base_path: PathBuf,
}

impl FileSessionStore {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    fn session_path(&self, session_id: &SessionId) -> PathBuf {
        self.base_path.join(session_id)
    }

    fn metadata_path(&self, session_id: &SessionId) -> PathBuf {
        self.session_path(session_id).join("metadata.json")
    }
}

#[async_trait]
impl SessionStore for FileSessionStore {
    async fn create(&self, info: &SessionInfo) -> Result<()> {
        let path = self.session_path(&info.session_id);
        fs::create_dir_all(&path).await?;

        let metadata_path = self.metadata_path(&info.session_id);
        let json = serde_json::to_string_pretty(info)?;
        fs::write(metadata_path, json).await?;

        info!("Created session: {}", info.session_id);
        Ok(())
    }

    async fn get(&self, session_id: &SessionId) -> Result<Option<SessionInfo>> {
        let metadata_path = self.metadata_path(session_id);

        if !metadata_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(metadata_path).await?;
        let info: SessionInfo = serde_json::from_str(&content)?;
        Ok(Some(info))
    }

    async fn update(&self, info: &SessionInfo) -> Result<()> {
        let metadata_path = self.metadata_path(&info.session_id);

        if !metadata_path.exists() {
            anyhow::bail!("Session not found: {}", info.session_id);
        }

        let json = serde_json::to_string_pretty(info)?;
        fs::write(metadata_path, json).await?;
        Ok(())
    }

    async fn delete(&self, session_id: &SessionId) -> Result<()> {
        let path = self.session_path(session_id);

        if path.exists() {
            fs::remove_dir_all(path).await?;
            info!("Deleted session: {}", session_id);
        }
        Ok(())
    }

    async fn list(&self, filter: SessionFilter) -> Result<Vec<SessionInfo>> {
        let mut sessions = Vec::new();

        let mut entries = fs::read_dir(&self.base_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                let metadata_path = path.join("metadata.json");
                if metadata_path.exists() {
                    if let Ok(content) = fs::read_to_string(metadata_path).await {
                        if let Ok(info) = serde_json::from_str::<SessionInfo>(&content) {
                            // Apply filters
                            if let Some(ref status) = filter.status {
                                if info.status != *status {
                                    continue;
                                }
                            }
                            sessions.push(info);
                        }
                    }
                }
            }
        }

        // Sort by updated_at descending
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        if let Some(limit) = filter.limit {
            sessions.truncate(limit);
        }

        Ok(sessions)
    }
}
```

**Step 4: 运行测试验证通过**

```bash
cargo test -p agent-session storage::tests
# Expected: PASS
```

**Step 5: Commit**

```bash
git add agent-session/src/storage.rs
git commit -m "feat(agent-session): implement FileSessionStore

- Add SessionStore trait
- Implement FileSessionStore with create/get/update/delete/list
- Add basic tests"
```

---

## 阶段 3: Runtime 实现 (agent-session)

### Task 4: 实现 SessionRuntime

**Files:**
- Create: `agent-session/src/session_runtime.rs`

**Step 1: 编写测试**

```rust
// agent-session/src/session_runtime.rs

#[cfg(test)]
mod tests {
    use super::*;
    use agent_core::{InputEnvelope, SessionMeta};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_session() {
        let temp_dir = TempDir::new().unwrap();
        let runtime = SessionRuntime::new(
            temp_dir.path().to_path_buf(),
            /* model */ Arc::new(MockModel::new()),
            /* tools */ Arc::new(MockTools::new()),
        );

        let session_id = runtime.create_session(None, Some("Test".into())).await.unwrap();
        assert!(!session_id.is_empty());
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let runtime = SessionRuntime::new(
            temp_dir.path().to_path_buf(),
            Arc::new(MockModel::new()),
            Arc::new(MockTools::new()),
        );

        runtime.create_session(None, Some("Session 1".into())).await.unwrap();
        runtime.create_session(None, Some("Session 2".into())).await.unwrap();

        let sessions = runtime.list_sessions(SessionFilter::default()).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }
}
```

**Step 2: 运行测试验证失败**

```bash
cargo test -p agent-session session_runtime::tests -- --nocapture 2>&1 | head -30
# Expected: compilation error - SessionRuntime not defined
```

**Step 3: 实现 SessionRuntime**

```rust
// agent-session/src/session_runtime.rs

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use agent_core::{
    new_id, InputEnvelope, ModelRequest, RuntimeEvent, RuntimeStreams, SessionId,
    SessionInfo, SessionMeta, SessionStatus, TurnContext, TurnId, TurnInfo, TurnStatus,
    TurnSummary,
};
use agent_turn::{TurnEngineConfig, TurnRuntime};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info};

use crate::storage::{FileSessionStore, SessionFilter, SessionStore};

pub struct SessionRuntime<L, T>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
    T: agent_turn::ToolExecutor + Send + Sync + 'static,
{
    store: Arc<FileSessionStore>,
    turn_runtime: Arc<TurnRuntime<L, T>>,
    sessions: Arc<RwLock<HashMap<SessionId, SessionState>>>,
    config: SessionConfig,
}

struct SessionState {
    info: SessionInfo,
    turns: Vec<TurnSummary>,
    current_turn: Option<TurnInfo>,
}

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub max_parallel_tools: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_parallel_tools: 4,
        }
    }
}

impl<L, T> SessionRuntime<L, T>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
    T: agent_turn::ToolExecutor + Send + Sync + 'static,
{
    pub fn new(base_path: PathBuf, model: Arc<L>, tools: Arc<T>) -> Self {
        let store = Arc::new(FileSessionStore::new(base_path));
        let turn_config = TurnEngineConfig::default();
        let turn_runtime = Arc::new(TurnRuntime::new(
            model,
            tools,
            turn_config,
        ));

        Self {
            store,
            turn_runtime,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            config: SessionConfig::default(),
        }
    }

    async fn ensure_session_loaded(&self, session_id: &SessionId) -> Result<SessionState> {
        // Check in-memory first
        {
            let sessions = self.sessions.read().await;
            if let Some(state) = sessions.get(session_id) {
                return Ok(state.clone());
            }
        }

        // Load from storage
        if let Some(info) = self.store.get(session_id).await? {
            let state = SessionState {
                info,
                turns: Vec::new(),
                current_turn: None,
            };
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), state.clone());
            return Ok(state);
        }

        Err(anyhow!("Session not found: {}", session_id))
    }
}

#[async_trait]
impl<L, T> agent_core::Runtime for SessionRuntime<L, T>
where
    L: agent_core::LanguageModel + Send + Sync + 'static,
    T: agent_turn::ToolExecutor + Send + Sync + 'static,
{
    async fn create_session(
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
            current_turn: None,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), state);

        info!("Created session: {}", session_id);
        Ok(session_id)
    }

    async fn list_sessions(&self, filter: SessionFilter) -> Result<Vec<SessionInfo>> {
        self.store.list(filter).await
    }

    async fn get_session(&self, session_id: &SessionId) -> Result<Option<SessionInfo>> {
        self.store.get(session_id).await
    }

    async fn delete_session(&self, session_id: &SessionId) -> Result<()> {
        // Remove from memory
        {
            let mut sessions = self.sessions.write().await;
            sessions.remove(session_id);
        }

        // Delete from storage
        self.store.delete(session_id).await
    }

    async fn run_turn(
        &self,
        session_id: &SessionId,
        input: InputEnvelope,
    ) -> Result<RuntimeStreams> {
        let state = self.ensure_session_loaded(session_id).await?;

        // Check if session is busy
        if state.current_turn.is_some() {
            return Err(anyhow!("Session {} is busy", session_id));
        }

        // Create turn context
        let turn_context = TurnContext::new(session_id.clone());

        // TODO: Integrate with TurnRuntime
        // For now, return a placeholder

        Ok(RuntimeStreams {
            run: Box::pin(futures::stream::empty()),
            ui: Box::pin(futures::stream::empty()),
        })
    }

    async fn inject_input(&self, turn_id: &TurnId, input: InputEnvelope) -> Result<()> {
        // Find session with this turn
        let sessions = self.sessions.read().await;
        for (session_id, state) in sessions.iter() {
            if let Some(ref current) = state.current_turn {
                if current.context.turn_id == turn_id {
                    // TODO: inject to turn runtime
                    return Ok(());
                }
            }
        }
        Err(anyhow!("Turn not found: {}", turn_id))
    }

    async fn cancel_turn(&self, turn_id: &TurnId, reason: Option<String>) -> Result<()> {
        // TODO: implement
        Ok(())
    }

    async fn restore_session(&self, session_id: &SessionId) -> Result<SessionInfo> {
        self.ensure_session_loaded(session_id)
            .await
            .map(|s| s.info)
    }
}
```

**Step 4: 运行测试验证通过**

```bash
cargo test -p agent-session session_runtime::tests
# Expected: PASS (or compile errors to fix)
```

**Step 5: Commit**

```bash
git add agent-session/src/session_runtime.rs
git commit -m "feat(agent-session): implement SessionRuntime

- Add SessionRuntime struct
- Implement Runtime trait
- Add session state management
- Integrate with TurnRuntime (placeholder)"
```

---

## 阶段 4: 集成与完善

### Task 5: 集成 TurnRuntime 实现完整流程

**Files:**
- Modify: `agent-session/src/session_runtime.rs`

**Step 1: 实现 run_turn 完整逻辑**

```rust
// 在 SessionRuntime 中实现完整的 turn 运行逻辑

async fn run_turn_impl(
    &self,
    session_id: &SessionId,
    input: InputEnvelope,
) -> Result<RuntimeStreams> {
    let mut sessions = self.sessions.write().await;
    let state = sessions
        .get_mut(session_id)
        .ok_or_else(|| anyhow!("Session not found"))?;

    // Check if busy
    if state.current_turn.is_some() {
        return Err(anyhow!("Session is busy"));
    }

    // Create turn context
    let turn_context = TurnContext::new(session_id.clone());
    let turn_id = turn_context.turn_id.clone();

    // Update session status
    state.info.status = SessionStatus::Active;
    state.info.updated_at = chrono::Utc::now().timestamp_millis();
    self.store.update(&state.info).await?;

    // Store turn info
    state.current_turn = Some(TurnInfo {
        context: turn_context,
        started_at: chrono::Utc::now().timestamp_millis(),
    });

    // Create turn request
    let request = TurnRequest {
        meta: SessionMeta {
            session_id: session_id.clone(),
            turn_id: turn_id.clone(),
        },
        initial_input: input,
    };

    // Run turn
    let streams = self.turn_runtime.run_turn(request).await?;

    Ok(streams)
}
```

**Step 2: 实现 turn 完成回调**

```rust
// 当 Turn 完成时调用
async fn on_turn_complete(&self, session_id: &SessionId, summary: TurnSummary) {
    let mut sessions = self.sessions.write().await;
    if let Some(state) = sessions.get_mut(session_id) {
        // Add to turns history
        state.turns.push(summary);

        // Clear current turn
        state.current_turn = None;

        // Update session status
        state.info.status = SessionStatus::Idle;
        state.info.updated_at = chrono::Utc::now().timestamp_millis();

        // Persist
        let _ = self.store.update(&state.info).await;
    }
}
```

**Step 3: Commit**

```bash
git commit -m "feat(agent-session): integrate TurnRuntime in SessionRuntime"
```

### Task 6: 添加集成测试

**Files:**
- Create: `agent-session/tests/integration.rs`

**Step 1: 编写集成测试**

```rust
use agent_session::{SessionRuntime, SessionFilter};
use agent_core::InputEnvelope;

#[tokio::test]
async fn test_full_session_lifecycle() {
    let temp_dir = tempfile::tempdir().unwrap();
    let runtime = SessionRuntime::new(
        temp_dir.path().to_path_buf(),
        /* model */ Arc::new(MockModel::new()),
        /* tools */ Arc::new(MockTools::new()),
    );

    // Create session
    let session_id = runtime.create_session(None, None).await.unwrap();

    // List sessions
    let sessions = runtime.list_sessions(SessionFilter::default()).await.unwrap();
    assert_eq!(sessions.len(), 1);

    // Get session
    let session = runtime.get_session(&session_id).await.unwrap();
    assert!(session.is_some());

    // Run turn (mock)
    let input = InputEnvelope::user_text("Hello");
    // let _streams = runtime.run_turn(&session_id, input).await.unwrap();

    // Delete session
    runtime.delete_session(&session_id).await.unwrap();

    let sessions = runtime.list_sessions(SessionFilter::default()).await.unwrap();
    assert_eq!(sessions.len(), 0);
}
```

**Step 2: 运行集成测试**

```bash
cargo test -p agent-session --test integration
```

**Step 3: Commit**

```bash
git add agent-session/tests/
git commit -m "test(agent-session): add integration tests"
```

---

## 完成

**Plan complete and saved to `docs/plans/2026-02-22-session-management-plan.md`**

**Two execution options:**

1. **Subagent-Driven (this session)** - Dispatch fresh subagent per task, review between tasks

2. **Parallel Session (separate)** - Open new session with executing-plans

**Which approach?**
