import { beforeEach, describe, expect, it, vi } from "vitest";
import { useChatStore } from "./chat-store";

vi.mock("@/lib/api/chat", () => ({
  createChatSession: vi.fn(),
  deleteChatSession: vi.fn(),
  getChatMessages: vi.fn(),
  getChatTurnSummaries: vi.fn(),
  listChatSessions: vi.fn(),
  updateChatSession: vi.fn(),
}));

describe("chat-store update_plan tool result", () => {
  beforeEach(() => {
    useChatStore.setState({
      sessions: [{
        id: "s1",
        title: "Session",
        color: "blue",
        status: "thinking",
        createdAt: 1,
        updatedAt: 1,
      }],
      currentSessionId: "s1",
      messages: { s1: [] },
      turns: { s1: [] },
      turnUiState: {},
      scrollToBottomSignal: {},
      cacheBytes: 0,
    });
  });

  it("hydrates structured plan from update_plan tool output", () => {
    const store = useChatStore.getState();
    store.applyAgentStreamEnvelope({
      sessionId: "s1",
      turnId: "t1",
      source: "ui",
      seq: 1,
      ts: 1,
      event: {
        type: "tool_call_requested",
        call_id: "c1",
        tool_name: "update_plan",
        arguments: { plan: [{ step: "Write test", status: "in_progress" }] },
      },
    });
    store.applyAgentStreamEnvelope({
      sessionId: "s1",
      turnId: "t1",
      source: "ui",
      seq: 2,
      ts: 2,
      event: {
        type: "tool_call_completed",
        result: {
          call_id: "c1",
          is_error: false,
          output: {
            plan: {
              title: "Execution Plan",
              tasks: [{ id: "task-1", title: "Write test", status: "in_progress" }],
              is_streaming: true,
            },
          },
        },
      },
    });

    const turn = useChatStore.getState().turns.s1?.[0];
    expect(turn?.planSource).toBe("structured");
    expect(turn?.plan?.tasks[0].title).toBe("Write test");
    expect(turn?.plan?.isStreaming).toBe(true);
  });
});
