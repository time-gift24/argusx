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
  overrides?: Omit<Partial<AgentTurnVM>, "reasoning"> & {
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
        defaultOpen: false,
        headerLabel: "Thinking...",
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

describe("TurnProcessSections runtime contract", () => {
  beforeEach(() => {
    vi.useRealTimers();
    mocks.state.turnUiState = {};
    mocks.state.setTurnSectionExpanded.mockReset();
    mocks.state.setTurnCodeExpanded.mockReset();
  });

  it("keeps reasoning collapsed by default while streaming, with shimmer and timer in header", () => {
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
          defaultOpen: false,
          headerLabel: "Thinking...",
        },
      ],
    });

    render(<TurnProcessSections sessionId={SESSION_ID} turn={turn} vm={vm} />);

    expect(screen.getByText("Thinking...")).toBeInTheDocument();
    expect(screen.getByText("0s")).toBeInTheDocument();
    expect(screen.queryByText("streaming reasoning")).not.toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(2000);
    });

    expect(screen.getByText("2s")).toBeInTheDocument();

    fireEvent.click(
      screen
        .getByText("Thinking...")
        .closest('[data-slot="collapsible-trigger"]')!
    );

    expect(mocks.state.setTurnSectionExpanded).toHaveBeenCalledWith(
      SESSION_ID,
      TURN_ID,
      "reasoning",
      true
    );
  });

  it("renders tools header with concrete tool name detail and compact items", () => {
    const turn = createTurn({
      queue: {
        items: [
          {
            callId: "call-1",
            status: "running",
            toolName: "web.search",
            updatedAt: 2,
          },
          {
            callId: "call-2",
            status: "waiting",
            toolName: "functions.exec_command",
            updatedAt: 1,
          },
        ],
      },
      tools: [
        {
          callId: "call-1",
          input: { q: "docs" },
          output: { count: 1 },
          state: "input-streaming",
          toolName: "web.search",
          updatedAt: 2,
        },
        {
          callId: "call-2",
          input: { cmd: "ls -la" },
          state: "input-streaming",
          toolName: "functions.exec_command",
          updatedAt: 1,
        },
      ],
    });

    const vm = createVm({
      sections: [
        {
          defaultOpen: false,
          isStreaming: true,
          key: "tools",
          preview: "web.search · running",
          title: "Tools",
          headerLabel: "Running tools...",
          headerDetail: "web.search +1",
          compactItems: [
            { id: "call-1", label: "web.search", status: "Running" },
            {
              id: "call-2",
              label: "functions.exec_command",
              status: "Waiting",
            },
          ],
        },
      ],
    });

    render(<TurnProcessSections sessionId={SESSION_ID} turn={turn} vm={vm} />);

    expect(screen.getByText("Running tools...")).toBeInTheDocument();
    expect(screen.getByText("web.search +1")).toBeInTheDocument();
    expect(screen.queryByText("web.search · Running")).not.toBeInTheDocument();

    fireEvent.click(
      screen
        .getByText("Running tools...")
        .closest('[data-slot="collapsible-trigger"]')!
    );

    expect(mocks.state.setTurnSectionExpanded).toHaveBeenCalledWith(
      SESSION_ID,
      TURN_ID,
      "tools",
      true
    );
  });

  it("keeps terminal collapsed by default and renders terminal surface when expanded", () => {
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
          defaultOpen: false,
          isStreaming: true,
          key: "terminal",
          preview: "Streaming terminal output...",
          title: "Terminal",
          headerLabel: "Running terminal...",
        },
      ],
      terminalOutput: "line-1\nline-2",
    });

    const { container } = render(
      <TurnProcessSections sessionId={SESSION_ID} turn={turn} vm={vm} />
    );

    expect(screen.getByText("Running terminal...")).toBeInTheDocument();
    expect(screen.queryByText(/line-1/)).not.toBeInTheDocument();

    fireEvent.click(
      screen
        .getByText("Running terminal...")
        .closest('[data-slot="collapsible-trigger"]')!
    );

    expect(mocks.state.setTurnSectionExpanded).toHaveBeenCalledWith(
      SESSION_ID,
      TURN_ID,
      "terminal",
      true
    );
    expect(container.querySelector(".llm-chat-terminal-surface")).toBeFalsy();
  });

  it("reuses code surface class for terminal surface", () => {
    mocks.state.turnUiState = {
      [SESSION_ID]: {
        [TURN_ID]: {
          processExpanded: false,
          sectionExpanded: { terminal: true },
          codeExpanded: {},
        },
      },
    };

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
          defaultOpen: false,
          isStreaming: true,
          key: "terminal",
          preview: "Streaming terminal output...",
          title: "Terminal",
          headerLabel: "Running terminal...",
        },
      ],
      terminalOutput: "line-1\nline-2",
    });

    const { container } = render(
      <TurnProcessSections sessionId={SESSION_ID} turn={turn} vm={vm} />
    );

    expect(
      container.querySelector(".llm-chat-terminal-surface.llm-chat-code-surface")
    ).toBeTruthy();
    expect(screen.queryByLabelText("Download terminal output")).not.toBeInTheDocument();
    expect(screen.getByLabelText("Copy terminal output")).toBeInTheDocument();
  });

  it("keeps tool code blocks copy-only without download action", () => {
    mocks.state.turnUiState = {
      [SESSION_ID]: {
        [TURN_ID]: {
          processExpanded: false,
          sectionExpanded: { tools: true },
          codeExpanded: {},
        },
      },
    };

    const turn = createTurn({
      queue: {
        items: [
          {
            callId: "call-1",
            status: "running",
            toolName: "web.search",
            updatedAt: 2,
          },
        ],
      },
      tools: [
        {
          callId: "call-1",
          input: { q: "docs" },
          output: { count: 1 },
          state: "input-streaming",
          toolName: "web.search",
          updatedAt: 2,
        },
      ],
    });

    const vm = createVm({
      sections: [
        {
          defaultOpen: false,
          isStreaming: true,
          key: "tools",
          preview: "web.search · running",
          title: "Tools",
          headerLabel: "Running tools...",
          headerDetail: "web.search",
          compactItems: [{ id: "call-1", label: "web.search", status: "Running" }],
        },
      ],
    });

    render(<TurnProcessSections sessionId={SESSION_ID} turn={turn} vm={vm} />);

    const toolTrigger = document.querySelector(
      '.llm-chat-process-tools [data-slot="collapsible"][data-state="closed"] [data-slot="collapsible-trigger"]'
    );

    expect(toolTrigger).toBeTruthy();
    fireEvent.click(toolTrigger as HTMLElement);

    expect(screen.queryByLabelText("Download code")).not.toBeInTheDocument();
    expect(screen.getAllByLabelText("Copy code").length).toBeGreaterThan(0);
  });

  it("renders plan with dedicated plan-surface style class", () => {
    const turn = createTurn({
      plan: {
        title: "Execution Plan",
        description: "desc",
        tasks: [{ id: "t1", title: "Step 1", status: "pending" }],
        isStreaming: false,
      },
    });

    const vm = createVm({
      sections: [
        {
          key: "plan",
          title: "Plan",
          preview: "0/1 completed",
          isStreaming: false,
          defaultOpen: false,
          headerLabel: "Plan",
        },
      ],
    });

    const { container } = render(
      <TurnProcessSections sessionId={SESSION_ID} turn={turn} vm={vm} />
    );

    expect(container.querySelector(".plan-surface")).toBeTruthy();
  });

  it("keeps inline code rendering under llm-chat-markdown scope", () => {
    mocks.state.turnUiState = {
      [SESSION_ID]: {
        [TURN_ID]: {
          processExpanded: false,
          sectionExpanded: { reasoning: true },
          codeExpanded: {},
        },
      },
    };

    const turn = createTurn({
      reasoning: {
        text: "This has `inline` code.",
      },
    });
    const vm = createVm({
      sections: [
        {
          key: "reasoning",
          title: "Reasoning",
          preview: "Streaming reasoning...",
          isStreaming: true,
          defaultOpen: false,
          headerLabel: "Thinking...",
        },
      ],
    });

    const { container } = render(
      <TurnProcessSections sessionId={SESSION_ID} turn={turn} vm={vm} />
    );

    const scoped = container.querySelector(".llm-chat-markdown");
    expect(scoped).toBeTruthy();
    expect(scoped?.querySelector('[data-streamdown="inline-code"]')).toBeTruthy();
  });

  it("keeps inline code non-highlighted while preserving block code rendering", () => {
    mocks.state.turnUiState = {
      [SESSION_ID]: {
        [TURN_ID]: {
          processExpanded: false,
          sectionExpanded: { reasoning: true },
          codeExpanded: {},
        },
      },
    };

    const turn = createTurn({
      reasoning: {
        text: "This has `inline` code and:\n\n```ts\nconst x = 1;\n```",
      },
    });
    const vm = createVm({
      sections: [
        {
          key: "reasoning",
          title: "Reasoning",
          preview: "Streaming reasoning...",
          isStreaming: true,
          defaultOpen: false,
          headerLabel: "Thinking...",
        },
      ],
    });

    const { container } = render(
      <TurnProcessSections sessionId={SESSION_ID} turn={turn} vm={vm} />
    );

    // Inline code should not be highlighted
    expect(container.querySelector(".llm-chat-markdown :not(pre) > code")).toBeTruthy();
    // Fenced code block should be highlighted
    expect(container.querySelector('[data-highlighted="true"]')).toBeTruthy();
  });
});
