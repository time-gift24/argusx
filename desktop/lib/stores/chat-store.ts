import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import * as chatApi from "@/lib/api/chat";

// 类型定义
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
  // 新增 async actions
  loadSessions: () => Promise<void>;
  createSession: (title?: string) => Promise<void>;
  switchSession: (id: string) => Promise<void>;
  deleteSession: (id: string) => Promise<void>;
  sendMessage: (content: string) => Promise<void>;
  stopAgent: () => Promise<void>;
}

// API 转换函数
function toSession(apiSession: chatApi.ChatSession): Session {
  return {
    id: apiSession.id,
    title: apiSession.title,
    color: apiSession.color,
    createdAt: apiSession.created_at,
    updatedAt: apiSession.updated_at,
    status: apiSession.status,
  };
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

export const useChatStore = create<ChatStore>((set, get) => ({
  ...initialState,

  setSessions: (sessions) => set({ sessions }),
  setCurrentSession: (id) => set({ currentSessionId: id, messages: [] }),
  addMessage: (message) =>
    set((state) => ({ messages: [...state.messages, message] })),
  updateAssistantMessage: (content) =>
    set((state) => {
      if (state.messages.length === 0) return state;

      const msgs = [...state.messages];
      const lastMsg = msgs[msgs.length - 1];

      // Only update if last message is from assistant
      if (lastMsg && lastMsg.role === "assistant") {
        msgs[msgs.length - 1] = { ...lastMsg, content };
      } else {
        // If last message is not assistant, add new assistant message
        msgs.push({
          id: `msg-${Date.now()}-assistant`,
          role: "assistant",
          content,
          timestamp: Date.now(),
        });
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
  setError: (error: string | null) => set({ error }),
  reset: () => set(initialState),

  // 新增: 加载 Sessions
  loadSessions: async () => {
    try {
      const apiSessions = await chatApi.listChatSessions();
      const sessions = apiSessions.map(toSession);
      set({ sessions, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  // 新增: 创建 Session
  createSession: async (title) => {
    try {
      const apiSession = await chatApi.createChatSession(title);
      const session = toSession(apiSession);
      set((state) => ({
        sessions: [...state.sessions, session],
        currentSessionId: session.id,
        messages: [],
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  // 新增: 切换 Session
  switchSession: async (id) => {
    set({ currentSessionId: id, messages: [] });
    // TODO: 后续加载历史消息
    // try {
    //   const messages = await chatApi.getChatMessages(id);
    //   set({ messages });
    // } catch (e) {
    //   set({ error: String(e) });
    // }
  },

  // 新增: 删除 Session
  deleteSession: async (id) => {
    try {
      await chatApi.deleteChatSession(id);
      set((state) => {
        const sessions = state.sessions.filter((s) => s.id !== id);
        const currentSessionId = state.currentSessionId === id
          ? (sessions[0]?.id || null)
          : state.currentSessionId;
        return { sessions, currentSessionId, error: null };
      });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  // 新增: 发送消息
  sendMessage: async (content) => {
    const { currentSessionId, addMessage, setAgentStatus } = get();
    if (!currentSessionId) {
      set({ error: "No active session" });
      return;
    }

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

    // 调用 Tauri 命令触发流式响应
    try {
      await invoke("chat_stream", { sessionId: currentSessionId, message: content });
    } catch (e) {
      setAgentStatus("error");
      set({ error: String(e) });
    }
  },

  // 新增: 停止 Agent
  stopAgent: async () => {
    set({ agentStatus: "idle", reasoningText: "", toolProgress: [] });
  },
}));
