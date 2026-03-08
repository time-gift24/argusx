# Session -> Thread -> Turn Implementation Plan
> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将单 Turn 结构重构为 Session -> Thread -> Turn 三层架构，支持多轮对话历史持久化和任务隔离

**Architecture:** 在现有 TurnDriver 之上新增 Thread 层和 SessionManager 层。新增 ThreadStore 用 SQLite 持化层，TurnDriver 保持不变。

**Tech Stack:** Rust, SQLite (sqlx), Tokio, async-trait, uuid, chrono, serde_json

---

## Task 3: 创建 TurnRecord 和 Thread 基础类型
**Files:**
- Create: `session/src/types.rs`
- Modify: `session/src/lib.rs`

**Step 1: 写 TurnRecord 测试**

Create test file: `session/src/types.rs`

```rust
#[cfg(test)]
fn turn_record_serialization() {
    use super::*;
    use crate::types::{TurnRecord, TurnRecordState};

    let record = TurnRecord {
        turn_number: 1,
        user_input: "Hello".to_string(),
        assistant_response: Some("Hi there!".to_string()),
        tool_calls: vec![],
        started_at: Utc::now(),
        completed_at: Some(Utc::now()),
        state: TurnRecordState::Completed,
    };

    let json = serde_json::to_string(&record).unwrap();
    let deserialized: TurnRecord = serde_json::from_str(json).unwrap();

    assert_eq!(record.turn_number, deserialized.turn_number);
    assert_eq!(record.user_input, deserialized.user_input);
}

#[test]
fn turn_record_with_tool_calls() {
    use super::*;
    use crate::types::{TurnRecord, TurnRecordState, ToolCallRecord};

    let record = TurnRecord {
        turn_number: 1,
        user_input: "Search".to_string(),
        assistant_response: Some("Found 3 results".to_string()),
        tool_calls: vec![
            ToolCallRecord {
                call_id: "call-1".to_string(),
                tool_name: "web_search".to_string(),
                arguments: r#"{"query": "rust"}"#.to_string(),
                result: Some("[...]".to_string(),
                is_error: false,
            },
        ],
        started_at: Utc::now(),
        completed_at: Some(Utc::now()),
        state: TurnRecordState::Completed,
    };
    let json = serde_json::to_string(&record).unwrap();
    let deserialized: TurnRecord = serde_json::from_str(json).unwrap();
    assert_eq!(record.tool_calls.len(), deserialized.tool_calls.len());
    assert_eq!(record.tool_calls[0].tool_name, deserialized.tool_calls[0].tool_name);
}
```

**Step 2: 运行测试验证失败**

Run: `cargo test -p session --lib session --no-default-features sqlx`
Expected: test failures for missing ToolCallRecord

**Step 3: 实现 TurnRecord 和 ToolCallRecord 类型**

Add to `session/src/types.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TurnRecordState {
    Completed,
    Failed,
    Interrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCallRecord {
    pub call_id: String,
    pub tool_name: String,
    pub arguments: String,
    pub result: Option<String>,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TurnRecord {
    pub turn_number: usize,
    pub user_input: String,
    pub assistant_response: Option<String>,
    pub tool_calls: Vec<ToolCallRecord>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub state: TurnRecordState,
}
```

**Step 4: 运行测试验证通过**

Run: `cargo test -p session --lib session --no-default-features sqlx`
Expected: All tests pass

**Step 5: 提交**

```bash
git add session/src/types.rs session/src/lib.rs
git commit -m "feat(session): add TurnRecord and ToolCallRecord types"
```
---
## Task 4: 创建 Thread 运行时结构（无后台执行）
**Files:**
- Create: `session/src/thread.rs`

**Step 1: 写 Thread 结构测试**

Create test file: `session/src/thread.rs`

```rust
#[cfg(test)]
fn thread_creation() {
    use super::*;
    use crate::thread::Thread;
    use crate::types::{ThreadState, ThreadRow};
    use uuid::Uuid;

    let thread = Thread::new("session-1".to_string(), None);
    assert_eq!(thread.state, ThreadState::Idle);
    assert!(thread.id != Uuid::nil());
    assert_eq!(thread.session_id, "session-1");
}

#[test]
fn thread_with_title() {
    use super::*;
    use crate::thread::Thread;
    use crate::types::ThreadState;

    let thread = Thread::new("session-1".to_string(), Some("My Thread".to_string()));
    assert_eq!(thread.title, Some("My Thread"));
}

#[test]
fn thread_state_transitions() {
    use super::*;
    use crate::thread::Thread;
    use crate::types::ThreadState;

    let mut thread = Thread::new("session-1".to_string(), None);
    assert!(thread.state, ThreadState::Idle);

    thread.set_state(ThreadState::Processing);
    assert_eq!(thread.state, ThreadState::Processing);

    thread.set_state(ThreadState::Idle);
    assert_eq!(thread.state, ThreadState::Idle);
}
```

**Step 2: 运行测试验证失败**

Run: `cargo test -p session --lib session --no-default-features sqlx`
Expected: test failures for missing set_state method

**Step 3: 实现 Thread 结构**

Add to `session/src/thread.rs`:

```rust
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::collections::Vec;
use crate::types::{ThreadState, ThreadRow, TurnRecord};

pub struct Thread {
    pub id: Uuid,
    pub session_id: String,
    pub title: Option<String>,
    pub state: ThreadState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub history: Vec<TurnRecord>,

    pub fn new(session_id: String, title: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            title,
            state: ThreadState::Idle,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            history: Vec::new(),
        }
    }

    pub fn set_state(&mut self, state: ThreadState) {
        self.updated_at = Utc::now();
        self.state = state;
    }
}
```

**Step 4: 运行测试验证通过**

Run: `cargo test -p session --lib session --no-default-features sqlx`
Expected: All tests pass

**Step 5: 提交**

```bash
git add session/src/thread.rs session/src/lib.rs
git commit -m "feat(session): add Thread runtime structure"
```
---
## Task 5: 创建 SessionManager
**Files:**
- Create: `session/src/manager.rs`
- Modify: `session/src/lib.rs`

**Step 1: 写 SessionManager 测试**

Create test file: `session/src/manager.rs`

```rust
#[cfg(test)]
fn session_manager_creation() {
    use super::*;
    use crate::manager::SessionManager;
    use crate::store::ThreadStore;
    use sqlx::SqlitePool;

    // Create in-memory database for testing
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    let store = ThreadStore::new(pool);
    let manager = SessionManager::new("user-1".to_string(), store);

    assert_eq!(manager.session_id, "user-1");
    assert_eq!(manager.active_thread_id(), None);
}

#[test]
fn session_manager_create_thread() {
    use super::*;
    use crate::manager::SessionManager;
    use crate::store::ThreadStore;
    use sqlx::SqlitePool;

    let pool = SqlitePool::connect(":memory:").await.unwrap();
    let store = ThreadStore::new(pool);
    let mut manager = SessionManager::new("user-1".to_string(), store);

    let thread = manager.create_thread(Some("Test Thread".to_string())).await.unwrap();
    assert!(manager.active_thread_id().is_some());
    assert_eq!(manager.threads.len(), 1);

    // List threads
    let threads = manager.list_threads().await.unwrap();
    assert_eq!(threads.len(), 1);
}
```

**Step 2: 运行测试验证失败**

Run: `cargo test -p session --lib session --no-default-features sqlx`
Expected: test failures for missing store initialization in SessionManager

**Step 3: 实现 SessionManager 结构**

Add to `session/src/manager.rs`:

```rust
use std::collections::HashMap;
use tokio::sync::{broadcast, Receiver};
use uuid::Uuid;
use crate::store::ThreadStore;
use crate::thread::Thread;
use crate::types::{ThreadEvent, ThreadState, ThreadSummary};

pub struct SessionManager {
    pub session_id: String,
    pub threads: HashMap<Uuid, Thread>,
    pub active_thread_id: Option<Uuid>,
    store: ThreadStore,
    event_sender: broadcast::Sender<ThreadEvent>,
    event_receiver: Receiver<ThreadEvent>,
}

impl SessionManager {
    pub fn new(session_id: String, store: ThreadStore) -> Self {
        Self {
            session_id,
            threads: HashMap::new(),
            active_thread_id: None,
            store,
            event_sender: broadcast::channel(16),
            event_receiver: event_sender.subscribe(),
        }
    }

    pub fn create_thread(&mut self, title: Option<String>) -> impl Result<Uuid, ThreadError> {
        // ... implementation
    }

    pub async fn list_threads(&self) -> impl Result<Vec<ThreadSummary>, ThreadError> {
        // ... implementation
    }

    pub fn subscribe(&self) -> Receiver<ThreadEvent> {
        self.event_receiver.clone()
    }
}
```

**Step 4: 运行测试验证通过**

Run: `cargo test -p session --lib session --no-default-features sqlx`
Expected: All tests pass

**Step 5: 实现 create_thread 和 list_threads 方法**

```rust
pub async fn create_thread(&mut self, title: Option<String>) -> Result<Uuid, ThreadError> {
    let thread = Thread::new(self.session_id.clone(), title);
    thread.set_state(ThreadState::Processing);
    self.threads.insert(thread.id, thread);
    self.active_thread_id = Some(thread.id);

    // Persist to store
    self.store.insert_thread(&thread).await?;

    Ok(thread.id)
}

pub async fn list_threads(&self) -> Result<Vec<ThreadSummary>, ThreadError> {
    let rows = self.store.list_threads(&self.session_id).await?;
    let summaries: Vec<ThreadSummary> = rows.into_iter().map(|row| {
        ThreadSummary {
            id: row.id.parse::<Uuid>().unwrap(),
            title: row.title,
            state: row.state.parse::<ThreadState>().unwrap(),
            created_at: row.created_at.parse::<DateTime>().unwrap(),
            updated_at: row.updated_at.parse::<DateTime>().unwrap(),
        }
    }).collect::<Result<Vec<ThreadSummary>, Error>>)
}

    Ok(summaries)
}
```

**Step 6: 提交**

```bash
git add session/src/manager.rs session/src/lib.rs
git commit -m "feat(session): add SessionManager with create_thread and list_threads"
```
---
## Task 6: 集成到 Tauri 吔
**Files:**
- Modify: `desktop/src-tauri/src/lib.rs`
- Modify: `desktop/src-tauri/Cargo.tom`

**Step 1: 添加 session crate依赖**

在 `desktop/src-tauri/Cargo.tom`:

```toml
[dependencies]
session = { path = "../session", version = "0.1.0" }
```

**Step 2: 添加 Tauri 命注册**

在 `desktop/src-tauri/src/lib.rs`:

```rust
mod commands {
    mod session_manager::SessionManager;
    // Expose to Tauri
}
```

**Step 3: 注册 Tauri 事件监听命令**

在 `desktop/src-tauri/src/lib.rs`:

```rust
mod commands {
    mod session_manager::SessionManager;
    use tauri::Event;
    use crate::session::ThreadEvent;

    #[tauri::command]
    fn subscribe_thread_events(
        manager: State<'session_manager::SessionManager>,
    ) -> Receiver<ThreadEvent> {
        let receiver = manager.subscribe();
        Ok(receiver)
    }
}
```

**Step 4: 运行测试验证 Tauri 命令注册**

Run: `cargo test -p desktop --lib desktop`
Expected: Build succeeds

**Step 5: 提交**

```bash
git add Cargo.toml desktop/src-tauri/
git commit -m "feat(desktop): add session crate dependency and tauri commands"
```
---

## Task 7: 集成 SessionManager 到 Tauri setup 初始化
**Files:**
- Modify: `desktop/src-tauri/build.rs`

**Step 1: 添加 SessionManager 刌态状态管理**

在 `desktop/src-tauri/build.rs`:

```rust
mod app_state {
    fn store_session_manager(state: State<'session_manager::SessionManager>) {
        let session_manager = state.session_manager.lock().unwrap().unwrap();
        *state = session_manager;
    }
}
```

**Step 2: 初始化 SessionManager**

在 `desktop/src-tauri/build.rs` 的 `setup` 函数中:

```rust
use crate::session::manager::SessionManager;
use crate::session::store::ThreadStore;

fn init_session_manager(app: &AppHandle) -> SessionManager {
    let state = app.state::<session_manager::SessionManager>.lock().unwrap().unwrap();
    if state.is_none() {
        let db_pool = SqlitePool::connect("sqlite::argusx.db?mode=Memory&create=true).await.unwrap();
        let store = ThreadStore::new(db_pool);
        let session_id = "default-session".to_string(); // TODO: use proper session ID
        let session_manager = SessionManager::new(session_id, store);
        state.session_manager.insert(session_manager.clone());
        session_manager
    } else {
        state.clone()
    }
}
```

**Step 3: 运行构建验证成功**

Run: `cargo build --manifest desktop --lib desktop`
Expected: Build succeeds

**Step 4: 提交**

```bash
git add desktop/src-tauri/build.rs desktop/src-tauri/Cargo.toml
git commit -m "feat(desktop): integrate SessionManager into app state"
```
---
## Task 8: 实现后台执行和事件通知（Phase 3)
**Files:**
- Modify: `session/src/thread.rs`
- Modify: `session/src/manager.rs`

**Step 1: 添加后台任务支持到 Thread**

在 `session/src/thread.rs`:

```rust
use std::sync::Arc;
use tokio::task::JoinHandle;
// Add to imports

pub struct Thread {
    // ... existing fields ...
    background_task: Option<JoinHandle<()>>,
}

impl Thread {
    pub fn start_background_task(&mut self, task: impl Future<()>) {
        if self.state != ThreadState::Processing {
            return Err(InvalidStateTransition);
        }
        self.state = ThreadState::BackgroundProcessing;
        let handle = tokio::spawn(task);
        self.background_task = Some(handle);
        Ok(())
    }
}
```

**Step 2: 添加事件发送到 SessionManager**

修改 `session/src/manager.rs`:

```rust
// In create_thread or elsewhere when turn completes
self.event_sender
    .send(ThreadEvent::TurnCompleted { thread_id, turn_number })
    .await?;
```

**Step 3: 写后台执行测试**

```rust
#[tokio::test]
async fn test_background_execution() {
    use super::*;
    // Setup with mock store
    let (tx, rx) = tokio::sync::mpsc::channel(32);
    // Create manager with channel for events
    // Spawn background task that sends event
    // Verify event received
}
```

**Step 4: 运行测试**

Run: `cargo test -p session --lib session --features background-exec`
Expected: Test passes

**Step 5: 提交**

```bash
git add -m "feat(session): add background execution and event notifications"
```
---
## Task 9: 巻加前端 Tauri 命令集成 (Phase 3)
**Files:**
- Modify: `desktop/src-tauri/src/lib.rs`

**Step 1: 添加 send_message 命令**

在 `desktop/src-tauri/src/lib.rs`:

```rust
#[tauri::command]
async fn send_message(
    manager: State<'session_manager::SessionManager>,
    thread_id: String,
    content: String,
) -> Result<(), String> {
    let thread_id = Uuid::parse_str(&thread_id)
        .map_err(|_| "Invalid thread ID".to_string())?;
    manager.send_message(thread_id, content).await
        .map_err(|e| e.to_string())
}
```

**Step 2: 添加 switch_thread 命令**

```rust
#[tauri::command]
async fn switch_thread(
    manager: State<'session_manager::SessionManager>,
    thread_id: String,
) -> Result<(), String> {
    let thread_id = Uuid::parse_str(&thread_id)
        .map_err(|_| "Invalid thread ID".to_string())?;
    manager.switch_thread(thread_id).await
        .map_err(|e| e.to_string())
}
```

**Step 3: 运行构建验证**

Run: `cargo build --manifest-path desktop/src-tauri/Cargo.toml`
Expected: Build succeeds

**Step 4: 提交**

```bash
git add -m "feat(desktop): add send_message and switch_thread commands"
```
---
## Task 10: 添加测试覆盖 (Phase 4)
**Files:**
- Create: `session/src/tests/integration_test.rs`

**Step 1: 写集成测试**

```rust
// Full integration test with SQLite
// Test complete flow: create thread -> send message -> switch -> background execution
```

**Step 2: 运行测试**

Run: `cargo test -p session --lib session --test integration`
Expected: Tests pass

**Step 3: 提交**

```bash
git add -m "test(session): add integration tests"
```
---

## 执行检查点

每个 Phase 完成后运行:
1. `cargo test --workspace argusx`
2. `cargo clippy --workspace argusx -- -D warnings`

确保所有测试通过且无 clippy 警告。

---

## 实现顺序

1. Task 1-2 (Phase 1) - 基础类型
2. Task 3-6 (Phase 2) - ThreadStore 和 SessionManager
3. Task 7 (Phase 2) - Tauri 集成
4. Task 8 (Phase 3) - 后台执行和事件
5. Task 9 (Phase 3) - 前端命令
6. Task 10 (Phase 4) - 测试覆盖

---

## 验收标准

1. 所有单元测试通过
2. 所有集成测试通过
3. `cargo clippy` 无警告
4. 手动测试 Tauri 应用创建/切换 Thread 功能正常
