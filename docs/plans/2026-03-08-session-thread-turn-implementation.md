# Session / Thread / Turn 重构 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在复用现有 `turn` 单轮执行引擎的前提下，实现 `Session -> Thread -> Turn` 三层模型，支持多轮历史参与下一轮上下文、thread 切换后台继续执行，以及重启后仅恢复已落盘历史。

**Architecture:** 先扩展 `turn` crate 的边界，让它能接受历史消息并产出稳定 `TurnOutcome`；再在 `session` crate 中建立新的持久化模型、运行时模型和 `SessionManager` 编排层；最后把 `desktop` 端接到 `SessionManager`，只包装 thread 级命令和事件，不复刻 turn 的状态机与事件协议。

**Tech Stack:** Rust 2024, Tokio, sqlx/sqlite, Tauri 2, serde/serde_json, uuid, chrono

---

## Plan Rules

- 参考设计文档：`docs/plans/2026-03-08-session-thread-turn-design.md`
- 参考技能：`@rust-router`, `@test-driven-development`, `@verification-before-completion`
- 每一项任务都先写失败测试，再做最小实现，再运行最小验证命令
- 不沿用旧 implementation 文档中的 `ThreadState/ThreadEvent/AssistantResponse/ToolCallRecord` 旧建模
- 不修改 `sql/schema.sql`，新增 session 专用 schema 文件，避免把 SQLite schema 混进现有 ClickHouse telemetry schema

---

### Task 1: 修正 `session` crate 基线并移除旧模型残留

**Files:**
- Modify: `session/src/lib.rs`
- Modify: `session/src/types.rs`
- Modify: `session/src/thread.rs`
- Modify: `session/src/store.rs`
- Modify: `session/src/manager.rs`
- Modify: `session/src/tests.rs`
- Test: `session/src/tests.rs`

**Step 1: 写一个最小编译测试，锁定新模块边界**

在 `session/src/tests.rs` 写一个 smoke test，要求新模块至少能被 crate 引入：

```rust
use crate::{manager::SessionManager, thread::ThreadRuntime, types::ThreadRecord};

#[test]
fn session_crate_exports_new_domain_types() {
    let _ = std::any::type_name::<SessionManager>();
    let _ = std::any::type_name::<ThreadRuntime>();
    let _ = std::any::type_name::<ThreadRecord>();
}
```

**Step 2: 运行测试，确认当前基线失败**

Run: `cargo test -p session session_crate_exports_new_domain_types -- --exact`
Expected: FAIL，因为当前 `session` 里仍然是旧模型残留，且 `manager/store/thread` 的类型边界不匹配

**Step 3: 将 `session` crate 调整为新边界的空骨架**

目标不是一次性实现功能，而是先让下面这些符号存在并且旧残留被删掉：

- `types.rs` 中保留或新增：`SessionRecord`, `ThreadRecord`, `ThreadLifecycle`, `TurnRecord`, `TurnStatus`, `PersistedMessage`, `PersistedToolCall`, `ThreadEvent`, `ThreadViewState`
- `thread.rs` 中保留或新增：`ThreadRuntime`, `ActiveTurnRuntime`
- `store.rs` 中保留或新增：`ThreadStore`
- `manager.rs` 中保留或新增：`SessionManager`
- 删除旧设计残留：`ThreadState`, `AssistantResponse`, 旧版 `ToolCallRecord`, 旧版 `Thread` 结构

示例骨架：

```rust
pub struct SessionManager;

pub struct ThreadRuntime {
    pub thread_id: uuid::Uuid,
    pub active_turn: Option<ActiveTurnRuntime>,
}

pub struct ActiveTurnRuntime {
    pub turn_id: uuid::Uuid,
    pub turn_number: u32,
}
```

**Step 4: 再次运行测试，确认 smoke test 通过**

Run: `cargo test -p session session_crate_exports_new_domain_types -- --exact`
Expected: PASS

**Step 5: 提交**

```bash
git add session/src/lib.rs session/src/types.rs session/src/thread.rs session/src/store.rs session/src/manager.rs session/src/tests.rs
git commit -m "refactor(session): reset crate surface for v2 architecture"
```

---

### Task 2: 为 `turn` 增加“历史注入”输入边界

**Files:**
- Modify: `turn/src/context.rs`
- Modify: `turn/src/driver.rs`
- Modify: `turn/src/lib.rs`
- Test: `turn/tests/transcript_turn_test.rs`

**Step 1: 写失败测试，证明新一轮可接收 thread 历史**

在 `turn/tests/transcript_turn_test.rs` 新增一个测试，要求首个 LLM request 的 messages 已经包含 prior history：

```rust
#[tokio::test]
async fn first_step_receives_prior_messages_before_current_user_input() {
    use std::sync::Arc;
    use argus_core::{FinishReason, ResponseEvent, Usage};
    use turn::{TurnDriver, TurnEvent, TurnMessage, TurnSeed};

    let model = Arc::new(support::FakeModelRunner::new(vec![vec![
        ResponseEvent::ContentDelta("done".into()),
        ResponseEvent::Done {
            reason: FinishReason::Stop,
            usage: Some(Usage::zero()),
        },
    ]]));
    let model_ref = Arc::clone(&model);

    let seed = TurnSeed {
        session_id: "session-1".into(),
        turn_id: "turn-2".into(),
        prior_messages: vec![
            TurnMessage::User { content: "hello".into() },
            TurnMessage::AssistantText { content: "hi".into() },
        ],
        user_message: "continue".into(),
    };

    let (handle, task) = TurnDriver::spawn(
        seed,
        model,
        Arc::new(support::instant_tool_runner()),
        Arc::new(support::FakeAuthorizer::default()),
        Arc::new(support::FakeObserver),
    );

    collect_events(handle).await;
    task.await.unwrap().unwrap();

    let requests = model_ref.received_requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].messages.len(), 3);
    assert!(matches!(message_at(&requests[0].messages, 0), TurnMessage::User { content } if content.as_ref() == "hello"));
    assert!(matches!(message_at(&requests[0].messages, 1), TurnMessage::AssistantText { content } if content.as_ref() == "hi"));
    assert!(matches!(message_at(&requests[0].messages, 2), TurnMessage::User { content } if content.as_ref() == "continue"));
}
```

**Step 2: 运行测试，确认失败**

Run: `cargo test -p turn first_step_receives_prior_messages_before_current_user_input -- --exact`
Expected: FAIL，因为当前 `TurnContext` 只有 `user_message`

**Step 3: 引入新的输入结构并让 `TurnDriver` 使用它**

在 `turn/src/context.rs` 中把旧 `TurnContext` 升级为新输入边界：

```rust
use crate::TurnMessage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnSeed {
    pub session_id: String,
    pub turn_id: String,
    pub prior_messages: Vec<TurnMessage>,
    pub user_message: String,
}
```

在 `turn/src/driver.rs` 中：

- 将 `context: TurnContext` 改为 `seed: TurnSeed`
- 在 `run()` 开头先把 `prior_messages` 逐个 push 进 transcript
- 最后再 push 当前 `user_message`
- 生成 `LlmStepRequest` 时仍旧使用 `transcript.snapshot()`

伪代码：

```rust
for message in &self.seed.prior_messages {
    self.transcript.push(message.clone());
}
self.transcript.push(TurnMessage::User {
    content: self.seed.user_message.as_str().into(),
});
```

同时更新 `turn/src/lib.rs` 导出：

```rust
pub use context::TurnSeed;
```

**Step 4: 修正现有测试和调用点**

将所有现有 `TurnContext { ... }` 调用替换为 `TurnSeed { prior_messages: vec![], ... }`

**Step 5: 运行验证**

Run: `cargo test -p turn transcript_turn_test --test transcript_turn_test`
Expected: PASS，且现有 transcript 行为回归测试全部继续通过

**Step 6: 提交**

```bash
git add turn/src/context.rs turn/src/driver.rs turn/src/lib.rs turn/tests/transcript_turn_test.rs turn/tests/*.rs
git commit -m "feat(turn): support seeded turn history"
```

---

### Task 3: 为 `turn` 增加稳定的 `TurnOutcome`

**Files:**
- Create: `turn/src/outcome.rs`
- Modify: `turn/src/driver.rs`
- Modify: `turn/src/lib.rs`
- Modify: `turn/src/transcript.rs`
- Test: `turn/tests/text_only_turn_test.rs`
- Test: `turn/tests/tool_batch_turn_test.rs`

**Step 1: 写失败测试，要求任务完成后能拿到最终 transcript 和 final output**

在 `turn/tests/text_only_turn_test.rs` 新增：

```rust
#[tokio::test]
async fn completed_turn_returns_transcript_and_final_output() {
    use std::sync::Arc;
    use argus_core::{FinishReason, ResponseEvent, Usage};
    use turn::{TurnDriver, TurnFinishReason, TurnMessage, TurnSeed};

    let model = Arc::new(support::FakeModelRunner::new(vec![vec![
        ResponseEvent::ContentDelta("hello".into()),
        ResponseEvent::ContentDelta(" world".into()),
        ResponseEvent::Done {
            reason: FinishReason::Stop,
            usage: Some(Usage::zero()),
        },
    ]]));

    let (handle, task) = TurnDriver::spawn(
        TurnSeed {
            session_id: "session-1".into(),
            turn_id: "turn-1".into(),
            prior_messages: vec![],
            user_message: "say hello".into(),
        },
        model,
        Arc::new(support::instant_tool_runner()),
        Arc::new(support::FakeAuthorizer::default()),
        Arc::new(support::FakeObserver),
    );

    collect_events(handle).await;
    let outcome = task.await.unwrap().unwrap();

    assert_eq!(outcome.finish_reason, TurnFinishReason::Completed);
    assert_eq!(outcome.final_output.as_deref(), Some("hello world"));
    assert!(matches!(outcome.transcript.last(), Some(TurnMessage::AssistantText { content }) if content.as_ref() == "hello world"));
}
```

**Step 2: 运行测试，确认失败**

Run: `cargo test -p turn completed_turn_returns_transcript_and_final_output -- --exact`
Expected: FAIL，因为当前任务返回 `Result<(), TurnError>`，没有 `TurnOutcome`

**Step 3: 实现 `TurnOutcome` 并在 driver 中维护最终 assistant 文本**

创建 `turn/src/outcome.rs`：

```rust
use crate::{TurnFinishReason, TurnMessage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnOutcome {
    pub turn_id: String,
    pub finish_reason: TurnFinishReason,
    pub transcript: Vec<TurnMessage>,
    pub final_output: Option<String>,
}
```

在 `turn/src/driver.rs`：

- 把 `spawn` 返回值改为 `JoinHandle<Result<TurnOutcome, TurnError>>`
- 为当前 step 增加 assistant text buffer
- 在 `FinishReason::Stop` 时把累计文本 push 成 `TurnMessage::AssistantText`
- `finish(...)` 返回 `TurnOutcome`，而不是 `()`

辅助方法建议加入 `transcript.rs`：

```rust
impl TurnTranscript {
    pub fn to_vec(&self) -> Vec<TurnMessage> {
        self.messages.iter().map(|m| m.as_ref().clone()).collect()
    }
}
```

**Step 4: 运行最小验证**

Run: `cargo test -p turn completed_turn_returns_transcript_and_final_output -- --exact`
Expected: PASS

**Step 5: 运行回归测试**

Run: `cargo test -p turn`
Expected: PASS

**Step 6: 提交**

```bash
git add turn/src/outcome.rs turn/src/driver.rs turn/src/lib.rs turn/src/transcript.rs turn/tests/text_only_turn_test.rs turn/tests/tool_batch_turn_test.rs turn/tests/*.rs
git commit -m "feat(turn): return final outcome with transcript"
```

---

### Task 4: 定义 `session` 的持久化模型与线程级事件包装

**Files:**
- Modify: `session/src/types.rs`
- Modify: `session/src/lib.rs`
- Test: `session/src/types.rs`

**Step 1: 写序列化测试，锁定新模型**

在 `session/src/types.rs` 中新增测试：

```rust
#[test]
fn turn_record_round_trips_with_transcript() {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    let record = TurnRecord {
        id: Uuid::new_v4(),
        thread_id: Uuid::new_v4(),
        turn_number: 2,
        user_input: "continue".into(),
        status: TurnStatus::Completed,
        finish_reason: Some("Completed".into()),
        transcript: vec![
            PersistedMessage::User { content: "hello".into() },
            PersistedMessage::AssistantText { content: "hi".into() },
        ],
        final_output: Some("hi".into()),
        started_at: Utc::now(),
        finished_at: Some(Utc::now()),
    };

    let json = serde_json::to_string(&record).unwrap();
    let decoded: TurnRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.turn_number, 2);
    assert_eq!(decoded.transcript.len(), 2);
}
```

**Step 2: 运行测试，确认失败**

Run: `cargo test -p session turn_record_round_trips_with_transcript -- --exact`
Expected: FAIL，因为当前 `types.rs` 还是旧版 turn record

**Step 3: 按设计文档重写 `types.rs`**

需要至少包含：

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThreadLifecycle {
    Open,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TurnStatus {
    Running,
    WaitingPermission,
    Completed,
    Cancelled,
    Failed,
    Interrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PersistedMessage {
    User { content: String },
    AssistantText { content: String },
    AssistantToolCalls { content: Option<String>, calls: Vec<PersistedToolCall> },
    ToolResult { call_id: String, tool_name: String, content: String, is_error: bool },
    SystemNote { content: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThreadEventEnvelope {
    pub thread_id: uuid::Uuid,
    pub turn_id: Option<uuid::Uuid>,
    pub event: ThreadEvent,
}
```

同时定义稳定的：

- `SessionRecord`
- `ThreadRecord`
- `TurnRecord`
- `PersistedToolCall`
- `ThreadViewState`
- `ThreadEvent`

**Step 4: 运行验证**

Run: `cargo test -p session types --lib`
Expected: PASS

**Step 5: 提交**

```bash
git add session/src/types.rs session/src/lib.rs
git commit -m "feat(session): define v2 persistence and event types"
```

---

### Task 5: 新建 session 专用 SQLite schema 和 `ThreadStore`

**Files:**
- Create: `sql/session_schema.sql`
- Modify: `session/src/store.rs`
- Modify: `session/src/types.rs`
- Test: `session/src/store.rs`

**Step 1: 写 store 测试，锁定新表结构与历史回放读取**

在 `session/src/store.rs` 新增测试：

```rust
#[tokio::test]
async fn store_round_trips_thread_and_turn_history() {
    use chrono::Utc;
    use sqlx::SqlitePool;
    use uuid::Uuid;

    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let store = ThreadStore::new(pool);
    store.init_schema().await.unwrap();

    let session = SessionRecord {
        id: "session-1".into(),
        user_id: None,
        default_model: "gpt-5".into(),
        system_prompt: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    store.upsert_session(&session).await.unwrap();

    let thread = ThreadRecord {
        id: Uuid::new_v4(),
        session_id: session.id.clone(),
        title: Some("Test".into()),
        lifecycle: ThreadLifecycle::Open,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_turn_number: 1,
    };
    store.insert_thread(&thread).await.unwrap();

    let turn = TurnRecord {
        id: Uuid::new_v4(),
        thread_id: thread.id,
        turn_number: 1,
        user_input: "hello".into(),
        status: TurnStatus::Completed,
        finish_reason: Some("Completed".into()),
        transcript: vec![PersistedMessage::User { content: "hello".into() }],
        final_output: Some("hi".into()),
        started_at: Utc::now(),
        finished_at: Some(Utc::now()),
    };
    store.insert_turn(&turn).await.unwrap();

    let history = store.list_turns(thread.id).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].final_output.as_deref(), Some("hi"));
}
```

**Step 2: 运行测试，确认失败**

Run: `cargo test -p session store_round_trips_thread_and_turn_history -- --exact`
Expected: FAIL，因为 `ThreadStore` 还是旧 schema 和旧字段

**Step 3: 创建新的 SQLite schema 文件**

在 `sql/session_schema.sql` 中定义至少三张表：

```sql
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT,
    default_model TEXT NOT NULL,
    system_prompt TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS threads (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    title TEXT,
    lifecycle TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_turn_number INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS turns (
    id TEXT PRIMARY KEY,
    thread_id TEXT NOT NULL,
    turn_number INTEGER NOT NULL,
    user_input TEXT NOT NULL,
    status TEXT NOT NULL,
    finish_reason TEXT,
    transcript_json TEXT NOT NULL,
    final_output TEXT,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    FOREIGN KEY (thread_id) REFERENCES threads(id) ON DELETE CASCADE
);
```

**Step 4: 重写 `ThreadStore`**

要求：

- `init_schema()` 从 `sql/session_schema.sql` 读入并执行
- 提供 `upsert_session`, `insert_thread`, `update_thread`, `insert_turn`, `update_turn`, `list_threads`, `list_turns`, `get_thread`, `mark_incomplete_turns_interrupted`
- `transcript_json` 用 `serde_json` 统一读写 `Vec<PersistedMessage>`

**Step 5: 运行验证**

Run: `cargo test -p session store_round_trips_thread_and_turn_history -- --exact`
Expected: PASS

**Step 6: 跑 store 全量测试**

Run: `cargo test -p session store --lib`
Expected: PASS

**Step 7: 提交**

```bash
git add sql/session_schema.sql session/src/store.rs session/src/types.rs
git commit -m "feat(session): add sqlite store for sessions threads and turns"
```

---

### Task 6: 实现 `ThreadRuntime` 和历史回放拼装

**Files:**
- Modify: `session/src/thread.rs`
- Modify: `session/src/types.rs`
- Test: `session/src/thread.rs`

**Step 1: 写失败测试，锁定 history flatten 行为**

在 `session/src/thread.rs` 新增测试：

```rust
#[test]
fn thread_runtime_flattens_completed_turn_history_in_order() {
    use crate::types::{PersistedMessage, TurnRecord, TurnStatus};
    use chrono::Utc;
    use uuid::Uuid;

    let thread_id = Uuid::new_v4();
    let runtime = ThreadRuntime::new(thread_id);

    let turns = vec![
        TurnRecord {
            id: Uuid::new_v4(),
            thread_id,
            turn_number: 1,
            user_input: "hello".into(),
            status: TurnStatus::Completed,
            finish_reason: Some("Completed".into()),
            transcript: vec![
                PersistedMessage::User { content: "hello".into() },
                PersistedMessage::AssistantText { content: "hi".into() },
            ],
            final_output: Some("hi".into()),
            started_at: Utc::now(),
            finished_at: Some(Utc::now()),
        },
        TurnRecord {
            id: Uuid::new_v4(),
            thread_id,
            turn_number: 2,
            user_input: "next".into(),
            status: TurnStatus::Interrupted,
            finish_reason: None,
            transcript: vec![PersistedMessage::User { content: "partial".into() }],
            final_output: None,
            started_at: Utc::now(),
            finished_at: None,
        },
    ];

    let prior = runtime.build_prior_messages(&turns);
    assert_eq!(prior.len(), 2);
}
```

**Step 2: 运行测试，确认失败**

Run: `cargo test -p session thread_runtime_flattens_completed_turn_history_in_order -- --exact`
Expected: FAIL，因为 `ThreadRuntime` 还没有 history replay 逻辑

**Step 3: 实现 `ThreadRuntime` 和转换函数**

要求：

- `ThreadRuntime::new(thread_id)`
- `build_prior_messages(&self, turns: &[TurnRecord]) -> Vec<turn::TurnMessage>`
- 仅回放 `Completed / Cancelled / Failed` turns
- 忽略 `Interrupted / Running / WaitingPermission`
- 将 `PersistedMessage` 转换为 `turn::TurnMessage`

建议结构：

```rust
pub struct ThreadRuntime {
    pub thread_id: Uuid,
    pub active_turn: Option<ActiveTurnRuntime>,
}

impl ThreadRuntime {
    pub fn new(thread_id: Uuid) -> Self {
        Self { thread_id, active_turn: None }
    }

    pub fn build_prior_messages(&self, turns: &[TurnRecord]) -> Vec<turn::TurnMessage> {
        turns.iter()
            .filter(|turn| matches!(turn.status, TurnStatus::Completed | TurnStatus::Cancelled | TurnStatus::Failed))
            .flat_map(|turn| turn.transcript.iter())
            .map(persisted_to_turn_message)
            .collect()
    }
}
```

**Step 4: 运行验证**

Run: `cargo test -p session thread_runtime_flattens_completed_turn_history_in_order -- --exact`
Expected: PASS

**Step 5: 提交**

```bash
git add session/src/thread.rs session/src/types.rs
git commit -m "feat(session): build thread runtime history replay"
```

---

### Task 7: 实现 `SessionManager` 的 thread 编排和 event bridge

**Files:**
- Modify: `session/src/manager.rs`
- Modify: `session/src/thread.rs`
- Modify: `session/src/store.rs`
- Modify: `session/src/types.rs`
- Test: `session/src/manager.rs`
- Test: `session/tests/session_manager_flow.rs`

**Step 1: 写失败测试，覆盖 create/list/switch/send 的核心流程**

创建 `session/tests/session_manager_flow.rs`：

```rust
use sqlx::SqlitePool;
use uuid::Uuid;
use session::{manager::SessionManager, store::ThreadStore};

#[tokio::test]
async fn create_thread_switch_thread_and_list_history() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let store = ThreadStore::new(pool);
    store.init_schema().await.unwrap();

    let mut manager = SessionManager::new("session-1".into(), store);
    let first = manager.create_thread(Some("A".into())).await.unwrap();
    let second = manager.create_thread(Some("B".into())).await.unwrap();

    manager.switch_thread(first).await.unwrap();
    assert_eq!(manager.active_thread_id(), Some(first));

    let threads = manager.list_threads().await.unwrap();
    assert_eq!(threads.len(), 2);
    assert!(threads.iter().any(|t| t.id == second));
}
```

**Step 2: 运行测试，确认失败**

Run: `cargo test -p session create_thread_switch_thread_and_list_history -- --exact`
Expected: FAIL，因为 `SessionManager` 还没有新边界下的实现

**Step 3: 实现 `SessionManager` 的只编排不执行重复逻辑原则**

要求：

- `new(session_id, store)`
- `active_thread_id()`
- `create_thread(title)`
- `switch_thread(thread_id)`
- `list_threads()`
- `load_thread_history(thread_id)`
- `send_message(thread_id, content, deps)`
- `resolve_permission(thread_id, request_id, decision)`
- `cancel_turn(thread_id)`
- `subscribe()`

其中 `send_message` 的核心流程必须是：

```rust
let history = self.store.list_turns(thread_id).await?;
let prior_messages = thread_runtime.build_prior_messages(&history);
let seed = turn::TurnSeed {
    session_id: self.session_id.clone(),
    turn_id: turn_id.to_string(),
    prior_messages,
    user_message: content,
};
let (handle, task) = TurnDriver::spawn(seed, model, tool_runner, authorizer, observer);
```

事件桥接要求：

- `TurnEvent` 不降级重写
- 包装成 `ThreadEvent::TurnEvent { thread_id, turn_id, event }`
- 收到 `ToolCallPermissionRequested` 时更新 `ActiveTurnRuntime.waiting_permission`
- 任务结束后持久化 `TurnOutcome`

**Step 4: 为 manager 增加一个 mockable 依赖包裹结构**

不要把模型、工具、审批器直接硬编码进 manager。建议定义：

```rust
pub struct TurnDependencies {
    pub model: Arc<dyn turn::ModelRunner>,
    pub tool_runner: Arc<dyn turn::ToolRunner>,
    pub authorizer: Arc<dyn turn::ToolAuthorizer>,
    pub observer: Arc<dyn turn::TurnObserver>,
}
```

这样测试可以注入 fake model/tool/authorizer。

**Step 5: 运行验证**

Run: `cargo test -p session create_thread_switch_thread_and_list_history -- --exact`
Expected: PASS

**Step 6: 跑 manager 全量测试**

Run: `cargo test -p session manager --lib && cargo test -p session --test session_manager_flow`
Expected: PASS

**Step 7: 提交**

```bash
git add session/src/manager.rs session/src/thread.rs session/src/store.rs session/src/types.rs session/tests/session_manager_flow.rs
git commit -m "feat(session): orchestrate threads and turns via session manager"
```

---

### Task 8: 处理应用重启与未完成 turn 的恢复策略

**Files:**
- Modify: `session/src/store.rs`
- Modify: `session/src/manager.rs`
- Test: `session/src/store.rs`
- Test: `session/tests/session_resume_test.rs`

**Step 1: 写失败测试，验证重启时未完成 turn 被标记为 `Interrupted`**

创建 `session/tests/session_resume_test.rs`：

```rust
use chrono::Utc;
use session::types::{PersistedMessage, ThreadLifecycle, ThreadRecord, TurnRecord, TurnStatus};
use session::store::ThreadStore;
use sqlx::SqlitePool;
use uuid::Uuid;

#[tokio::test]
async fn manager_marks_incomplete_turns_interrupted_on_startup() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let store = ThreadStore::new(pool);
    store.init_schema().await.unwrap();

    // seed a running turn
    let thread_id = Uuid::new_v4();
    store.upsert_session(&session::types::SessionRecord {
        id: "session-1".into(),
        user_id: None,
        default_model: "gpt-5".into(),
        system_prompt: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }).await.unwrap();
    store.insert_thread(&ThreadRecord {
        id: thread_id,
        session_id: "session-1".into(),
        title: Some("A".into()),
        lifecycle: ThreadLifecycle::Open,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_turn_number: 1,
    }).await.unwrap();
    let turn_id = Uuid::new_v4();
    store.insert_turn(&TurnRecord {
        id: turn_id,
        thread_id,
        turn_number: 1,
        user_input: "hello".into(),
        status: TurnStatus::Running,
        finish_reason: None,
        transcript: vec![PersistedMessage::User { content: "hello".into() }],
        final_output: None,
        started_at: Utc::now(),
        finished_at: None,
    }).await.unwrap();

    store.mark_incomplete_turns_interrupted().await.unwrap();
    let turns = store.list_turns(thread_id).await.unwrap();
    assert!(matches!(turns[0].status, TurnStatus::Interrupted));
}
```

**Step 2: 运行测试，确认失败**

Run: `cargo test -p session manager_marks_incomplete_turns_interrupted_on_startup -- --exact`
Expected: FAIL

**Step 3: 在 store 和 manager 中实现启动恢复逻辑**

要求：

- `ThreadStore::mark_incomplete_turns_interrupted()` 将 `Running` / `WaitingPermission` 统一更新为 `Interrupted`
- `SessionManager::initialize()` 或等效启动入口中调用该逻辑
- 初始化完成后不恢复任何 `ActiveTurnRuntime`

**Step 4: 运行验证**

Run: `cargo test -p session --test session_resume_test`
Expected: PASS

**Step 5: 提交**

```bash
git add session/src/store.rs session/src/manager.rs session/tests/session_resume_test.rs
git commit -m "feat(session): mark incomplete turns interrupted on startup"
```

---

### Task 9: 接入 Tauri 状态、命令和 thread 级事件转发

**Files:**
- Modify: `desktop/src-tauri/Cargo.toml`
- Modify: `desktop/src-tauri/src/lib.rs`
- Create: `desktop/src-tauri/src/session_commands.rs`
- Test: `desktop/src-tauri/src/lib.rs`

**Step 1: 写一个最小桌面编译测试目标**

先不做 UI 测试，先保证桌面端能编译出新的 command surface。给 `desktop/src-tauri/src/lib.rs` 增加一个最小 smoke test 模块：

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn desktop_lib_builds() {
        assert_eq!(2 + 2, 4);
    }
}
```

**Step 2: 运行现状验证**

Run: `cargo check -p desktop`
Expected: 现状可能失败或尚未接入 `session` 依赖，但该命令作为后续回归基线

**Step 3: 添加 `session` / `turn` 依赖并注入 manager state**

在 `desktop/src-tauri/Cargo.toml` 添加：

```toml
session = { path = "../../session" }
turn = { path = "../../turn" }
sqlx = { workspace = true, features = ["runtime-tokio", "sqlite"] }
uuid = { workspace = true, features = ["v4"] }
tokio = { workspace = true, features = ["rt-multi-thread", "macros", "sync"] }
```

在 `desktop/src-tauri/src/session_commands.rs` 中定义命令：

```rust
#[tauri::command]
async fn create_thread(...) -> Result<String, String>;

#[tauri::command]
async fn list_threads(...) -> Result<Vec<ThreadSummaryDto>, String>;

#[tauri::command]
async fn switch_thread(...) -> Result<(), String>;

#[tauri::command]
async fn send_message(...) -> Result<(), String>;

#[tauri::command]
async fn resolve_thread_permission(...) -> Result<(), String>;

#[tauri::command]
async fn cancel_thread_turn(...) -> Result<(), String>;
```

注意：

- 不直接把 `tokio::sync::Receiver` 暴露给 Tauri command 返回值
- 通过 `app.emit("thread-event", payload)` 或等效方式转发事件
- 状态中放 `Arc<tokio::sync::Mutex<SessionManager>>`

**Step 4: 在 `run()` 的 builder 中初始化 store 和 manager**

伪代码：

```rust
let pool = tokio::runtime::Handle::current().block_on(async {
    sqlx::SqlitePool::connect("sqlite:argusx.db").await
})?;
let store = session::store::ThreadStore::new(pool);
let manager = session::manager::SessionManager::new("default-session".into(), store);
```

同时在 setup 阶段调用 session 初始化逻辑，而不是写进 `build.rs`。

**Step 5: 运行验证**

Run: `cargo check -p desktop`
Expected: PASS

**Step 6: 提交**

```bash
git add desktop/src-tauri/Cargo.toml desktop/src-tauri/src/lib.rs desktop/src-tauri/src/session_commands.rs
git commit -m "feat(desktop): integrate session manager commands and event bridge"
```

---

### Task 10: 增加跨 crate 集成测试和最终验证

**Files:**
- Create: `session/tests/thread_background_flow_test.rs`
- Modify: `turn/tests/support/mod.rs`
- Modify: `session/src/manager.rs`
- Test: `session/tests/thread_background_flow_test.rs`

**Step 1: 写端到端测试，覆盖后台执行与 thread 切换语义**

创建 `session/tests/thread_background_flow_test.rs`：

```rust
#[tokio::test]
async fn switching_active_thread_does_not_cancel_running_turn() {
    // 1. create manager + two threads
    // 2. start a slow turn in thread A
    // 3. switch to thread B before A completes
    // 4. assert thread A still finishes successfully
    // 5. assert completed turn is persisted and listed in history
}
```

要求断言：

- `switch_thread()` 不会调用 cancel
- 慢速 turn 最终能写回 `Completed`
- 切回原 thread 后能读到完整 transcript / final_output

**Step 2: 运行测试，确认失败**

Run: `cargo test -p session switching_active_thread_does_not_cancel_running_turn -- --exact`
Expected: FAIL，直到后台语义真正跑通

**Step 3: 只做最小修正让测试通过**

如果当前实现仍把“后台执行”建模成持久化状态或显式迁移执行器，删掉这些多余逻辑，保持：

- turn 一直跑
- `active_thread_id` 改变
- UI 视图状态由 manager 派生

**Step 4: 运行 session 全量测试**

Run: `cargo test -p session`
Expected: PASS

**Step 5: 运行 turn 全量测试**

Run: `cargo test -p turn`
Expected: PASS

**Step 6: 运行桌面编译检查**

Run: `cargo check -p desktop`
Expected: PASS

**Step 7: 工作区最终验证**

Run: `cargo test --workspace`
Expected: PASS

Run: `cargo clippy --workspace -- -D warnings`
Expected: PASS

**Step 8: 提交**

```bash
git add session/tests/thread_background_flow_test.rs session/src/manager.rs turn/tests/support/mod.rs
git commit -m "test(session): cover thread background execution flow"
```

---

## 执行顺序

1. Task 1: 清理 `session` 旧基线
2. Task 2: `turn` 支持历史注入
3. Task 3: `turn` 产出 `TurnOutcome`
4. Task 4: `session` 新模型定义
5. Task 5: SQLite store 与 schema
6. Task 6: `ThreadRuntime` 历史回放
7. Task 7: `SessionManager` 编排
8. Task 8: 重启恢复策略
9. Task 9: Tauri 集成
10. Task 10: 跨 crate 集成验证

## 验收标准

1. `turn` 首轮请求支持注入 thread 历史
2. `turn` 完成后可产出稳定 `TurnOutcome`
3. `session` 仅保存一份可回放 transcript 真相源
4. 切换 thread 不会取消正在运行的 turn
5. 应用重启后只恢复已落盘历史，未完成 turn 统一变为 `Interrupted`
6. 前端通过 thread 级事件包装复用现有 `TurnEvent`
7. `cargo test --workspace` 通过
8. `cargo clippy --workspace -- -D warnings` 通过
