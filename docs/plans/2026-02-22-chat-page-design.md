# Chat 页面设计方案

> 设计日期: 2026-02-22

## 1. 布局结构

```
┌────────────────────────────────────────────────────┐
│                                                    │
│              Message List (页面背景)                │
│                                                    │
│                                                    │
├────────────────────────────────────────────────────┤
│ [Session1] [● Session2 ▼] [+] │                  │ ← Row 1: Session badges
├────────────────────────────────────────────────────┤
│           Enter your message...                   │ ← Row 2: Input
├────────────────────────────────────────────────────┤
│                                            [发送] │ ← Row 3: Send button
└────────────────────────────────────────────────────┘
           ↑ floating fixed 在页面底部
```

### 说明
- 页面背景就是 message list
- Chat input 是一个 floating 组件，固定在页面底部
- 三层结构：Session Switcher → Input → Send Button

## 2. 组件设计

| 组件 | 职责 |
|------|------|
| `ChatPage` | 页面容器，加载 session 列表和消息 |
| `ChatInput` | 整体容器，三层结构 |
| `SessionBadge` | 单个 session 标签，显示颜色圆点 + 名称 |
| `SessionDropdown` | [...] 按钮，下拉菜单 |
| `MessageList` | 消息列表区域 |

## 3. Session Badge 交互

- **左键点击 Badge**: 切换到该 session
- **左键点击 [...]**: 展开下拉菜单
- **右键点击 Badge**: 上下文菜单（修改名称、修改颜色、删除）
- **[+] 按钮**: 创建新 session

## 4. Badge 样式

```typescript
// 颜色池（随机分配）
const COLORS = ['blue', 'green', 'purple', 'orange', 'pink', 'cyan'];

// 状态样式
const STATUS_STYLES = {
  active: 'ring-2 ring-primary shadow-md',  // 高亮
  idle: 'opacity-80',
  archived: 'opacity-50',
};
```

## 5. 数据流

```
Tauri Backend (Rust)
       ↓ invoke()
useChatStore (Zustand)
       ↓ render
React Components
```

## 6. 核心 API

```typescript
// 创建 session
createSession(title?: string): Promise<Session>

// 获取 session 列表
listSessions(): Promise<Session[]>

// 发送消息（流式）
sendMessage(sessionId: string, content: string): AsyncGenerator<string>

// 更新 session（颜色、名称）
updateSession(id: string, { title, color }): Promise<Session>
```

## 7. 参考

- Vercel AI SDK 设计模式
- 现有 shadcn/ui 组件库
- Tailwind CSS v4
