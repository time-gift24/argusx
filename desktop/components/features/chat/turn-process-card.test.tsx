import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { AgentTurnVM } from "@/lib/stores/chat-store";

import { TurnProcessCard } from "./turn-process-card";

type MockChatStoreState = {
  turnUiState: Record<string, Record<string, { processExpanded: boolean }>>;
  setTurnProcessExpanded: ReturnType<typeof vi.fn>;
};

const mocks = vi.hoisted(() => {
  const state: MockChatStoreState = {
    turnUiState: {},
    setTurnProcessExpanded: vi.fn(),
  };

  return {
    state,
  };
});

vi.mock("@/lib/stores/chat-store", () => ({
  useChatStore: (selector: (state: MockChatStoreState) => unknown) =>
    selector(mocks.state),
}));

vi.mock("./turn-process-view-model", () => ({
  buildTurnProcessVM: () => ({
    hasProcess: true,
    status: "thinking",
    statusLabel: "Thinking",
    summary: "Thinking",
    metrics: {
      toolCount: 0,
      queue: { waiting: 0, running: 0, completed: 0, failed: 0 },
      terminalLines: 0,
    },
    sections: [],
    terminalOutput: "",
  }),
}));

vi.mock("./turn-process-sections", () => ({
  TurnProcessSections: () => <div data-testid="turn-process-sections">sections</div>,
}));

const SESSION_ID = "session-1";
const TURN_ID = "turn-1";

const createTurn = (): AgentTurnVM => ({
  id: TURN_ID,
  sessionId: SESSION_ID,
  createdAt: 1,
  updatedAt: 1,
  status: "started",
  assistantText: "",
  reasoning: {
    isStreaming: false,
    isExpanded: false,
    preview: "",
    text: "",
    charCount: 0,
    truncated: false,
    updatedAt: 1,
    status: "idle",
  },
  tools: [],
  subAgents: [],
  queue: { items: [] },
  terminal: {
    stdout: "",
    stderr: "",
    output: "",
    isStreaming: false,
    updatedAt: 1,
  },
  lastSeq: 0,
});

describe("TurnProcessCard", () => {
  beforeEach(() => {
    mocks.state.turnUiState = {};
    mocks.state.setTurnProcessExpanded.mockReset();
  });

  it("defaults process to expanded when no frontend state exists", () => {
    render(<TurnProcessCard sessionId={SESSION_ID} turn={createTurn()} />);

    expect(screen.getByTestId("turn-process-sections")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Process/ })).toBeInTheDocument();
  });

  it("persists expand toggle to chat store", () => {
    render(<TurnProcessCard sessionId={SESSION_ID} turn={createTurn()} />);

    fireEvent.click(screen.getByRole("button", { name: /Process/ }));

    expect(mocks.state.setTurnProcessExpanded).toHaveBeenCalledWith(
      SESSION_ID,
      TURN_ID,
      false
    );
  });
});
