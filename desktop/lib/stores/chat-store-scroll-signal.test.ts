import { beforeEach, describe, expect, it, vi } from "vitest";

import type { ChatSession } from "./chat-store";

const mocks = vi.hoisted(() => ({
  getChatTurnSummariesMock: vi.fn(),
}));

vi.mock("@/lib/api/chat", () => ({
  createChatSession: vi.fn(),
  deleteChatSession: vi.fn(),
  getChatMessages: vi.fn(),
  getChatTurnSummaries: mocks.getChatTurnSummariesMock,
  listChatSessions: vi.fn(),
  updateChatSession: vi.fn(),
}));

import { useChatStore } from "./chat-store";

const SESSION_ID = "session-1";

const createSession = (): ChatSession => ({
  id: SESSION_ID,
  title: "Session",
  color: "blue",
  status: "wait-input",
  createdAt: 1,
  updatedAt: 1,
});

describe("chat-store scroll signal", () => {
  beforeEach(() => {
    useChatStore.setState({
      sessions: [createSession()],
      currentSessionId: SESSION_ID,
      messages: { [SESSION_ID]: [] },
      turns: { [SESSION_ID]: [] },
      turnUiState: {},
      cacheBytes: 0,
    });
  });

  it("exposes scroll signal action and increments per session", () => {
    const state = useChatStore.getState() as unknown as {
      requestScrollToBottom?: (sessionId: string) => void;
      scrollToBottomSignal?: Record<string, number>;
    };

    expect(typeof state.requestScrollToBottom).toBe("function");

    state.requestScrollToBottom?.(SESSION_ID);
    state.requestScrollToBottom?.(SESSION_ID);

    const after = useChatStore.getState() as unknown as {
      scrollToBottomSignal?: Record<string, number>;
    };
    expect(after.scrollToBottomSignal?.[SESSION_ID]).toBe(2);
  });
});
