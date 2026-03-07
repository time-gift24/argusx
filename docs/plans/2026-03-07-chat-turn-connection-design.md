# Chat 页面前后端连接设计

## 概述

本文档描述如何将前端 PromptComposer 组件与后端 Turn 运行时连接，实现单轮对话能力。

## 架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                      Frontend (Next.js)                     │
│  ┌─────────────────┐    ┌─────────────────────────────┐   │
│  │  PromptComposer │───▶│  useTurn (hook)             │   │
│  │  (输入 + Agent)  │    │  - listen Tauri events      │   │
│  └─────────────────┘    │  - invoke start_turn        │   │
│                          └──────────────┬────────────────┘   │
└─────────────────────────────────────────┼───────────────────┘
                                          │ Tauri IPC
                                          ▼
┌───────────────────────────────────────────────────────────────┐
│                     Backend (Rust Tauri)                      │
│  ┌──────────────────────────────────────────────────────┐    │
│  │  Tauri Commands                                       │    │
│  │  - start_turn(prompt, agent_id) → turn_id            │    │
│  │  - cancel_turn(turn_id)                              │    │
│  └─────────────────────┬────────────────────────────────┘    │
│                        │                                      │
│                        ▼                                      │
│  ┌──────────────────────────────────────────────────────┐    │
│  │  Turn Runtime (from turn crate)                       │    │
│  │  - TurnDriver::spawn()                               │    │
│  │  - emit TurnEvent via Tauri window                   │    │
│  └──────────────────────────────────────────────────────┘    │
│                        │                                      │
│                        ▼                                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐      │
│  │ ModelRunner │  │ ToolRunner  │  │ ToolAuthorizer  │      │
│  │ (固定provider)│  │ (内置工具)  │  │ (允许所有)      │      │
│  └─────────────┘  └─────────────┘  └─────────────────┘      │
└───────────────────────────────────────────────────────────────┘
```

## 配置

### ~/.argusx/config.toml

```toml
# ArgusX Desktop 配置
# 默认位置: ~/.argusx/config.toml

[storage]
# SQLite 数据库路径
path = "~/.argusx/sqlite"

[llm]
# 默认 LLM provider (暂时只支持固定实现)
# 后续会从环境变量或配置读取
provider = "openai"
model = "gpt-4o-mini"

[agent]
# 默认 agent 名称 (无标签)
default = "sre-agent"
```

### 配置加载顺序

1. `~/.argusx/config.toml` (用户配置)
2. 内置默认配置

### 目录结构

```
~/.argusx/
├── config.toml      # 主配置文件
└── sqlite/         # SQLite 数据库目录
    └── argusx.db   # 数据库文件
```

## 前端设计

### Agent 定义

```typescript
// lib/types/agent.ts
export interface Agent {
  id: string;
  label: string;
  description: string;
}

// 预定义 agents
export const AGENTS: Agent[] = [
  {
    id: "sre-agent",
    label: "sre-agent",
    description: "General purpose AI assistant",
  },
];
```

### Turn Hook

```typescript
// hooks/use-turn.ts
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

export interface TurnEvent {
  type: "started" | "text_delta" | "tool_call" | "tool_result" | "finished";
  data: unknown;
}

export function useTurn() {
  const startTurn = async (prompt: string, agentId: string) => {
    const turnId = await invoke<string>("start_turn", { prompt, agentId });
    return turnId;
  };

  const cancelTurn = async (turnId: string) => {
    await invoke("cancel_turn", { turnId });
  };

  // 监听 turn 事件
  const subscribe = (callback: (event: TurnEvent) => void) => {
    return listen<TurnEvent>("turn-event", (event) => {
      callback(event.payload);
    });
  };

  return { startTurn, cancelTurn, subscribe };
}
```

### Chat 页面集成

```typescript
// app/chat/page.tsx
"use client";

import { PromptComposer } from "@/components/ai";
import { useTurn, AGENTS } from "@/lib/chat";

export default function ChatPage() {
  const { startTurn, subscribe } = useTurn();

  // 订阅 turn 事件，更新 UI
  useEffect(() => {
    const unlisten = subscribe((event) => {
      // 处理事件，更新消息列表
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  return (
    <PromptComposer
      agents={AGENTS}
      workflows={[]}
      onSubmit={async ({ draft, selectionId }) => {
        await startTurn(draft, selectionId);
      }}
    />
  );
}
```

## 后端设计

### Tauri Commands

```rust
// src-tauri/src/lib.rs

#[tauri::command]
async fn start_turn(
    app: AppHandle,
    prompt: String,
    agent_id: String,
) -> Result<String, String> {
    let turn_id = uuid::Uuid::new_v4().to_string();
    let session_id = "default".to_string();

    // 创建 turn context
    let context = TurnContext::new(session_id, turn_id.clone(), prompt);

    // 获取/创建 model runner (暂时内置)
    let model = get_model_runner();

    // 创建内置 tool runner
    let tool_runner = create_tool_runner();

    // 允许所有工具调用
    let authorizer = AllowAllToolAuthorizer;

    // 创建 observer，emit Tauri 事件
    let observer = TauriTurnObserver::new(app);

    // 启动 turn
    let (_handle, _task) = TurnDriver::spawn(
        context,
        model,
        Arc::new(tool_runner),
        Arc::new(authorizer),
        Arc::new(observer),
    );

    Ok(turn_id)
}

#[tauri::command]
async fn cancel_turn(turn_id: String) -> Result<(), String> {
    // 从 turn 管理器中获取 handle 并取消
    todo!()
}
```

### TauriTurnObserver

```rust
// src-tauri/src/observer.rs

pub struct TauriTurnObserver {
    app: AppHandle,
}

impl TurnObserver for TauriTurnObserver {
    async fn on_event(&self, event: &TurnEvent) -> Result<(), TurnError> {
        let payload = serde_json::to_string(event).map_err(|e| TurnError::Runtime(e.to_string()))?;
        self.app.emit("turn-event", payload).map_err(|e| TurnError::Runtime(e.to_string()))?;
        Ok(())
    }
}
```

### 内置 ModelRunner

暂时使用固定实现，后续从配置读取：

```rust
// src-tauri/src/model.rs

pub struct SimpleModelRunner {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

#[async_trait]
impl ModelRunner for SimpleModelRunner {
    async fn start(&self, request: LlmStepRequest) -> Result<ResponseStream, TurnError> {
        // 调用 OpenAI API (或其他 provider)
        // 返回 ResponseStream
        todo!()
    }
}
```

### 内置 ToolRunner

支持基础工具：

- `Read` - 读取文件
- `Glob` - 文件搜索
- `Grep` - 内容搜索
- `Shell` - 执行命令

```rust
// src-tauri/src/tools.rs

pub struct BuiltinToolRunner;

#[async_trait]
impl ToolRunner for BuiltinToolRunner {
    async fn execute(
        &self,
        call: ToolCall,
        context: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        match call {
            ToolCall::FunctionCall { name, arguments_json, .. } => {
                // 实现工具
                todo!()
            }
            // ...
        }
    }
}
```

## 数据库设计

### 表: agents

```sql
CREATE TABLE agents (
    id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    description TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- 预置 agent
INSERT INTO agents (id, label, description, created_at, updated_at)
VALUES ('sre-agent', 'sre-agent', 'General purpose AI assistant', unixepoch(), unixepoch());
```

### 表: turns

```sql
CREATE TABLE turns (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    prompt TEXT NOT NULL,
    status TEXT NOT NULL, -- 'running', 'completed', 'cancelled', 'failed'
    created_at INTEGER NOT NULL,
    finished_at INTEGER,
    FOREIGN KEY (agent_id) REFERENCES agents(id)
);
```

### 表: messages

```sql
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    turn_id TEXT NOT NULL,
    role TEXT NOT NULL, -- 'user', 'assistant', 'tool'
    content TEXT NOT NULL,
    tool_name TEXT,
    tool_call_id TEXT,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (turn_id) REFERENCES turns(id)
);
```

## 事件映射

TurnEvent → Tauri Event:

| TurnEvent | Tauri Event | 前端处理 |
|-----------|-------------|---------|
| `TurnStarted` | `turn-started` | 显示"开始"状态 |
| `LlmTextDelta` | `llm-text-delta` | 追加文本到 assistant 消息 |
| `LlmReasoningDelta` | `llm-reasoning-delta` | 显示 reasoning |
| `ToolCallPrepared` | `tool-call-prepared` | 显示 tool call |
| `ToolCallCompleted` | `tool-call-completed` | 显示 tool result |
| `TurnFinished` | `turn-finished` | 显示完成状态 |

## 实现步骤

1. **配置加载**
   - 创建 `~/.argusx/config.toml` 加载模块
   - 实现默认配置

2. **数据库初始化**
   - 创建 SQLite 连接模块
   - 初始化表结构

3. **Turn 运行时集成**
   - 在 Tauri 中引入 turn crate
   - 实现 TauriTurnObserver
   - 实现内置 ModelRunner (先 mock 后实现)

4. **Tauri Commands**
   - 实现 `start_turn` 命令
   - 实现 `cancel_turn` 命令
   - 注册命令到 Tauri

5. **前端 Hook**
   - 实现 `useTurn` hook
   - 订阅 Tauri 事件

6. **Chat 页面**
   - 集成 PromptComposer 和 useTurn
   - 显示消息列表

## 测试计划

### 单元测试

- 配置加载
- 数据库操作

### 集成测试

- 启动 turn → 接收事件 → 完成
- 取消 turn

### E2E 测试

- 前端输入 → 后端处理 → 前端显示结果
