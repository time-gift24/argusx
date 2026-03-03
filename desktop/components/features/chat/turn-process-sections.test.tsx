import { fireEvent, render, screen } from "@testing-library/react";
import { act } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { AgentTurnVM } from "@/lib/stores/chat-store";

import type { TurnProcessVM } from "./turn-process-view-model";
import { TurnProcessSections } from "./turn-process-sections";

type MockChatStoreState = {
  turnUiState: Record<
    string,
    Record<
      string,
      {
        processExpanded: boolean;
        sectionExpanded: Partial<
          Record<"reasoning" | "plan" | "tools" | "terminal", boolean>
        >;
        codeExpanded: Record<string, boolean>;
      }
    >
  >;
  setTurnSectionExpanded: ReturnType<typeof vi.fn>;
  setTurnCodeExpanded: ReturnType<typeof vi.fn>;
};

const mocks = vi.hoisted(() => {
  const state: MockChatStoreState = {
    turnUiState: {},
    setTurnSectionExpanded: vi.fn(),
    setTurnCodeExpanded: vi.fn(),
  };

  return {
    state,
  };
});

vi.mock("@/lib/stores/chat-store", () => ({
  useChatStore: (selector: (state: MockChatStoreState) => unknown) =>
    selector(mocks.state),
}));

const SESSION_ID = "session-1";
const TURN_ID = "turn-1";

const createTurn = (
  overrides?: Partial<AgentTurnVM> & {
    reasoning?: Partial<AgentTurnVM["reasoning"]>;
  }
): AgentTurnVM => {
  const base: AgentTurnVM = {
    id: TURN_ID,
    sessionId: SESSION_ID,
    createdAt: 1,
    updatedAt: 1,
    status: "streaming",
    assistantText: "",
    reasoning: {
      isStreaming: true,
      isExpanded: false,
      preview: "Streaming reasoning...",
      text: "Thinking with `inline` code",
      charCount: 10,
      truncated: false,
      updatedAt: 1,
      status: "streaming",
    },
    tools: [],
    queue: { items: [] },
    terminal: {
      stdout: "",
      stderr: "",
      output: "",
      isStreaming: false,
      updatedAt: 1,
    },
    lastSeq: 1,
  };

  return {
    ...base,
    ...overrides,
    reasoning: {
      ...base.reasoning,
      ...overrides?.reasoning,
    },
  };
};

const createVm = (
  overrides?: Partial<TurnProcessVM> & {
    sections?: TurnProcessVM["sections"];
  }
): TurnProcessVM => {
  const base: TurnProcessVM = {
    hasProcess: true,
    status: "thinking",
    statusLabel: "Thinking",
    summary: "Thinking",
    metrics: {
      toolCount: 0,
      queue: {
        waiting: 0,
        running: 0,
        completed: 0,
        failed: 0,
      },
      terminalLines: 0,
    },
    sections: [
      {
        key: "reasoning",
        title: "Reasoning",
        preview: "Streaming reasoning...",
        isStreaming: true,
        defaultOpen: true,
      },
    ],
    terminalOutput: "",
  };

  return {
    ...base,
    ...overrides,
    sections: overrides?.sections ?? base.sections,
  };
};

describe("TurnProcessSections reasoning", () => {
  beforeEach(() => {
    vi.useRealTimers();
    mocks.state.turnUiState = {};
    mocks.state.setTurnSectionExpanded.mockReset();
    mocks.state.setTurnCodeExpanded.mockReset();
  });

  it("shows reasoning shimmer + live timer and auto closes around 1s after completion", () => {
    vi.useFakeTimers();
    const turn = createTurn({
      reasoning: {
        isStreaming: true,
        text: "streaming reasoning",
      },
    });
    const vm = createVm({
      sections: [
        {
          key: "reasoning",
          title: "Reasoning",
          preview: "Streaming reasoning...",
          isStreaming: true,
          defaultOpen: true,
        },
      ],
    });

    const { rerender } = render(
      <TurnProcessSections sessionId={SESSION_ID} turn={turn} vm={vm} />
    );

    expect(screen.getByText("Thinking...")).toBeInTheDocument();
    expect(screen.getByText("0s")).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(2000);
    });
    expect(screen.getByText("2s")).toBeInTheDocument();

    const reasoningRoot = screen
      .getByText("Thinking...")
      .closest('[data-slot="collapsible"]');
    expect(reasoningRoot?.className).not.toContain("border");

    expect(screen.getByText("streaming reasoning")).toBeVisible();

    const completedTurn = createTurn({
      status: "done",
      reasoning: {
        isStreaming: false,
        status: "completed",
        text: "final reasoning",
      },
    });
    const completedVm = createVm({
      status: "done",
      statusLabel: "Completed",
      summary: "Completed",
      sections: [
        {
          key: "reasoning",
          title: "Reasoning",
          preview: "Reasoning captured",
          isStreaming: false,
          defaultOpen: false,
        },
      ],
    });

    rerender(
      <TurnProcessSections
        sessionId={SESSION_ID}
        turn={completedTurn}
        vm={completedVm}
      />
    );

    expect(screen.getByText("Thought for 2s")).toBeInTheDocument();
    expect(screen.getByText("final reasoning")).toBeVisible();

    act(() => {
      vi.advanceTimersByTime(999);
    });
    expect(screen.getByText("final reasoning")).toBeVisible();

    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(screen.queryByText("final reasoning")).not.toBeInTheDocument();
  });

  it("shows tools shimmer + live timer and auto closes around 1s after completion", () => {
    vi.useFakeTimers();
    const turn = createTurn({
      queue: {
        items: [
          {
            callId: "call-1",
            status: "running",
            toolName: "web.search",
            updatedAt: 1,
          },
        ],
      },
      tools: [
        {
          callId: "call-1",
          input: { q: "docs" },
          state: "input-streaming",
          toolName: "web.search",
          updatedAt: 1,
        },
      ],
    });
    const vm = createVm({
      sections: [
        {
          defaultOpen: true,
          isStreaming: true,
          key: "tools",
          preview: "web.search · running",
          title: "Tools",
        },
      ],
    });

    const { rerender, container } = render(
      <TurnProcessSections sessionId={SESSION_ID} turn={turn} vm={vm} />
    );

    expect(screen.getByText("Running tools...")).toBeInTheDocument();
    expect(screen.getByText("0s")).toBeInTheDocument();
    expect(screen.queryByText("web.search · Running")).not.toBeInTheDocument();

    fireEvent.click(
      screen.getByText("Running tools...").closest('[data-slot="collapsible-trigger"]')!
    );
    const toolTitle = screen
      .getAllByText("web.search")
      .find((node) => node.closest('[data-slot="collapsible-trigger"]'));
    const toolTrigger = toolTitle?.closest('[data-slot="collapsible-trigger"]');
    if (!toolTrigger) {
      throw new Error("Expected tool trigger to exist");
    }
    fireEvent.click(toolTrigger);

    expect(container.querySelector(".llm-chat-code-surface")).toBeTruthy();
    expect(screen.getAllByLabelText("Copy code").length).toBeGreaterThan(0);
    expect(screen.getAllByLabelText("Download code").length).toBeGreaterThan(0);

    act(() => {
      vi.advanceTimersByTime(2000);
    });
    expect(screen.getByText("2s")).toBeInTheDocument();

    const toolsRoot = screen
      .getByText("Running tools...")
      .closest('[data-slot="collapsible"]');
    expect(toolsRoot?.className).not.toContain("border");

    expect(screen.getByText("web.search · Running")).toBeVisible();

    const completedTurn = createTurn({
      queue: {
        items: [
          {
            callId: "call-1",
            status: "completed",
            toolName: "web.search",
            updatedAt: 2,
          },
        ],
      },
      status: "done",
      tools: [
        {
          callId: "call-1",
          input: { q: "docs" },
          output: { count: 1 },
          state: "output-available",
          toolName: "web.search",
          updatedAt: 2,
        },
      ],
    });
    const completedVm = createVm({
      sections: [
        {
          defaultOpen: false,
          isStreaming: false,
          key: "tools",
          preview: "web.search · completed",
          title: "Tools",
        },
      ],
      status: "done",
      statusLabel: "Completed",
      summary: "Completed",
    });

    rerender(
      <TurnProcessSections
        sessionId={SESSION_ID}
        turn={completedTurn}
        vm={completedVm}
      />
    );

    expect(screen.getByText("Tools ran for 2s")).toBeInTheDocument();
    expect(screen.getByText("web.search · Completed")).toBeVisible();

    act(() => {
      vi.advanceTimersByTime(999);
    });
    expect(screen.getByText("web.search · Completed")).toBeVisible();

    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(screen.queryByText("web.search · Completed")).not.toBeInTheDocument();
  });

  it("renders terminal with aligned text header and no outer card", () => {
    vi.useFakeTimers();
    const turn = createTurn({
      terminal: {
        isStreaming: true,
        output: "line-1\nline-2",
        stderr: "",
        stdout: "line-1\nline-2",
        updatedAt: 1,
      },
    });
    const vm = createVm({
      sections: [
        {
          defaultOpen: true,
          isStreaming: true,
          key: "terminal",
          preview: "Streaming terminal output...",
          title: "Terminal",
        },
      ],
      terminalOutput: "line-1\nline-2",
    });

    const { rerender } = render(
      <TurnProcessSections sessionId={SESSION_ID} turn={turn} vm={vm} />
    );

    expect(screen.getByText("Running terminal...")).toBeInTheDocument();
    expect(screen.getByText("0s")).toBeInTheDocument();
    expect(screen.queryByText(/line-1/)).not.toBeInTheDocument();

    fireEvent.click(
      screen.getByText("Running terminal...").closest('[data-slot="collapsible-trigger"]')!
    );

    expect(screen.getByLabelText("Copy terminal output")).toBeInTheDocument();
    expect(screen.getByLabelText("Download terminal output")).toBeInTheDocument();
    expect(
      screen
        .getByLabelText("Copy terminal output")
        .closest(".llm-chat-terminal-surface")
    ).toBeTruthy();

    const terminalRoot = screen
      .getByText("Running terminal...")
      .closest('[data-slot="collapsible"]');
    expect(terminalRoot?.className).not.toContain("border");
    expect(screen.getByText(/line-1/)).toBeVisible();

    act(() => {
      vi.advanceTimersByTime(2000);
    });

    const completedTurn = createTurn({
      status: "done",
      terminal: {
        isStreaming: false,
        output: "line-1\nline-2",
        stderr: "",
        stdout: "line-1\nline-2",
        updatedAt: 2,
      },
    });
    const completedVm = createVm({
      sections: [
        {
          defaultOpen: false,
          isStreaming: false,
          key: "terminal",
          preview: "stdout: line-1",
          title: "Terminal",
        },
      ],
      status: "done",
      statusLabel: "Completed",
      summary: "Completed",
      terminalOutput: "line-1\nline-2",
    });

    rerender(
      <TurnProcessSections
        sessionId={SESSION_ID}
        turn={completedTurn}
        vm={completedVm}
      />
    );

    expect(screen.getByText("Terminal ran for 2s")).toBeInTheDocument();
    expect(screen.getByText(/line-1/)).toBeVisible();

    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(screen.queryByText(/line-1/)).not.toBeInTheDocument();
  });

  it("renders inline code inside llm-chat-markdown scope", () => {
    const turn = createTurn({
      queue: {
        items: [
          {
            callId: "call-1",
            status: "running",
            toolName: "web.search",
            updatedAt: 1,
          },
        ],
      },
      reasoning: {
        text: "This has `inline` code.",
      },
      tools: [
        {
          callId: "call-1",
          input: { q: "docs" },
          state: "input-streaming",
          toolName: "web.search",
          updatedAt: 1,
        },
      ],
      terminal: {
        isStreaming: true,
        output: "line-1\nline-2",
        stderr: "",
        stdout: "line-1\nline-2",
        updatedAt: 1,
      },
    });
    const vm = createVm({
      sections: [
        {
          key: "reasoning",
          title: "Reasoning",
          preview: "Streaming reasoning...",
          isStreaming: true,
          defaultOpen: true,
        },
        {
          key: "tools",
          title: "Tools",
          preview: "web.search · running",
          isStreaming: true,
          defaultOpen: true,
        },
        {
          key: "terminal",
          title: "Terminal",
          preview: "Streaming terminal output...",
          isStreaming: true,
          defaultOpen: true,
        },
      ],
      terminalOutput: "line-1\nline-2",
    });

    const { container } = render(
      <TurnProcessSections sessionId={SESSION_ID} turn={turn} vm={vm} />
    );

    const scoped = container.querySelector(".llm-chat-markdown");
    expect(scoped).toBeTruthy();
    expect(
      scoped?.querySelector('[data-streamdown="inline-code"]')
    ).toBeTruthy();

    expect(container.querySelector(".llm-chat-process-tools")).toBeTruthy();
    fireEvent.click(
      screen.getByText("Running terminal...").closest('[data-slot="collapsible-trigger"]')!
    );
    expect(container.querySelector(".llm-chat-terminal-surface")).toBeTruthy();
  });
});
