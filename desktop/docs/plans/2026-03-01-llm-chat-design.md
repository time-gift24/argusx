# LLM Chat 页面设计文档

## 概述

使用现有 ai-elements 组件设计一个现代化的 LLM 聊天页面，采用悬浮式无边框设计，支持多会话并发。

## 页面布局

```
┌─────────────────────────────────────────────────────────┐
│                                                          │
│              Conversation Area (全屏滚动)                │
│           消息列表 + 流式打字效果                         │
│                                                          │
│           ↓ 内容滚动时有虚化效果 ↓                        │
│  ┌──────────────────────────────────────────────────┐   │
│  │ [Badge1] [Badge2] [Badge3] →                    │   │  ← 悬浮
│  ├──────────────────────────────────────────────────┤   │
│  │ [PromptInput + 文件附件 + 模型选择]               │   │  ← 唯一边框
│  └──────────────────────────────────────────────────┘   │
│  (backdrop-blur + bg-background/80)                     │
└─────────────────────────────────────────────────────────┘
```

## 核心特性

1. **无边框设计**：整个页面无边框，仅 PromptInput 有边框
2. **悬浮式输入**：Badge 列表 + PromptInput 悬浮在页面底部
3. **虚化背景**：内容滚动穿透时有 backdrop-blur 效果
4. **多会话并发**：无并发限制，用户可同时运行多个会话
5. **流式响应**：打字机效果展示 AI 回复

## 样式规范

### 全局 Token 使用

```css
/* 悬浮区域 */
.floating-area {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  z-index: 50;
  backdrop-filter: blur(12px);
  background: oklch(from var(--background) l c h / 0.8);
}

/* PromptInput（唯一边框） */
.prompt-input-wrapper {
  border: 1px solid var(--border);
  border-radius: var(--radius-xl);
  box-shadow: 0 4px 20px oklch(from var(--foreground) l c h / 0.1);
}
```

### Tailwind 类名

- 悬浮区域：`fixed bottom-0 left-0 right-0 z-50 bg-background/80 backdrop-blur-xl`
- PromptInput 边框：`border border-border rounded-xl shadow-lg`
- Badge 列表：`flex gap-2 overflow-x-auto scrollbar-hide`

## Badge 设计

### 状态类型

| 状态 | 图标 | 说明 |
|------|------|------|
| wait-input | 圆点 | 等待用户输入 |
| thinking | 加载动画 | AI 思考中 |
| tool-call | 工具图标 | 调用工具 |
| outputing | 打字动画 | 正在输出 |

### 交互行为

| 操作 | 行为 |
|------|------|
| 左键点击 | 切换到该会话 |
| 右键点击 | 弹出菜单：改名 / 改颜色 / 删除 |
| 删除 | AlertDialog 二次确认 |
| 激活 | 深色高亮 `bg-primary text-primary-foreground` |

### 颜色预设

使用 chart 系列 token：
- `chart-1` / `chart-2` / `chart-3` / `chart-4` / `chart-5`

## 后端接口设计

### 现有接口

```rust
#[tauri::command]
async fn create_chat_session(title: Option<String>) -> Result<ChatSession, String>;

#[tauri::command]
async fn list_chat_sessions() -> Result<Vec<ChatSession>, String>;

#[tauri::command]
async fn delete_chat_session(id: String) -> Result<(), String>;
```

### 需要新增的接口

```rust
// P0 - 核心功能
#[tauri::command]
async fn send_chat_message(
    session_id: String,
    message: String,
    model: String,
    attachments: Vec<FileAttachment>,
) -> Result<ChatResponse, String>;

#[tauri::command]
async fn get_chat_messages(session_id: String) -> Result<Vec<ChatMessage>, String>;

// P1 - 辅助功能
#[tauri::command]
async fn stop_generation(session_id: String) -> Result<(), String>;

#[tauri::command]
async fn list_available_models() -> Result<Vec<ModelInfo>, String>;

// P2 - 会话管理
#[tauri::command]
async fn update_chat_session(
    id: String,
    title: Option<String>,
    color: Option<String>,
) -> Result<ChatSession, String>;
```

### 流式响应方案

使用 Tauri Event 系统推送 SSE 数据：
- 前端监听 `chat:chunk:{session_id}` 事件
- 后端通过 `app.emit()` 推送流式内容

## 文件结构

### 前端

```
app/chat/
  page.tsx                    # 路由页面

components/features/chat/
  chat-page.tsx               # 主页面组件
  conversation-view.tsx       # 消息列表视图
  session-badge-list.tsx      # Badge 列表
  session-badge.tsx           # 单个 Badge
  badge-context-menu.tsx      # 右键菜单

lib/api/
  chat.ts                     # Tauri IPC 封装

lib/stores/
  chat-store.ts               # 会话状态管理
```

### 后端

```
src-tauri/src/
  lib.rs                      # Tauri 命令注册
  chat/
    mod.rs                    # Chat 模块
    commands.rs               # Chat 相关命令
    types.rs                  # 类型定义
    llm_client.rs             # LLM 客户端封装
```

## 实现阶段

### Phase 1: 前端基础 (当前)

1. 创建路由页面
2. 实现悬浮式布局
3. 实现 Badge 列表组件
4. 使用 mock 数据展示 UI
5. 集成现有 ai-elements 组件

### Phase 2: 后端接口

1. 实现 send_chat_message 命令
2. 集成 llm-client crate
3. 实现 SSE 流式推送
4. 实现会话持久化

### Phase 3: 完善功能

1. 文件附件上传
2. 工具调用支持
3. 模型切换
4. 会话管理 UI

---

Created: 2026-03-01
