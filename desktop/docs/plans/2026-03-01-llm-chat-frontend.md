# LLM Chat 前端实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 实现悬浮式无边框 LLM 聊天页面，包含会话 Badge 列表和 PromptInput，使用 mock 数据先完成 UI

**Architecture:** 使用现有 ai-elements 组件（Conversation, PromptInput, Message），新增 SessionBadge 列表组件，采用 Tauri IPC 调用后端（先用 mock），状态管理用 zustand

**Tech Stack:** Next.js 16, React 19, Tailwind CSS v4, shadcn/ui, zustand, Tauri v2

---

## Task 1: 创建路由页面

**Files:**
- Create: `app/chat/page.tsx`

**Step 1: 创建 chat 路由目录**

Run: `mkdir -p app/chat`
Expected: Directory created

**Step 2: 创建基础页面文件**

```tsx
"use client";

import { ChatPage } from "@/components/features/chat/chat-page";

export default function Chat() {
  return <ChatPage />;
}
```

**Step 3: 验证路由可访问**

Run: `pnpm dev`
Visit: http://localhost:3000/chat
Expected: 页面加载（显示空白或错误，因为组件未创建）

---

## Task 2: 创建 ChatPage 主组件

**Files:**
- Create: `components/features/chat/chat-page.tsx`
- Create: `lib/stores/chat-store.ts`

**Step 1: 创建 chat store**

```typescript
// lib/stores/chat-store.ts
import { create } from "zustand";
import { persist } from "zustand/middleware";

export type ChatStatus = "wait-input" | "thinking" | "tool-call" | "outputing";

export interface ChatSession {
  id: string;
  title: string;
  color: string;
  status: ChatStatus;
  createdAt: number;
  updatedAt: number;
}

export interface ChatMessage {
  id: string;
  sessionId: string;
  role: "user" | "assistant" | "system";
  content: string;
  createdAt: number;
}

interface ChatState {
  sessions: ChatSession[];
  currentSessionId: string | null;
  messages: Record<string, ChatMessage[]>;

  // Actions
  createSession: () => string;
  deleteSession: (id: string) => void;
  updateSession: (id: string, updates: Partial<Pick<ChatSession, "title" | "color">>) => void;
  setCurrentSession: (id: string) => void;
  addMessage: (sessionId: string, message: Omit<ChatMessage, "id" | "sessionId" | "createdAt">) => void;
  updateSessionStatus: (id: string, status: ChatStatus) => void;
}

const COLORS = ["chart-1", "chart-2", "chart-3", "chart-4", "chart-5"];

export const useChatStore = create<ChatState>()(
  persist(
    (set, get) => ({
      sessions: [],
      currentSessionId: null,
      messages: {},

      createSession: () => {
        const id = `session-${Date.now()}`;
        const now = Date.now();
        const colorIndex = get().sessions.length % COLORS.length;

        const newSession: ChatSession = {
          id,
          title: `Chat ${get().sessions.length + 1}`,
          color: COLORS[colorIndex],
          status: "wait-input",
          createdAt: now,
          updatedAt: now,
        };

        set((state) => ({
          sessions: [...state.sessions, newSession],
          currentSessionId: id,
          messages: { ...state.messages, [id]: [] },
        }));

        return id;
      },

      deleteSession: (id) => {
        set((state) => {
          const sessions = state.sessions.filter((s) => s.id !== id);
          const messages = { ...state.messages };
          delete messages[id];

          let currentSessionId = state.currentSessionId;
          if (currentSessionId === id) {
            currentSessionId = sessions[0]?.id ?? null;
          }

          return { sessions, messages, currentSessionId };
        });
      },

      updateSession: (id, updates) => {
        set((state) => ({
          sessions: state.sessions.map((s) =>
            s.id === id ? { ...s, ...updates, updatedAt: Date.now() } : s
          ),
        }));
      },

      setCurrentSession: (id) => {
        set({ currentSessionId: id });
      },

      addMessage: (sessionId, message) => {
        const id = `msg-${Date.now()}`;
        const newMessage: ChatMessage = {
          ...message,
          id,
          sessionId,
          createdAt: Date.now(),
        };

        set((state) => ({
          messages: {
            ...state.messages,
            [sessionId]: [...(state.messages[sessionId] ?? []), newMessage],
          },
        }));
      },

      updateSessionStatus: (id, status) => {
        set((state) => ({
          sessions: state.sessions.map((s) =>
            s.id === id ? { ...s, status, updatedAt: Date.now() } : s
          ),
        }));
      },
    }),
    {
      name: "chat-storage",
    }
  )
);
```

**Step 2: 创建 ChatPage 组件**

```tsx
// components/features/chat/chat-page.tsx
"use client";

import { useEffect } from "react";
import { cn } from "@/lib/utils";
import { useChatStore } from "@/lib/stores/chat-store";
import { ConversationView } from "./conversation-view";
import { SessionBadgeList } from "./session-badge-list";
import { ChatPromptInput } from "./chat-prompt-input";

export function ChatPage() {
  const { sessions, currentSessionId, createSession } = useChatStore();

  // 如果没有会话，自动创建一个
  useEffect(() => {
    if (sessions.length === 0) {
      createSession();
    }
  }, [sessions.length, createSession]);

  const currentSession = sessions.find((s) => s.id === currentSessionId);

  return (
    <div className="relative flex h-screen flex-col">
      {/* 主内容区域 - 消息列表 */}
      <div className="flex-1 overflow-hidden pb-40">
        {currentSession ? (
          <ConversationView sessionId={currentSession.id} />
        ) : (
          <div className="flex h-full items-center justify-center text-muted-foreground">
            Select or create a chat session
          </div>
        )}
      </div>

      {/* 悬浮底部区域 */}
      <div
        className={cn(
          "fixed bottom-0 left-0 right-0 z-50",
          "bg-background/80 backdrop-blur-xl",
          "border-t border-border/50"
        )}
      >
        {/* Badge 列表 */}
        <SessionBadgeList />

        {/* 输入框 */}
        <div className="mx-auto max-w-3xl p-4">
          <ChatPromptInput />
        </div>
      </div>
    </div>
  );
}
```

**Step 3: 验证页面加载**

Run: `pnpm dev`
Visit: http://localhost:3000/chat
Expected: 页面显示，但提示组件未创建

---

## Task 3: 创建 ConversationView 组件

**Files:**
- Create: `components/features/chat/conversation-view.tsx`

**Step 1: 实现消息列表视图**

```tsx
// components/features/chat/conversation-view.tsx
"use client";

import { useChatStore } from "@/lib/stores/chat-store";
import {
  Conversation,
  ConversationContent,
  ConversationEmptyState,
} from "@/components/ai-elements/conversation";
import { Message, MessageResponse } from "@/components/ai-elements/message";
import { BotIcon } from "lucide-react";

interface ConversationViewProps {
  sessionId: string;
}

export function ConversationView({ sessionId }: ConversationViewProps) {
  const messages = useChatStore((state) => state.messages[sessionId] ?? []);

  return (
    <Conversation className="h-full">
      <ConversationContent className="mx-auto max-w-3xl px-4">
        {messages.length === 0 ? (
          <ConversationEmptyState
            description="Send a message to start the conversation"
            icon={<BotIcon className="size-12" />}
            title="No messages yet"
          />
        ) : (
          messages.map((message) => (
            <Message key={message.id}>
              {message.role === "assistant" ? (
                <MessageResponse>{message.content}</MessageResponse>
              ) : (
                <div className="whitespace-pre-wrap">{message.content}</div>
              )}
            </Message>
          ))
        )}
      </ConversationContent>
    </Conversation>
  );
}
```

**Step 2: 验证消息列表组件**

Run: `pnpm dev`
Expected: 无 TypeScript 错误

---

## Task 4: 创建 SessionBadge 组件

**Files:**
- Create: `components/features/chat/session-badge.tsx`

**Step 1: 实现单个 Badge 组件**

```tsx
// components/features/chat/session-badge.tsx
"use client";

import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { ChatStatus, ChatSession } from "@/lib/stores/chat-store";
import {
  Loader2Icon,
  MessageSquareIcon,
  WrenchIcon,
  TypeIcon,
} from "lucide-react";

interface SessionBadgeProps {
  session: ChatSession;
  isActive: boolean;
  onClick: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
}

const statusConfig: Record<
  ChatStatus,
  { icon: React.ReactNode; label: string }
> = {
  "wait-input": { icon: <MessageSquareIcon className="size-3" />, label: "Ready" },
  thinking: { icon: <Loader2Icon className="size-3 animate-spin" />, label: "Thinking" },
  "tool-call": { icon: <WrenchIcon className="size-3" />, label: "Tool" },
  outputing: { icon: <TypeIcon className="size-3" />, label: "Writing" },
};

export function SessionBadge({
  session,
  isActive,
  onClick,
  onContextMenu,
}: SessionBadgeProps) {
  const status = statusConfig[session.status];

  return (
    <Badge
      className={cn(
        "relative cursor-pointer px-3 py-1.5 transition-all",
        "hover:bg-accent/80",
        isActive && "bg-primary text-primary-foreground hover:bg-primary/90",
        `bg-${session.color}/20 text-${session.color}-foreground border-${session.color}/30`,
        isActive && `bg-primary`
      )}
      onClick={onClick}
      onContextMenu={onContextMenu}
      variant="outline"
    >
      {/* 状态图标 */}
      <span className="mr-1.5">{status.icon}</span>

      {/* 标题 */}
      <span className="max-w-24 truncate text-xs font-medium">
        {session.title}
      </span>
    </Badge>
  );
}
```

**Step 2: 验证 Badge 组件**

Run: `pnpm dev`
Expected: 无 TypeScript 错误

---

## Task 5: 创建 SessionBadgeList 组件

**Files:**
- Create: `components/features/chat/session-badge-list.tsx`
- Create: `components/features/chat/badge-context-menu.tsx`

**Step 1: 实现右键菜单组件**

```tsx
// components/features/chat/badge-context-menu.tsx
"use client";

import { useState } from "react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { ChatSession } from "@/lib/stores/chat-store";

interface BadgeContextMenuProps {
  session: ChatSession;
  children: React.ReactNode;
  onRename: (title: string) => void;
  onChangeColor: (color: string) => void;
  onDelete: () => void;
}

const COLORS = [
  { value: "chart-1", label: "Blue" },
  { value: "chart-2", label: "Cyan" },
  { value: "chart-3", label: "Teal" },
  { value: "chart-4", label: "Indigo" },
  { value: "chart-5", label: "Violet" },
];

export function BadgeContextMenu({
  session,
  children,
  onRename,
  onChangeColor,
  onDelete,
}: BadgeContextMenuProps) {
  const [showRenameDialog, setShowRenameDialog] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [newTitle, setNewTitle] = useState(session.title);

  const handleRename = () => {
    if (newTitle.trim()) {
      onRename(newTitle.trim());
      setShowRenameDialog(false);
    }
  };

  return (
    <>
      <ContextMenu>
        <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
        <ContextMenuContent className="w-48">
          <ContextMenuItem onClick={() => setShowRenameDialog(true)}>
            Rename
          </ContextMenuItem>
          <ContextMenuSeparator />
          {COLORS.map((color) => (
            <ContextMenuItem
              key={color.value}
              onClick={() => onChangeColor(color.value)}
            >
              <span className={`mr-2 size-3 rounded-full bg-${color.value}`} />
              {color.label}
            </ContextMenuItem>
          ))}
          <ContextMenuSeparator />
          <ContextMenuItem
            className="text-destructive"
            onClick={() => setShowDeleteDialog(true)}
          >
            Delete
          </ContextMenuItem>
        </ContextMenuContent>
      </ContextMenu>

      {/* Rename Dialog */}
      <Dialog open={showRenameDialog} onOpenChange={setShowRenameDialog}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Rename Session</DialogTitle>
          </DialogHeader>
          <div className="py-4">
            <Label htmlFor="title">Title</Label>
            <Input
              className="mt-2"
              id="title"
              onChange={(e) => setNewTitle(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleRename()}
              value={newTitle}
            />
          </div>
          <DialogFooter>
            <Button onClick={handleRename} size="sm">
              Save
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <AlertDialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Session?</AlertDialogTitle>
            <AlertDialogDescription>
              This will permanently delete "{session.title}" and all its
              messages. This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive text-destructive-foreground"
              onClick={onDelete}
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
```

**Step 2: 实现 Badge 列表组件**

```tsx
// components/features/chat/session-badge-list.tsx
"use client";

import { PlusIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useChatStore } from "@/lib/stores/chat-store";
import { SessionBadge } from "./session-badge";
import { BadgeContextMenu } from "./badge-context-menu";
import { cn } from "@/lib/utils";

export function SessionBadgeList() {
  const {
    sessions,
    currentSessionId,
    createSession,
    setCurrentSession,
    updateSession,
    deleteSession,
  } = useChatStore();

  return (
    <div className="flex items-center gap-2 overflow-x-auto px-4 py-2 scrollbar-hide">
      {sessions.map((session) => (
        <BadgeContextMenu
          key={session.id}
          onChangeColor={(color) => updateSession(session.id, { color })}
          onDelete={() => deleteSession(session.id)}
          onRename={(title) => updateSession(session.id, { title })}
          session={session}
        >
          <SessionBadge
            isActive={session.id === currentSessionId}
            onClick={() => setCurrentSession(session.id)}
            session={session}
          />
        </BadgeContextMenu>
      ))}

      {/* 新建会话按钮 */}
      <Button
        className={cn(
          "shrink-0 rounded-full",
          "border border-dashed border-muted-foreground/50",
          "hover:border-primary hover:bg-primary/10"
        )}
        onClick={() => createSession()}
        size="icon"
        variant="ghost"
      >
        <PlusIcon className="size-4" />
      </Button>
    </div>
  );
}
```

**Step 3: 验证 Badge 列表**

Run: `pnpm dev`
Visit: http://localhost:3000/chat
Expected: 看到 Badge 列表，可以点击创建新会话

---

## Task 6: 创建 ChatPromptInput 组件

**Files:**
- Create: `components/features/chat/chat-prompt-input.tsx`

**Step 1: 实现输入框组件**

```tsx
// components/features/chat/chat-prompt-input.tsx
"use client";

import { useState } from "react";
import { useChatStore } from "@/lib/stores/chat-store";
import {
  PromptInput,
  PromptInputTextarea,
  PromptInputSubmit,
  PromptInputTools,
  PromptInputActionMenu,
  PromptInputActionMenuTrigger,
  PromptInputActionMenuContent,
  PromptInputActionAddAttachments,
} from "@/components/ai-elements/prompt-input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";

const MOCK_MODELS = [
  { id: "glm-4", name: "GLM-4" },
  { id: "glm-5", name: "GLM-5" },
  { id: "gpt-4o", name: "GPT-4o" },
];

export function ChatPromptInput() {
  const { currentSessionId, addMessage, updateSessionStatus } = useChatStore();
  const [selectedModel, setSelectedModel] = useState("glm-4");
  const [status, setStatus] = useState<"idle" | "submitted">("idle");

  const handleSubmit = async (message: { text: string }) => {
    if (!currentSessionId || !message.text.trim()) return;

    // 添加用户消息
    addMessage(currentSessionId, {
      role: "user",
      content: message.text,
    });

    // 模拟 AI 响应
    setStatus("submitted");
    updateSessionStatus(currentSessionId, "thinking");

    // Mock: 模拟延迟后添加响应
    setTimeout(() => {
      addMessage(currentSessionId, {
        role: "assistant",
        content: `This is a mock response to: "${message.text}"\n\nIn a real implementation, this would be streamed from the LLM backend using Tauri IPC.`,
      });
      updateSessionStatus(currentSessionId, "wait-input");
      setStatus("idle");
    }, 1000);
  };

  return (
    <div className={cn("rounded-xl border border-border bg-card p-2 shadow-lg")}>
      <PromptInput onSubmit={handleSubmit}>
        <PromptInputTextarea
          className="min-h-[60px]"
          placeholder="Send a message..."
        />

        <div className="flex items-center justify-between gap-2 pt-2">
          <PromptInputTools>
            <PromptInputActionMenu>
              <PromptInputActionMenuTrigger />
              <PromptInputActionMenuContent>
                <PromptInputActionAddAttachments />
              </PromptInputActionMenuContent>
            </PromptInputActionMenu>

            {/* 模型选择 */}
            <Select value={selectedModel} onValueChange={setSelectedModel}>
              <SelectTrigger className="h-8 w-28 border-none bg-transparent text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {MOCK_MODELS.map((model) => (
                  <SelectItem key={model.id} value={model.id}>
                    {model.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </PromptInputTools>

          <PromptInputSubmit status={status === "submitted" ? "submitted" : "idle"} />
        </div>
      </PromptInput>
    </div>
  );
}
```

**Step 2: 验证完整页面功能**

Run: `pnpm dev`
Visit: http://localhost:3000/chat
Expected:
- 页面正常显示
- 可以创建新会话
- 可以发送消息
- 右键 Badge 显示菜单
- 删除会话有确认对话框

---

## Task 7: 添加 scrollbar-hide 工具类

**Files:**
- Modify: `app/globals.css`

**Step 1: 添加隐藏滚动条样式**

在 `@layer base` 后添加：

```css
@layer utilities {
  .scrollbar-hide {
    -ms-overflow-style: none;
    scrollbar-width: none;
  }
  .scrollbar-hide::-webkit-scrollbar {
    display: none;
  }
}
```

**Step 2: 验证样式生效**

Run: `pnpm dev`
Expected: Badge 列表横向滚动时无滚动条

---

## Task 8: 提交代码

**Step 1: 检查代码质量**

Run: `pnpm lint`
Expected: 无 ESLint 错误

**Step 2: 提交**

```bash
git add .
git commit -m "feat(chat): add llm chat page with session badges

- Create chat route page at /chat
- Implement floating layout with backdrop blur
- Add session badge list with context menu
- Integrate ai-elements (Conversation, PromptInput, Message)
- Add chat store with zustand for state management
- Support session CRUD operations
- Add mock data for initial development"
```

---

## 完成标准

- [ ] 访问 `/chat` 页面正常显示
- [ ] 悬浮布局正确，背景虚化效果
- [ ] Badge 列表横向滚动，无滚动条
- [ ] 点击 + 按钮创建新会话
- [ ] 左键点击 Badge 切换会话
- [ ] 右键 Badge 显示菜单（改名/改颜色/删除）
- [ ] 删除会话有二次确认
- [ ] 输入框有边框，支持发送消息
- [ ] Mock AI 响应正常显示
- [ ] 无 TypeScript 和 ESLint 错误
