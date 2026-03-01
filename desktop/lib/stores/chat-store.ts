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
