# Chat 页面前后端连接实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 实现前端 PromptComposer 与后端 Turn 运行时的连接，支持单轮对话能力

**Architecture:** 使用 Tauri IPC 通信，后端启动 TurnDriver 处理对话，通过 Tauri 事件将流式输出推送至前端

**Tech Stack:** Rust (Tauri v2, turn crate), TypeScript (Next.js 16), SQLite

---

## Task 1: 配置加载模块

**Files:**
- Create: `desktop/src-tauri/src/config.rs`
- Modify: `desktop/src-tauri/Cargo.toml`
- Test: `desktop/src-tauri/tests/config_test.rs`

### Step 1: 添加 config 依赖

**Modify:** `desktop/src-tauri/Cargo.toml`

```toml
[dependencies]
directories = "5"
```

### Step 2: 编写配置结构

**Create:** `desktop/src-tauri/src/config.rs`

```rust
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub agent: AgentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub path: PathBuf,
}

impl Default for StorageConfig {
    fn default() -> Self {
        let base = ProjectDirs::from("com", "argusx", "argusx")
            .map(|p| p.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("~/.argusx"));
        Self {
            path: base.join("sqlite"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: String,
    pub model: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "gpt-4o-mini".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub default: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            default: "sre-agent".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = Self::config_path();

        if config_path.exists() {
            let content = fs::read_to_string(&config_path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_default()
        } else {
            let default_config = Self::default();
            if let Some(parent) = config_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&config_path, toml::to_string_pretty(&default_config).unwrap());
            default_config
        }
    }

    fn config_path() -> PathBuf {
        ProjectDirs::from("com", "argusx", "argusx")
            .map(|p| p.config_dir().join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("~/.argusx/config.toml"))
    }
}
```

### Step 3: 编写测试

**Create:** `desktop/src-tauri/tests/config_test.rs`

```rust
use argusx_desktop::config::Config;

#[test]
fn test_config_default() {
    let config = Config::default();
    assert_eq!(config.llm.provider, "openai");
    assert_eq!(config.llm.model, "gpt-4o-mini");
    assert_eq!(config.agent.default, "sre-agent");
}

#[test]
fn test_config_load() {
    let config = Config::load();
    assert_eq!(config.agent.default, "sre-agent");
}
```

### Step 4: 运行测试验证

**Run:** `cd desktop/src-tauri && cargo test config_test -- --nocapture`

Expected: PASS

### Step 5: Commit

```bash
git add desktop/src-tauri/src/config.rs desktop/src-tauri/Cargo.toml desktop/src-tauri/tests/config_test.rs
git commit -m "feat: add config loading module for ~/.argusx/config.toml"
```

---

## Task 2: 数据库初始化模块

**Files:**
- Create: `desktop/src-tauri/src/db.rs`
- Test: `desktop/src-tauri/tests/db_test.rs`

### Step 1: 编写数据库模块

**Create:** `desktop/src-tauri/src/db.rs`

```rust
use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &PathBuf) -> Result<Self> {
        let db_path = path.join("argusx.db");
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(&db_path)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS agents (
                id TEXT PRIMARY KEY,
                label TEXT NOT NULL,
                description TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS turns (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                prompt TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                finished_at INTEGER,
                FOREIGN KEY (agent_id) REFERENCES agents(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                turn_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                tool_name TEXT,
                tool_call_id TEXT,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (turn_id) REFERENCES turns(id)
            )",
            [],
        )?;

        // 插入默认 agent
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT OR IGNORE INTO agents (id, label, description, created_at, updated_at)
             VALUES ('sre-agent', 'sre-agent', 'General purpose AI assistant', ?1, ?2)",
            [now, now],
        )?;

        Ok(())
    }
}
```

### Step 2: 编写测试

**Create:** `desktop/src-tauri/tests/db_test.rs`

```rust
use argusx_desktop::db::Database;
use std::path::PathBuf;

#[test]
fn test_database_init() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().to_path_buf();

    let db = Database::new(&path).unwrap();

    // 验证默认 agent 已创建
    let conn = db.conn.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, label, description FROM agents WHERE id = 'sre-agent'")
        .unwrap();
    let agent = stmt
        .query_row([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .unwrap();

    assert_eq!(agent.0, "sre-agent");
}
```

### Step 3: 运行测试

**Run:** `cd desktop/src-tauri && cargo test db_test -- --nocapture`

Expected: PASS

### Step 4: Commit

```bash
git add desktop/src-tauri/src/db.rs desktop/src-tauri/tests/db_test.rs
git commit -m "feat: add database initialization module"
```

---

## Task 3: Turn Observer 实现

**Files:**
- Create: `desktop/src-tauri/src/observer.rs`

### Step 1: 实现 TauriTurnObserver

**Create:** `desktop/src-tauri/src/observer.rs`

```rust
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use turn::TurnEvent;

#[derive(Clone, Serialize)]
pub struct TurnEventPayload {
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: serde_json::Value,
}

pub struct TauriTurnObserver {
    app: AppHandle,
}

impl TauriTurnObserver {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl turn::TurnObserver for TauriTurnObserver {
    async fn on_event(&self, event: &TurnEvent) -> Result<(), turn::TurnError> {
        let (event_type, data) = match event {
            TurnEvent::TurnStarted { .. } => ("turn-started", serde_json::json!({})),
            TurnEvent::LlmTextDelta { text } => {
                ("llm-text-delta", serde_json::json!({ "text": text.as_ref() }))
            }
            TurnEvent::LlmReasoningDelta { text } => {
                ("llm-reasoning-delta", serde_json::json!({ "text": text.as_ref() }))
            }
            TurnEvent::ToolCallPrepared { call } => {
                ("tool-call-prepared", serde_json::json!({ "call": call.as_ref() }))
            }
            TurnEvent::ToolCallCompleted { call_id, result } => {
                ("tool-call-completed", serde_json::json!({ "call_id": call_id.as_ref(), "result": result }))
            }
            TurnEvent::TurnFinished { reason } => {
                ("turn-finished", serde_json::json!({ "reason": reason }))
            }
            _ => return Ok(()),
        };

        let payload = TurnEventPayload {
            event_type: event_type.to_string(),
            data,
        };

        self.app
            .emit("turn-event", payload)
            .map_err(|e| turn::TurnError::Runtime(e.to_string()))?;

        Ok(())
    }
}
```

### Step 2: Commit

```bash
git add desktop/src-tauri/src/observer.rs
git commit -m "feat: implement TauriTurnObserver for event emission"
```

---

## Task 4: Turn 管理器

**Files:**
- Create: `desktop/src-tauri/src/turn_manager.rs`

### Step 1: 实现 Turn 管理器

**Create:** `desktop/src-tauri/src/turn_manager.rs`

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use turn::{TurnContext, TurnDriver, TurnHandle};

pub struct TurnManager {
    handles: Arc<Mutex<HashMap<String, TurnHandle>>>,
}

impl TurnManager {
    pub fn new() -> Self {
        Self {
            handles: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start_turn(
        &self,
        turn_id: String,
        session_id: String,
        prompt: String,
        model: Arc<dyn turn::ModelRunner>,
        tool_runner: Arc<dyn turn::ToolRunner>,
        authorizer: Arc<dyn turn::ToolAuthorizer>,
        observer: Arc<dyn turn::TurnObserver>,
    ) -> Result<TurnHandle, turn::TurnError> {
        let context = TurnContext::new(session_id, turn_id.clone(), prompt);

        let (handle, _task) = TurnDriver::spawn(
            context,
            model,
            tool_runner,
            authorizer,
            observer,
        );

        let mut handles = self.handles.lock().await;
        handles.insert(turn_id, handle.clone());

        Ok(handle)
    }

    pub async fn get_handle(&self, turn_id: &str) -> Option<TurnHandle> {
        let handles = self.handles.lock().await;
        handles.get(turn_id).cloned()
    }

    pub async fn remove(&self, turn_id: &str) {
        let mut handles = self.handles.lock().await;
        handles.remove(turn_id);
    }
}

impl Default for TurnManager {
    fn default() -> Self {
        Self::new()
    }
}
```

### Step 2: Commit

```bash
git add desktop/src-tauri/src/turn_manager.rs
git commit -m "feat: add TurnManager for managing turn lifecycle"
```

---

## Task 5: Tauri Commands

**Files:**
- Modify: `desktop/src-tauri/src/lib.rs`

### Step 1: 添加命令实现

**Modify:** `desktop/src-tauri/src/lib.rs`

```rust
mod config;
mod db;
mod observer;
mod turn_manager;

use config::Config;
use db::Database;
use observer::TauriTurnObserver;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use turn_manager::TurnManager;

pub struct AppState {
    pub config: Config,
    pub db: Database,
    pub turn_manager: TurnManager,
}

#[tauri::command]
async fn start_turn(
    app: AppHandle,
    state: State<'_, AppState>,
    prompt: String,
    agent_id: String,
) -> Result<String, String> {
    let turn_id = uuid::Uuid::new_v4().to_string();
    let session_id = "default".to_string();

    // 创建 model runner (暂时 mock)
    let model = Arc::new(MockModelRunner) as Arc<dyn turn::ModelRunner>;

    // 创建 tool runner (暂时 mock)
    let tool_runner = Arc::new(MockToolRunner) as Arc<dyn turn::ToolRunner>;

    // 允许所有工具调用
    let authorizer = Arc::new(MockToolAuthorizer) as Arc<dyn turn::ToolAuthorizer>;

    // 创建 observer
    let observer = Arc::new(TauriTurnObserver::new(app.clone())) as Arc<dyn turn::TurnObserver>;

    // 启动 turn
    let handle = state
        .turn_manager
        .start_turn(
            turn_id.clone(),
            session_id,
            prompt,
            model,
            tool_runner,
            authorizer,
            observer,
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(turn_id)
}

#[tauri::command]
async fn cancel_turn(
    state: State<'_, AppState>,
    turn_id: String,
) -> Result<(), String> {
    if let Some(handle) = state.turn_manager.get_handle(&turn_id).await {
        handle.cancel().await.map_err(|e| e.to_string())?;
        state.turn_manager.remove(&turn_id).await;
    }
    Ok(())
}

// Mock implementations (后续实现真实版本)
struct MockModelRunner;

#[async_trait::async_trait]
impl turn::ModelRunner for MockModelRunner {
    async fn start(
        &self,
        _request: turn::LlmStepRequest,
    ) -> Result<argus_core::ResponseStream, turn::TurnError> {
        // TODO: 实现真实 model runner
        Err(turn::TurnError::Runtime("not implemented".to_string()))
    }
}

struct MockToolRunner;

#[async_trait::async_trait]
impl turn::ToolRunner for MockToolRunner {
    async fn execute(
        &self,
        _call: argus_core::ToolCall,
        _context: tool::ToolContext,
    ) -> Result<tool::ToolResult, tool::ToolError> {
        Ok(tool::ToolResult::output("mock result"))
    }
}

struct MockToolAuthorizer;

#[async_trait::async_trait]
impl turn::ToolAuthorizer for MockToolAuthorizer {
    async fn authorize(
        &self,
        _call: &argus_core::ToolCall,
    ) -> Result<turn::AuthorizationDecision, turn::TurnError> {
        Ok(turn::AuthorizationDecision::Allow)
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load();
    let db = Database::new(&config.storage.path).map_err(|e| e.to_string())?;

    let run_result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            config,
            db,
            turn_manager: TurnManager::new(),
        })
        .invoke_handler(tauri::generate_handler![start_turn, cancel_turn])
        .on_window_event(|_app, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                tracing::info!(event_name = "window_closed");
            }
        })
        .run(tauri::generate_context!());

    run_result?;
    Ok(())
}
```

### Step 2: 添加依赖

**Modify:** `desktop/src-tauri/Cargo.toml`

```toml
[dependencies]
turn = { path = "../../turn" }
core = { path = "../../core" }
tool = { path = "../../tool" }
```

### Step 3: 编译验证

**Run:** `cd desktop/src-tauri && cargo check`

Expected: 编译成功（可能有 mock 的警告）

### Step 4: Commit

```bash
git add desktop/src-tauri/src/lib.rs desktop/src-tauri/Cargo.toml
git commit -m "feat: add start_turn and cancel_turn tauri commands"
```

---

## Task 6: 前端 Hook

**Files:**
- Create: `desktop/lib/chat.ts`
- Create: `desktop/lib/types/agent.ts`

### Step 1: 定义 Agent 类型

**Create:** `desktop/lib/types/agent.ts`

```typescript
export interface Agent {
  id: string;
  label: string;
  description: string;
}

export const AGENTS: Agent[] = [
  {
    id: "sre-agent",
    label: "sre-agent",
    description: "General purpose AI assistant",
  },
];
```

### Step 2: 实现 useTurn hook

**Create:** `desktop/lib/chat.ts`

```typescript
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

export type TurnEventType =
  | "turn-started"
  | "llm-text-delta"
  | "llm-reasoning-delta"
  | "tool-call-prepared"
  | "tool-call-completed"
  | "turn-finished";

export interface TurnEvent {
  type: TurnEventType;
  data: Record<string, unknown>;
}

export interface UseTurnReturn {
  startTurn: (prompt: string, agentId: string) => Promise<string>;
  cancelTurn: (turnId: string) => Promise<void>;
  subscribe: (
    callback: (event: TurnEvent) => void
  ) => Promise<UnlistenFn>;
}

export function useTurn(): UseTurnReturn {
  const startTurn = async (prompt: string, agentId: string): Promise<string> => {
    const turnId = await invoke<string>("start_turn", { prompt, agentId });
    return turnId;
  };

  const cancelTurn = async (turnId: string): Promise<void> => {
    await invoke("cancel_turn", { turnId });
  };

  const subscribe = async (
    callback: (event: TurnEvent) => void
  ): Promise<UnlistenFn> => {
    const unlisten = await listen<TurnEvent>("turn-event", (event) => {
      callback(event.payload);
    });
    return unlisten;
  };

  return { startTurn, cancelTurn, subscribe };
}
```

### Step 3: Commit

```bash
git add desktop/lib/chat.ts desktop/lib/types/agent.ts
git commit -m "feat: add useTurn hook for frontend turn management"
```

---

## Task 7: Chat 页面集成

**Files:**
- Modify: `desktop/app/chat/page.tsx`

### Step 1: 更新 Chat 页面

**Modify:** `desktop/app/chat/page.tsx`

```typescript
"use client";

import { useState, useEffect, useCallback } from "react";
import { PromptComposer } from "@/components/ai";
import { useTurn, TurnEvent } from "@/lib/chat";
import { AGENTS } from "@/lib/types/agent";

interface Message {
  id: string;
  role: "user" | "assistant" | "tool";
  content: string;
  toolName?: string;
}

export default function ChatPage() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [currentAssistantMessage, setCurrentAssistantMessage] = useState("");
  const { startTurn, subscribe } = useTurn();

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    subscribe((event: TurnEvent) => {
      switch (event.type) {
        case "turn-started":
          setCurrentAssistantMessage("");
          break;
        case "llm-text-delta":
          setCurrentAssistantMessage((prev) => prev + (event.data.text as string));
          break;
        case "turn-finished":
          if (currentAssistantMessage) {
            setMessages((prev) => [
              ...prev,
              {
                id: crypto.randomUUID(),
                role: "assistant",
                content: currentAssistantMessage,
              },
            ]);
          }
          setCurrentAssistantMessage("");
          break;
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, [subscribe, currentAssistantMessage]);

  const handleSubmit = useCallback(
    async (payload: { draft: string; selectionId: string }) => {
      const userMessage: Message = {
        id: crypto.randomUUID(),
        role: "user",
        content: payload.draft,
      };
      setMessages((prev) => [...prev, userMessage]);

      await startTurn(payload.draft, payload.selectionId);
    },
    [startTurn]
  );

  return (
    <div className="flex min-h-0 flex-1 flex-col p-4 lg:p-6">
      <div className="flex-1 overflow-y-auto space-y-4">
        {messages.map((message) => (
          <div
            key={message.id}
            className={`p-4 rounded-lg ${
              message.role === "user"
                ? "bg-primary/10 ml-auto max-w-[80%]"
                : "bg-muted mr-auto max-w-[80%]"
            }`}
          >
            <p className="whitespace-pre-wrap">{message.content}</p>
          </div>
        ))}
        {currentAssistantMessage && (
          <div className="bg-muted mr-auto max-w-[80%] p-4 rounded-lg">
            <p className="whitespace-pre-wrap">{currentAssistantMessage}</p>
            <span className="animate-pulse">...</span>
          </div>
        )}
      </div>
      <div className="mx-auto w-full max-w-5xl mt-4">
        <PromptComposer
          agents={[...AGENTS]}
          onSubmit={handleSubmit}
          workflows={[]}
        />
      </div>
    </div>
  );
}
```

### Step 2: Commit

```bash
git add desktop/app/chat/page.tsx
git commit -m "feat: integrate PromptComposer with useTurn hook"
```

---

## Task 8: Mock ModelRunner 实现（可选，用于测试）

**Files:**
- Modify: `desktop/src-tauri/src/lib.rs`

### Step 1: 实现返回固定文本的 ModelRunner

```rust
struct MockModelRunner;

#[async_trait::async_trait]
impl turn::ModelRunner for MockModelRunner {
    async fn start(
        &self,
        _request: turn::LlmStepRequest,
    ) -> Result<argus_core::ResponseStream, turn::TurnError> {
        use tokio::sync::mpsc;

        let (tx, rx) = mpsc::channel(32);
        let tx_clone = tx.clone();

        tokio::spawn(async move {
            // 模拟延迟
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // 发送文本
            let _ = tx_clone
                .send(argus_core::ResponseEvent::ContentDelta("Hello, I am sre-agent. How can I help you today?".into()))
                .await;

            // 发送完成
            let _ = tx_clone
                .send(argus_core::ResponseEvent::Done {
                    reason: argus_core::FinishReason::Stop,
                    usage: None,
                })
                .await;
        });

        Ok(argus_core::ResponseStream::from_parts(
            rx,
            tokio::task::spawn(async {}).abort_handle(),
        ))
    }
}
```

### Step 2: Commit

```bash
git add desktop/src-tauri/src/lib.rs
git commit -m "feat: implement mock model runner for testing"
```

---

## 总结

完成所有任务后，你将拥有：

1. ✅ 配置文件加载 (`~/.argusx/config.toml`)
2. ✅ SQLite 数据库初始化
3. ✅ Turn 事件观察者 (Tauri 事件发射)
4. ✅ Turn 管理器
5. ✅ Tauri 命令 (`start_turn`, `cancel_turn`)
6. ✅ 前端 `useTurn` hook
7. ✅ Chat 页面集成
8. ✅ 可测试的 Mock ModelRunner
