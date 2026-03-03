import { beforeEach, describe, expect, it, vi } from "vitest";

import type { AgentTurnVM, ChatSession } from "./chat-store";

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
const TURN_ID = "turn-1";

const createSession = (): ChatSession => ({
  id: SESSION_ID,
  title: "Session",
  color: "blue",
  status: "wait-input",
  createdAt: 1,
  updatedAt: 1,
});

const createTurn = (overrides?: Partial<AgentTurnVM>): AgentTurnVM => ({
  id: TURN_ID,
  sessionId: SESSION_ID,
  requestMessageId: "req-1",
  createdAt: 1,
  updatedAt: 4,
  status: "done",
  assistantText: "local summary",
  reasoning: {
    isStreaming: false,
    isExpanded: true,
    preview: "local reasoning",
    text: "local reasoning",
    charCount: 14,
    truncated: false,
    updatedAt: 4,
    status: "completed",
  },
  tools: [
    {
      callId: "call-1",
      toolName: "functions.exec_command",
      state: "output-available",
      output: "ls -la",
      updatedAt: 4,
    },
  ],
  queue: {
    items: [
      {
        callId: "call-1",
        toolName: "functions.exec_command",
        status: "completed",
        updatedAt: 4,
      },
    ],
  },
  terminal: {
    stdout: "local stdout",
    stderr: "",
    output: "local stdout",
    isStreaming: false,
    exitCode: 0,
    durationMs: 123,
    updatedAt: 4,
  },
  plan: {
    title: "Execution Plan",
    tasks: [{ id: "task-1", title: "step", status: "completed" }],
    isStreaming: false,
  },
  planSource: "structured",
  lastSeq: 10,
  ...overrides,
});

describe("chat-store loadSessionTurns", () => {
  beforeEach(() => {
    mocks.getChatTurnSummariesMock.mockReset();
    useChatStore.setState({
      sessions: [createSession()],
      currentSessionId: SESSION_ID,
      messages: { [SESSION_ID]: [] },
      turns: { [SESSION_ID]: [] },
      turnUiState: {},
      cacheBytes: 0,
    });
  });

  it("keeps frontend process details when backend only returns turn summary", async () => {
    useChatStore.setState({
      turns: { [SESSION_ID]: [createTurn()] },
    });
    mocks.getChatTurnSummariesMock.mockResolvedValueOnce([
      {
        id: TURN_ID,
        session_id: SESSION_ID,
        status: "done",
        final_message: "backend summary",
        created_at: 1,
        updated_at: 5,
      },
    ]);

    await useChatStore.getState().loadSessionTurns(SESSION_ID);

    const merged = useChatStore.getState().turns[SESSION_ID]?.[0];
    expect(merged?.assistantText).toBe("backend summary");
    expect(merged?.reasoning.text).toBe("local reasoning");
    expect(merged?.tools).toHaveLength(1);
    expect(merged?.terminal.output).toBe("local stdout");
    expect(merged?.lastSeq).toBe(10);
  });

  it("keeps local streaming status when backend still reports running", async () => {
    useChatStore.setState({
      turns: {
        [SESSION_ID]: [
          createTurn({
            status: "streaming",
            reasoning: {
              isStreaming: true,
              isExpanded: true,
              preview: "streaming",
              text: "streaming",
              charCount: 9,
              truncated: false,
              updatedAt: 4,
              status: "streaming",
            },
          }),
        ],
      },
    });
    mocks.getChatTurnSummariesMock.mockResolvedValueOnce([
      {
        id: TURN_ID,
        session_id: SESSION_ID,
        status: "running",
        created_at: 1,
        updated_at: 5,
      },
    ]);

    await useChatStore.getState().loadSessionTurns(SESSION_ID);

    const merged = useChatStore.getState().turns[SESSION_ID]?.[0];
    expect(merged?.status).toBe("streaming");
    expect(merged?.reasoning.isStreaming).toBe(true);
  });
});
