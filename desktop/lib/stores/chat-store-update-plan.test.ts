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

  it("hydrates plan and todoQueue from update_plan output with queue.todos", () => {
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
        arguments: { plan: [{ step: "Plan step", status: "completed" }] },
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
              tasks: [{ id: "task-1", title: "Plan step", status: "completed" }],
              is_streaming: false,
              queue: {
                todos: [
                  { id: "todo-1", title: "TODO Item 1", status: "pending" },
                  { id: "todo-2", title: "TODO Item 2", status: "in_progress" },
                ],
              },
            },
          },
        },
      },
    });

    const turn = useChatStore.getState().turns.s1?.[0];
    expect(turn?.plan?.tasks[0].title).toBe("Plan step");
    expect(turn?.todoQueue?.todos).toHaveLength(2);
    expect(turn?.todoQueue?.todos[0].title).toBe("TODO Item 1");
    expect(turn?.todoQueue?.todos[1].status).toBe("in_progress");
  });

  it("derives todoQueue from plan.tasks when queue.todos missing", () => {
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
        arguments: { plan: [{ step: "Task 1", status: "completed" }] },
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
              tasks: [{ id: "task-1", title: "Task 1", status: "completed" }],
              is_streaming: false,
            },
          },
        },
      },
    });

    const turn = useChatStore.getState().turns.s1?.[0];
    expect(turn?.todoQueue?.todos).toHaveLength(1);
    expect(turn?.todoQueue?.todos[0].title).toBe("Task 1");
    expect(turn?.todoQueue?.todos[0].status).toBe("completed");
  });

  it("normalizes unknown todo status to pending", () => {
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
        arguments: { plan: [{ step: "Task", status: "pending" }] },
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
              tasks: [{ id: "task-1", title: "Task", status: "pending" }],
              is_streaming: true,
              queue: {
                todos: [
                  { id: "todo-1", title: "TODO with unknown status", status: "unknown_status" },
                ],
              },
            },
          },
        },
      },
    });

    const turn = useChatStore.getState().turns.s1?.[0];
    expect(turn?.todoQueue?.todos[0].status).toBe("pending");
  });
});
