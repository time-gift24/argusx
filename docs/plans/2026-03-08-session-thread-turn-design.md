# Session -> Thread -> Turn 重构设计

日期: 2026-03-08

## 概述

将当前的单 Turn 结构重构为 Session -> Thread -> Turn 三层结构，支持：
- 多轮对话历史持久化
- 任务隔离（多 Thread 管理）
- Thread 切换时后台继续执行

## 架构

```
┌─────────────────────────────────────────────────────────────┐
│                        Frontend (Tauri)                      │
│  持有 SessionManager，监听 Thread 事件                       │
└─────────────────────────┬───────────────────────────────────┘
                          │ Tauri Commands + Events
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    SessionManager (新增)                     │
│  - session_id: String (用户标识)                             │
│  - threads: HashMap<Uuid, Thread>                           │
│  - active_thread_id: Option<Uuid>                           │
│  - create_thread() / switch_thread() / list_threads()       │
│  - subscribe_thread_events() -> Receiver<ThreadEvent>       │
└─────────────────────────┬───────────────────────────────────┘
                          │
          ┌───────────────┴───────────────┐
          ▼                               ▼
┌─────────────────────┐         ┌─────────────────────┐
│   Thread (新增)      │         │   ThreadStore (新增) │
│  - id, title, meta  │         │   SQLite 持久化      │
│  - state: ThreadState│        │   - save_turn()     │
│  - current_turn:    │         │   - load_history()  │
│    Option<TurnDriver│         │   - list_threads() │
│  - history: Vec<TurnRecord>    └─────────────────────┘
└─────────────────────┘
          │
          ▼ (复用现有)
┌─────────────────────┐
│   TurnDriver        │
│   TurnTranscript    │
│   TurnState         │
└─────────────────────┘
```

## 事件流

```
Thread (后台执行)
    → TurnDriver emit TurnEvent
    → Thread 转换为 ThreadEvent
    → SessionManager broadcast 给所有 subscriber
    → Frontend 收到通知更新 UI
```

## 核心数据结构

### ThreadState

```rust
pub enum ThreadState {
    Idle,                    // 空闲，等待用户输入
    Processing,              // 正在处理 Turn
    BackgroundProcessing,    // 后台执行（用户切换走了）
    WaitingForPermission,    // 等待工具权限确认
    Completed,               // 用户主动关闭
    Failed(String),          // 错误状态
}
```

### Thread

```rust
pub struct Thread {
    pub id: Uuid,
    pub session_id: String,
    pub title: Option<String>,
    pub state: ThreadState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // 运行时状态
    current_turn: Option<TurnDriver>,
    history: Vec<TurnRecord>,

    // 后台任务
    background_task: Option<JoinHandle<()>>,
}
```

### TurnRecord（持久化用）

```rust
pub struct TurnRecord {
    pub turn_number: usize,
    pub user_input: String,
    pub assistant_response: Option<String>,
    pub tool_calls: Vec<ToolCallRecord>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub state: TurnRecordState,  // Completed / Failed / Interrupted
}

pub struct ToolCallRecord {
    pub call_id: String,
    pub tool_name: String,
    pub arguments: String,
    pub result: Option<String>,
    pub is_error: bool,
}
```

### ThreadEvent（前端订阅）

```rust
pub enum ThreadEvent {
    Created { thread_id: Uuid },
    Activated { thread_id: Uuid },
    TurnStarted { thread_id: Uuid, turn_number: usize },
    TurnProgress { thread_id: Uuid, message: TurnMessage },
    TurnCompleted { thread_id: Uuid, turn_number: usize },
    StateChanged { thread_id: Uuid, old_state: ThreadState, new_state: ThreadState },
    Deleted { thread_id: Uuid },
}
```

## 核心数据流

### 1. 创建 Thread

```
Frontend: session_manager.create_thread(title?)
    │
    ▼
SessionManager:
    │ 1. 生成 Uuid
    │ 2. ThreadStore.insert_thread() → SQLite
    │ 3. threads.insert(id, Thread::new())
    │ 4. active_thread_id = Some(id)
    │ 5. broadcast ThreadEvent::Created
    ▼
返回 ThreadInfo { id, title, state: Idle, ... }
```

### 2. 发送消息（活跃 Thread）

```
Frontend: session_manager.send_message(content)
    │
    ▼
SessionManager:
    │ 1. 获取 active_thread
    │ 2. thread.start_turn(content)
    │     ├── 创建 TurnDriver
    │     ├── state = Processing
    │     ├── broadcast TurnStarted
    │     └── TurnDriver::spawn() 执行
    │ 3. (Turn 完成回调)
    │     ├── TurnRecord.state = Completed
    │     ├── ThreadStore.save_turn() → SQLite
    │     ├── history.push(turn_record)
    │     └── broadcast TurnCompleted
```

### 3. 切换 Thread（当前有活跃任务）

```
Frontend: session_manager.switch_thread(target_id)
    │
    ▼
SessionManager:
    │ 1. 当前 active_thread.state == Processing?
    │    Yes →
    │       - active_thread.state = BackgroundProcessing
    │       - 保持 TurnDriver 运行
    │       - broadcast StateChanged
    │ 2. active_thread_id = target_id
    │ 3. target_thread.state = Idle/Processing (恢复状态)
    │ 4. broadcast ThreadEvent::Activated
    ▼
返回成功
```

### 4. 后台任务完成通知

```
Thread (BackgroundProcessing) TurnDriver 完成:
    │
    ▼
Thread:
    │ 1. state = Idle
    │ 2. TurnRecord.state = Completed
    │ 3. ThreadStore.save_turn() → SQLite
    │ 4. broadcast ThreadEvent::StateChanged
    ▼
Frontend 收到通知，显示 "Thread XXX 已完成"
```

## SQLite Schema

```sql
-- Thread 表
CREATE TABLE threads (
    id TEXT PRIMARY KEY,              -- Uuid as string
    session_id TEXT NOT NULL,
    title TEXT,
    state TEXT NOT NULL,              -- Idle/Processing/BackgroundProcessing/...
    created_at TEXT NOT NULL,         -- ISO 8601
    updated_at TEXT NOT NULL,
    metadata TEXT                     -- JSON for extensibility
);

CREATE INDEX idx_threads_session ON threads(session_id);
CREATE INDEX idx_threads_updated ON threads(updated_at DESC);

-- Turn 记录表
CREATE TABLE turns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    thread_id TEXT NOT NULL,
    turn_number INTEGER NOT NULL,
    user_input TEXT NOT NULL,
    assistant_response TEXT,
    state TEXT NOT NULL,              -- Completed/Failed/Interrupted
    started_at TEXT NOT NULL,
    completed_at TEXT,
    FOREIGN KEY (thread_id) REFERENCES threads(id) ON DELETE CASCADE
);

CREATE INDEX idx_turns_thread ON turns(thread_id);
CREATE INDEX idx_turns_number ON turns(thread_id, turn_number);

-- ToolCall 记录表
CREATE TABLE tool_calls (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    turn_id INTEGER NOT NULL,
    call_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    arguments TEXT NOT NULL,
    result TEXT,
    is_error INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (turn_id) REFERENCES turns(id) ON DELETE CASCADE
);

CREATE INDEX idx_tool_calls_turn ON tool_calls(turn_id);
```

## 错误处理

```rust
pub enum ThreadError {
    StoreError { source: sqlx::Error },
    InvalidStateTransition {
        thread_id: Uuid,
        from: ThreadState,
        to: ThreadState
    },
    TurnError { source: TurnError },
    ThreadNotFound { thread_id: Uuid },
}
```

### 关键错误恢复策略

| 场景 | 处理方式 |
|------|----------|
| SQLite 写入失败 | 内存状态保留，重试 3 次后标记 thread 为 degraded |
| 后台 Turn 失败 | 记录 TurnRecord.state=Failed，通知前端显示错误 |
| 切换时 TurnDriver panic | 捕获，标记 Thread 为 Failed，不影响其他 Thread |
| 加载历史失败 | 返回空历史，记录日志，不阻塞 Thread 创建 |

## Tauri 启动初始化流程

```
Tauri::run()
    │
    ▼
setup()
    ├─ init_logging()
    ├─ init_telemetry()
    ├─ init_database()           -- SQLite 连接池
    ├─ init_thread_store()       -- ThreadStore::new(db_pool)
    └─ init_session_manager()    -- SessionManager::new(thread_store, telemetry)
         │
         ▼
    AppHandle 管理状态，注入到 Tauri commands
```

## 组件清单

| 组件 | 职责 | 文件位置 |
|------|------|----------|
| `SessionManager` | 顶层入口，管理 Thread 集合，事件广播 | `session/manager.rs` |
| `Thread` | 单个对话的运行时状态，封装 TurnDriver | `session/thread.rs` |
| `ThreadState` | Thread 状态机 | `session/thread.rs` |
| `ThreadStore` | SQLite 持久化层 | `session/store.rs` |
| `TurnRecord` | 持久化的 Turn 数据 | `session/types.rs` |
| `ThreadEvent` | 前端订阅的事件类型 | `session/types.rs` |

## 改动范围

| 模块 | 改动类型 | 说明 |
|------|----------|------|
| `turn/` | **不改动** | TurnDriver/TurnTranscript 保持现有实现 |
| `session/` | **新增** | SessionManager + Thread + ThreadStore |
| `desktop/` | **修改** | Tauri commands 接入 SessionManager，启动初始化 |
| `sql/` | **新增** | SQLite schema + migrations |

## 实现阶段

1. **Phase 1** - ThreadStore + 基础 Thread（无后台执行）
2. **Phase 2** - SessionManager + Tauri 启动集成
3. **Phase 3** - 前端接入 + 后台执行 + 事件通知
4. **Phase 4** - 测试覆盖 + 边缘场景

## 参考

- IronClaw 实现位于 `.vendor/ironclaw/src/agent/`
- 核心参考文件: `session.rs`, `session_manager.rs`, `thread_ops.rs`
