# Chat 页面设计方案

> 设计日期: 2026-02-24

## 1. 目标

实现 argusx-desktop 中的 Chat 页面，参考 agent-cli 的 TUI 设计，在 Web 端展示完整的 Agent 运行状态，包括：
- 多会话管理
- 消息展示（用户/助手/思考过程/工具调用）
- 流式响应
- Agent 状态显示

## 2. 布局结构

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│                      Message List                                │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ [User] Hello                                            │   │
│  │                                                           │   │
│  │ [Agent] Thinking... (reasoning)                        │   │
│  │                                                           │   │
│  │ [Tool] read_file: running                               │   │
│  │ [Tool] write_file: done                                 │   │
│  │                                                           │   │
│  │ [Agent] 好的,我已经实现了...                            │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│ Status: running (session: xxx)              [停止]              │
├─────────────────────────────────────────────────────────────────┤
│ [● Session 1] [● Session 2 ▼] [+] │                            │
├─────────────────────────────────────────────────────────────────┤
│  ┌────────────────────────────────────────┐  ┌────────────┐    │
│  │ 输入消息...                              │  │   发送     │    │
│  └────────────────────────────────────────┘  └────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                          ↑ floating fixed 在页面底部
```

### 说明
- 页面背景是 message list
- Chat input 是 floating 组件，固定在页面底部
- 四层结构：Message List → Status Bar → Session Switcher → Input

## 3. 组件设计

| 组件 | 职责 |
|------|------|
| `ChatPage` | 页面容器，管理 session 状态和 Tauri 调用 |
| `MessageList` | 消息列表区域，支持滚动 |
| `MessageBubble` | 单条消息渲染（User/Assistant/System） |
| `ReasoningBlock` | 思考过程展示（可折叠） |
| `ToolProgressBlock` | 工具调用进度展示 |
| `StatusBar` | Agent 运行状态显示 + 停止按钮 |
| `SessionSwitcher` | Session 切换器（Badge + Dropdown + New） |
| `ChatInput` | 输入框 + 发送按钮 |

## 4. 消息类型

```typescript
type MessageRole = 'user' | 'assistant' | 'system';

type MessageType =
  | { type: 'user'; content: string }
  | { type: 'assistant'; content: string }
  | { type: 'reasoning'; content: string }
  | { type: 'tool_call'; tool: string; status: 'running' | 'done' | 'error'; output?: string }
  | { type: 'system'; content: string };

interface Message {
  id: string;
  role: MessageRole;
  content: string;
  timestamp: number;
}
```

## 5. Session 数据模型

```typescript
interface Session {
  id: string;
  title: string;
  color: string;        // 颜色标识
  createdAt: number;
  updatedAt: number;
  status: 'active' | 'idle' | 'archived';
}
```

## 6. 数据流

```
┌─────────────────────────────────────────────────────────────┐
│                    Tauri Backend (Rust)                     │
│  - create_session / list_sessions / delete_session         │
│  - chat_stream (Event stream)                              │
└─────────────────────────┬───────────────────────────────────┘
                          │ invoke() / Event stream
                          ↓
┌─────────────────────────────────────────────────────────────┐
│                    useChatStore (Zustand)                   │
│  - sessions, currentSessionId                              │
│  - messages, agentStatus                                   │
│  - reasoningText, toolProgress                             │
│  - createSession, sendMessage, stopAgent                  │
└─────────────────────────┬───────────────────────────────────┘
                          │ subscribe / render
                          ↓
┌─────────────────────────────────────────────────────────────┐
│                    React Components                          │
└─────────────────────────────────────────────────────────────┘
```

## 7. 状态机

```
┌────────┐  sendMessage()   ┌─────────┐  complete   ┌────────┐
│  Idle  │ ───────────────► │ Running │ ──────────► │  Idle  │
└────────┘                   └─────────┘             └────────┘
    ▲                              │                     │
    │                              │ error               │
    └──────────────────────────────┘                     │
                         ↻ stopAgent()                    │
                                                          │
                         ┌─────────┐ ◄────────────────────┘
                         │  Error  │
                         └─────────┘
```

## 8. 核心 API（前端 Store）

```typescript
interface ChatStore {
  // State
  sessions: Session[];
  currentSessionId: string | null;
  messages: Message[];
  agentStatus: 'idle' | 'running' | 'error';
  reasoningText: string;
  toolProgress: ToolCall[];
  error: string | null;

  // Session Actions
  createSession(title?: string): Promise<void>;
  deleteSession(id: string): Promise<void>;
  switchSession(id: string): Promise<void>;

  // Agent Actions
  sendMessage(content: string): Promise<void>;
  stopAgent(): Promise<void>;

  // Initialization
  loadSessions(): Promise<void>;
}
```

## 9. Tauri 命令接口

假设后端提供（需要实现）：

```rust
// Session 管理
#[tauri::command]
async fn create_session(title: Option<String>) -> Result<Session, String>;

#[tauri::command]
async fn list_sessions() -> Result<Vec<Session>, String>;

#[tauri::command]
async fn delete_session(id: String) -> Result<(), String>;

#[tauri::command]
async fn get_session_messages(id: String) -> Result<Vec<Message>, String>;

// Agent 对话
#[tauri::command]
async fn chat_stream(
    session_id: String,
    message: String,
) -> Result<Receiver<StreamEvent>, String>;
```

## 10. Session Switcher 交互

- **左键点击 Badge**: 切换到该 session
- **左键点击 [...]**: 展开下拉菜单
- **右键点击 Badge**: 上下文菜单（修改名称、修改颜色、删除）
- **[+] 按钮**: 创建新 session

## 11. 颜色池

```typescript
const COLORS = ['blue', 'green', 'purple', 'orange', 'pink', 'cyan'];
```

## 12. 参考

- Vercil AI SDK 设计模式
- agent-cli TUI 实现 (`agent-cli/src/ui.rs`)
- 现有 shadcn/ui 组件库
- Tailwind CSS v4
