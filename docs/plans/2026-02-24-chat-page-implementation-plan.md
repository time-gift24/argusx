# Chat 页面实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**目标:** 在 argusx-desktop 中实现完整的 Chat 页面，集成 Agent 能力，展示类似 Claude Code 的 Agent 运行状态。

**架构:** 采用 Tauri IPC 通信，前端使用 Zustand 管理状态，后端通过 Rust agent crate 提供对话能力。

**技术栈:**
- 前端: Next.js 16 + React 19 + Tailwind CSS v4 + shadcn/ui + Zustand
- 后端: Tauri v2 + Rust agent crate
- 通信: Tauri IPC (invoke + Event)

---

## 阶段一: 基础框架搭建

### Task 1: 创建 Chat 页面路由和基础组件

**文件:**
- 创建: `argusx-desktop/app/chat/page.tsx`
- 创建: `argusx-desktop/components/features/chat/chat-input.tsx`
- 创建: `argusx-desktop/components/features/chat/message-list.tsx`
- 创建: `argusx-desktop/components/features/chat/session-switcher.tsx`
- 创建: `argusx-desktop/components/features/chat/status-bar.tsx`

**步骤 1: 创建 Chat 页面入口**

```typescript
// argusx-desktop/app/chat/page.tsx
import { ChatPage } from "@/components/features/chat/chat-page";

export default function Page() {
  return <ChatPage />;
}
```

**步骤 2: 创建 ChatPage 容器组件（Mock 数据）**

```typescript
// argusx-desktop/components/features/chat/chat-page.tsx
"use client";

import { MessageList } from "./message-list";
import { ChatInput } from "./chat-input";
import { SessionSwitcher } from "./session-switcher";
import { StatusBar } from "./status-bar";

export function ChatPage() {
  return (
    <div className="flex flex-col h-screen">
      <MessageList />
      <StatusBar />
      <SessionSwitcher />
      <ChatInput />
    </div>
  );
}
```

**步骤 3: 创建 MessageList 组件（Mock 数据）**

```typescript
// argusx-desktop/components/features/chat/message-list.tsx
"use client";

const MOCK_MESSAGES = [
  { id: "1", role: "user", content: "Hello，帮我写个排序算法" },
  { id: "2", role: "assistant", content: "好的，我来帮你实现一个快速排序算法" },
];

export function MessageList() {
  return (
    <div className="flex-1 overflow-auto p-4">
      {MOCK_MESSAGES.map((msg) => (
        <div key={msg.id} className={msg.role === "user" ? "text-right" : "text-left"}>
          <span className="font-bold">{msg.role === "user" ? "User" : "Agent"}</span>
          <p className="mt-1">{msg.content}</p>
        </div>
      ))}
    </div>
  );
}
```

**步骤 4: 创建 StatusBar 组件**

```typescript
// argusx-desktop/components/features/chat/status-bar.tsx
"use client";

export function StatusBar() {
  return (
    <div className="border-t px-4 py-1 text-sm text-muted-foreground">
      Status: idle (session: default)
    </div>
  );
}
```

**步骤 5: 创建 SessionSwitcher 组件**

```typescript
// argusx-desktop/components/features/chat/session-switcher.tsx
"use client";

import { Badge } from "@/components/ui/badge";

const MOCK_SESSIONS = [
  { id: "1", title: "New Chat", color: "blue" },
];

export function SessionSwitcher() {
  return (
    <div className="flex items-center gap-2 px-4 py-2 border-t">
      {MOCK_SESSIONS.map((s) => (
        <Badge key={s.id} variant="outline" className={`border-${s.color}-500`}>
          {s.title}
        </Badge>
      ))}
    </div>
  );
}
```

**步骤 6: 创建 ChatInput 组件**

```typescript
// argusx-desktop/components/features/chat/chat-input.tsx
"use client";

import { useState } from "react";
import { Textarea } from "@/components/ui/textarea";
import { Button } from "@/components/ui/button";

export function ChatInput() {
  const [input, setInput] = useState("");

  return (
    <div className="border-t p-4 flex gap-2">
      <Textarea
        value={input}
        onChange={(e) => setInput(e.target.value)}
        placeholder="输入消息..."
        className="flex-1"
      />
      <Button>发送</Button>
    </div>
  );
}
```

**步骤 7: 运行验证**

Run: `pnpm dev`
Expected: 访问 http://localhost:3000/chat 能看到页面渲染

---

### Task 2: 创建 Zustand Chat Store

**文件:**
- 创建: `argusx-desktop/lib/stores/chat-store.ts`

**步骤 1: 定义类型**

```typescript
// argusx-desktop/lib/stores/chat-store.ts
export interface Message {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  timestamp: number;
}

export interface Session {
  id: string;
  title: string;
  color: string;
  createdAt: number;
  updatedAt: number;
  status: "active" | "idle" | "archived";
}

export interface ToolCall {
  callId: string;
  toolName: string;
  status: "running" | "done" | "error";
  output?: string;
}

export type AgentStatus = "idle" | "running" | "error";
```

**步骤 2: 创建 Store**

```typescript
import { create } from "zustand";
import type { Message, Session, ToolCall, AgentStatus } from "./types";

interface ChatStore {
  // State
  sessions: Session[];
  currentSessionId: string | null;
  messages: Message[];
  agentStatus: AgentStatus;
  reasoningText: string;
  toolProgress: ToolCall[];
  error: string | null;

  // Actions
  setSessions: (sessions: Session[]) => void;
  setCurrentSession: (id: string) => void;
  addMessage: (message: Message) => void;
  updateAssistantMessage: (content: string) => void;
  setAgentStatus: (status: AgentStatus) => void;
  setReasoningText: (text: string) => void;
  addToolCall: (toolCall: ToolCall) => void;
  updateToolCall: (callId: string, updates: Partial<ToolCall>) => void;
  setError: (error: string | null) => void;
  reset: () => void;
}

const initialState = {
  sessions: [],
  currentSessionId: null,
  messages: [],
  agentStatus: "idle" as AgentStatus,
  reasoningText: "",
  toolProgress: [],
  error: null,
};

export const useChatStore = create<ChatStore>((set) => ({
  ...initialState,

  setSessions: (sessions) => set({ sessions }),
  setCurrentSession: (id) => set({ currentSessionId: id }),
  addMessage: (message) =>
    set((state) => ({ messages: [...state.messages, message] })),
  updateAssistantMessage: (content) =>
    set((state) => {
      const msgs = [...state.messages];
      const last = msgs[msgs.length - 1];
      if (last && last.role === "assistant") {
        msgs[msgs.length - 1] = { ...last, content };
      }
      return { messages: msgs };
    }),
  setAgentStatus: (status) => set({ agentStatus: status }),
  setReasoningText: (text) => set({ reasoningText: text }),
  addToolCall: (toolCall) =>
    set((state) => ({ toolProgress: [...state.toolProgress, toolCall] })),
  updateToolCall: (callId, updates) =>
    set((state) => ({
      toolProgress: state.toolProgress.map((t) =>
        t.callId === callId ? { ...t, ...updates } : t
      ),
    })),
  setError: (error) => set({ error }),
  reset: () => set(initialState),
}));
```

**步骤 3: 验证 Store 可以导入**

Run: `pnpm lint`
Expected: 无错误

---

## 阶段二: Tauri 后端集成

### Task 3: 添加 Chat Tauri 命令（后端）

**文件:**
- 修改: `argusx-desktop/src-tauri/src/lib.rs`
- 修改: `argusx-desktop/src-tauri/src/main.rs` (如需要)

**步骤 1: 添加 Session 数据结构**

在 lib.rs 中添加:

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub color: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub status: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: i64,
}
```

**步骤 2: 添加 Tauri 命令**

```rust
#[tauri::command]
async fn create_chat_session(title: Option<String>) -> Result<ChatSession, String> {
    let session = ChatSession {
        id: format!("s-{}", uuid::Uuid::new_v4()),
        title: title.unwrap_or_else(|| "New Chat".to_string()),
        color: "blue".to_string(),
        created_at: chrono::Utc::now().timestamp_millis(),
        updated_at: chrono::Utc::now().timestamp_millis(),
        status: "active".to_string(),
    };
    Ok(session)
}

#[tauri::command]
async fn list_chat_sessions() -> Result<Vec<ChatSession>, String> {
    // TODO: 后续从数据库加载
    Ok(vec![])
}

#[tauri::command]
async fn delete_chat_session(id: String) -> Result<(), String> {
    // TODO: 后续实现
    Ok(())
}
```

**步骤 3: 注册命令**

在 generate_handler! 中添加:
```rust
create_chat_session,
list_chat_sessions,
delete_chat_session,
```

**步骤 4: 编译验证**

Run: `cd argusx-desktop && cargo build --manifest-path src-tauri/Cargo.toml`
Expected: 编译成功

---

### Task 4: 创建 Chat API 客户端（前端）

**文件:**
- 创建: `argusx-desktop/lib/api/chat.ts`

**步骤 1: 创建 API 客户端**

```typescript
// argusx-desktop/lib/api/chat.ts
import { invoke } from "@tauri-apps/api/core";

export interface ChatSession {
  id: string;
  title: string;
  color: string;
  created_at: number;
  updated_at: number;
  status: "active" | "idle" | "archived";
}

export interface ChatMessage {
  id: string;
  session_id: string;
  role: "user" | "assistant" | "system";
  content: string;
  created_at: number;
}

export async function createChatSession(title?: string): Promise<ChatSession> {
  return invoke("create_chat_session", { title });
}

export async function listChatSessions(): Promise<ChatSession[]> {
  return invoke("list_chat_sessions");
}

export async function deleteChatSession(id: string): Promise<void> {
  return invoke("delete_chat_session", { id });
}

export async function getChatMessages(sessionId: string): Promise<ChatMessage[]> {
  return invoke("get_chat_messages", { sessionId });
}
```

**步骤 2: 验证类型检查**

Run: `pnpm tsc --noEmit`
Expected: 无类型错误

---

## 阶段三: 完整功能集成

### Task 5: 连接 Store 与 API

**文件:**
- 修改: `argusx-desktop/lib/stores/chat-store.ts`

**步骤 1: 添加 API 调用到 Store**

```typescript
import { create } from "zustand";
import * as chatApi from "@/lib/api/chat";
import type { Message, Session, ToolCall, AgentStatus } from "./types";

interface ChatStore {
  // ... existing state
  loadSessions: () => Promise<void>;
  createSession: (title?: string) => Promise<void>;
  switchSession: (id: string) => Promise<void>;
  deleteSession: (id: string) => Promise<void>;
  sendMessage: (content: string) => Promise<void>;
}

// 在 Store 实现中添加:
loadSessions: async () => {
  const sessions = await chatApi.listChatSessions();
  set({ sessions });
},

createSession: async (title) => {
  const session = await chatApi.createChatSession(title);
  set((state) => ({
    sessions: [...state.sessions, session],
    currentSessionId: session.id,
    messages: [],
  }));
},
```

**步骤 2: 验证**

Run: `pnpm lint`
Expected: 无错误

---

### Task 6: 实现流式响应（Event Listener）

**文件:**
- 修改: `argusx-desktop/components/features/chat/chat-page.tsx`
- 修改: `argusx-desktop/lib/stores/chat-store.ts`

**步骤 1: 添加事件监听**

```typescript
// 在 chat-page.tsx 中添加:
useEffect(() => {
  const unlisten = listen<StreamEvent>("chat-stream-event", (event) => {
    const { event_type, data } = event.payload;
    switch (event_type) {
      case "message_delta":
        store.updateAssistantMessage(data.content);
        break;
      case "reasoning":
        store.setReasoningText(data.content);
        break;
      case "tool_start":
        store.addToolCall({ callId: data.call_id, toolName: data.tool_name, status: "running" });
        break;
      case "tool_end":
        store.updateToolCall(data.call_id, { status: "done", output: data.output });
        break;
      case "turn_done":
        store.setAgentStatus("idle");
        store.setReasoningText("");
        break;
    }
  });

  return () => {
    unlisten.then((fn) => fn());
  };
}, []);
```

**步骤 2: 实现 sendMessage**

```typescript
// 在 store 中:
sendMessage: async (content) => {
  const { currentSessionId, addMessage, setAgentStatus } = get();
  if (!currentSessionId) return;

  // 添加用户消息
  addMessage({
    id: `msg-${Date.now()}`,
    role: "user",
    content,
    timestamp: Date.now(),
  });

  // 添加空的 assistant 消息占位
  addMessage({
    id: `msg-${Date.now()}-assistant`,
    role: "assistant",
    content: "",
    timestamp: Date.now(),
  });

  setAgentStatus("running");

  // TODO: 调用 Tauri 命令触发流式响应
  // await invoke("chat_stream", { sessionId: currentSessionId, message: content });
},
```

---

## 阶段四: UI 完善

### Task 7: 完善消息展示组件

**文件:**
- 修改: `argusx-desktop/components/features/chat/message-list.tsx`

**步骤 1: 添加 Reasoning 和 ToolCall 展示**

```typescript
"use client";

import { useChatStore } from "@/lib/stores/chat-store";
import { cn } from "@/lib/utils";

export function MessageList() {
  const { messages, reasoningText, toolProgress, agentStatus } = useChatStore();

  return (
    <div className="flex-1 overflow-auto p-4 space-y-4">
      {messages.map((msg) => (
        <div key={msg.id} className={cn("max-w-[80%]", msg.role === "user" ? "ml-auto" : "mr-auto")}>
          <span className="text-xs font-bold text-muted-foreground">
            {msg.role === "user" ? "User" : msg.role === "assistant" ? "Agent" : "System"}
          </span>
          <div className={cn("rounded-lg p-3", msg.role === "user" ? "bg-primary text-primary-foreground" : "bg-muted")}>
            {msg.content}
          </div>
        </div>
      ))}

      {/* Reasoning 显示 */}
      {reasoningText && agentStatus === "running" && (
        <div className="mr-auto max-w-[80%]">
          <span className="text-xs font-bold text-muted-foreground">Thinking</span>
          <div className="rounded-lg p-3 bg-muted text-muted-foreground italic">
            {reasoningText}
          </div>
        </div>
      )}

      {/* Tool Progress 显示 */}
      {toolProgress.length > 0 && (
        <div className="mr-auto max-w-[80%]">
          <span className="text-xs font-bold text-muted-foreground">Tools</span>
          <div className="space-y-1">
            {toolProgress.map((tool) => (
              <div key={tool.callId} className="text-sm">
                <span className="font-mono">{tool.toolName}</span>
                <span className={cn("ml-2", tool.status === "running" && "animate-pulse", tool.status === "done" && "text-green-500", tool.status === "error" && "text-red-500")}>
                  {tool.status}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
```

---

### Task 8: 完善 SessionSwitcher 交互

**文件:**
- 修改: `argusx-desktop/components/features/chat/session-switcher.tsx`

**步骤 1: 添加完整交互**

```typescript
"use client";

import { useChatStore } from "@/lib/stores/chat-store";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Plus } from "lucide-react";
import { useState } from "react";

export function SessionSwitcher() {
  const { sessions, currentSessionId, createSession, switchSession, deleteSession } = useChatStore();
  const [openMenu, setOpenMenu] = useState<string | null>(null);

  const handleCreate = async () => {
    await createSession("New Chat");
  };

  const handleSwitch = async (id: string) => {
    await switchSession(id);
  };

  return (
    <div className="flex items-center gap-2 px-4 py-2 border-t">
      {sessions.map((s) => (
        <Badge
          key={s.id}
          variant={s.id === currentSessionId ? "default" : "outline"}
          className="cursor-pointer relative"
          onClick={() => handleSwitch(s.id)}
        >
          <span className={`w-2 h-2 rounded-full bg-${s.color}-500 mr-1`} />
          {s.title}
        </Badge>
      ))}
      <Button variant="ghost" size="sm" onClick={handleCreate}>
        <Plus className="w-4 h-4" />
      </Button>
    </div>
  );
}
```

---

### Task 9: 完善 StatusBar

**文件:**
- 修改: `argusx-desktop/components/features/chat/status-bar.tsx`

**步骤 1: 添加停止按钮**

```typescript
"use client";

import { useChatStore } from "@/lib/stores/chat-store";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { StopCircle, Loader2 } from "lucide-react";

export function StatusBar() {
  const { agentStatus, currentSessionId, setAgentStatus } = useChatStore();

  return (
    <div className="flex items-center justify-between border-t px-4 py-1 text-sm">
      <span className="text-muted-foreground">
        Status:{" "}
        <span
          className={cn(
            agentStatus === "idle" && "text-green-500",
            agentStatus === "running" && "text-blue-500",
            agentStatus === "error" && "text-red-500"
          )}
        >
          {agentStatus}
        </span>
        {currentSessionId && ` (session: ${currentSessionId.slice(0, 8)})`}
      </span>

      {agentStatus === "running" && (
        <Button variant="outline" size="sm" onClick={() => setAgentStatus("idle")}>
          <StopCircle className="w-4 h-4 mr-1" />
          停止
        </Button>
      )}
    </div>
  );
}
```

---

### Task 10: 完善 ChatInput

**文件:**
- 修改: `argusx-desktop/components/features/chat/chat-input.tsx`

**步骤 1: 添加完整交互**

```typescript
"use client";

import { useState, useRef, useEffect } from "react";
import { Textarea } from "@/components/ui/textarea";
import { Button } from "@/components/ui/button";
import { Send } from "lucide-react";
import { useChatStore } from "@/lib/stores/chat-store";

export function ChatInput() {
  const [input, setInput] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const { sendMessage, agentStatus } = useChatStore();

  const handleSend = async () => {
    if (!input.trim() || agentStatus === "running") return;
    const content = input.trim();
    setInput("");
    await sendMessage(content);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  // 自动聚焦
  useEffect(() => {
    textareaRef.current?.focus();
  }, []);

  return (
    <div className="border-t p-4 flex gap-2 items-end">
      <Textarea
        ref={textareaRef}
        value={input}
        onChange={(e) => setInput(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="输入消息... (Shift+Enter 换行)"
        className="flex-1 min-h-[60px] max-h-[200px] resize-none"
        disabled={agentStatus === "running"}
      />
      <Button onClick={handleSend} disabled={!input.trim() || agentStatus === "running"}>
        <Send className="w-4 h-4" />
      </Button>
    </div>
  );
}
```

---

## 阶段五: 测试与集成

### Task 11: 端到端测试

**步骤 1: 测试页面加载**

Run: `pnpm dev`
访问 http://localhost:3000/chat

Expected:
- 页面正常渲染
- 可以看到 MessageList、StatusBar、SessionSwitcher、ChatInput 组件

**步骤 2: 测试创建 Session**

点击 [+] 按钮

Expected:
- 新 Session 添加到列表
- 当前 Session 切换到新 Session

**步骤 3: 测试发送消息**

在输入框输入内容，点击发送

Expected:
- 用户消息显示在列表
- Agent 状态变为 "running"
- (由于后端未实现，暂时显示空回复)

---

## 后续工作（不在本计划范围内）

1. **后端完整实现**: 实现 `agent` crate 的 `chat_stream` 方法
2. **数据库持久化**: Session 和 Message 持久化到 SQLite
3. **工具调用展示**: 完善工具调用的详细信息展示
4. **历史消息加载**: Session 切换时加载历史消息
5. **Session 编辑**: 右键菜单修改 Session 名称/颜色
6. **响应式布局**: 适配移动端

---

## 验收标准

- [ ] 访问 /chat 页面能正常渲染
- [ ] 组件结构完整: MessageList + StatusBar + SessionSwitcher + ChatInput
- [ ] Zustand Store 正常工作
- [ ] Tauri 命令可以调用（即使返回空数据）
- [ ] UI 交互基本可用（创建 Session、发送消息）
- [ ] 代码风格符合项目规范
