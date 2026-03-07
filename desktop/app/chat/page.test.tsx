import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import ChatPage, {
  createPendingTurn,
  reduceTurnEventForTurn,
} from "./page";

const startTurn = vi.fn();
const cancelTurn = vi.fn();
const subscribe = vi.fn();

let onTurnEvent: ((event: {
  turnId: string;
  type: string;
  data: Record<string, unknown>;
}) => void) | null = null;

vi.mock("@/lib/chat", () => ({
  useTurn: () => ({
    cancelTurn,
    startTurn,
    subscribe,
  }),
}));

describe("ChatPage", () => {
  beforeEach(() => {
    vi.stubGlobal(
      "ResizeObserver",
      class ResizeObserver {
        observe() {}
        disconnect() {}
      }
    );
    startTurn.mockReset();
    cancelTurn.mockReset();
    subscribe.mockReset();
    onTurnEvent = null;

    startTurn.mockResolvedValue({ turnId: "turn-1" });
    subscribe.mockImplementation(async (callback) => {
      onTurnEvent = callback;
      return () => {
        onTurnEvent = null;
      };
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("keeps multi-turn history and separates turns with numbered checkpoints", async () => {
    const user = userEvent.setup();
    const { container } = render(<ChatPage />);

    startTurn
      .mockResolvedValueOnce({ turnId: "turn-1" })
      .mockResolvedValueOnce({ turnId: "turn-2" });

    expect(screen.getByRole("textbox", { name: /prompt/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Agents" })).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Workflows" })
    ).not.toBeInTheDocument();
    expect(screen.queryByText("对话模块已移除")).not.toBeInTheDocument();

    await user.type(
      screen.getByRole("textbox", { name: /prompt/i }),
      "Review this plan"
    );
    await user.click(screen.getByRole("button", { name: "Send" }));

    expect(startTurn).toHaveBeenCalledWith({
      prompt: "Review this plan",
      targetId: "reviewer",
      targetKind: "agent",
    });

    await act(async () => {
      onTurnEvent?.({
        turnId: "turn-1",
        type: "llm-text-delta",
        data: { text: "Assistant answer" },
      });
      onTurnEvent?.({
        turnId: "turn-1",
        type: "llm-reasoning-delta",
        data: { text: "Reasoning trace" },
      });
      onTurnEvent?.({
        turnId: "turn-1",
        type: "tool-call-prepared",
        data: {
          argumentsJson: '{"path":"."}',
          callId: "call-1",
          name: "glob",
        },
      });
      onTurnEvent?.({
        turnId: "turn-1",
        type: "tool-call-completed",
        data: {
          callId: "call-1",
          result: {
            output: {
              matches: ["Cargo.toml"],
            },
            status: "success",
          },
        },
      });
      onTurnEvent?.({
        turnId: "turn-1",
        type: "turn-finished",
        data: { reason: "completed" },
      });
    });

    await user.click(screen.getByRole("button", { name: /Reasoning/i }));
    expect(screen.getByText("Reasoning trace")).toBeInTheDocument();
    expect(screen.getByText("glob")).toBeInTheDocument();

    await user.type(
      screen.getByRole("textbox", { name: /prompt/i }),
      "Plan the rollout"
    );
    await user.click(screen.getByRole("button", { name: "Send" }));

    await act(async () => {
      onTurnEvent?.({
        turnId: "turn-2",
        type: "llm-text-delta",
        data: { text: "Second turn answer" },
      });
      onTurnEvent?.({
        turnId: "turn-2",
        type: "turn-finished",
        data: { reason: "completed" },
      });
    });

    expect(screen.getByText("第 1 轮")).toBeInTheDocument();
    expect(screen.getByText("第 2 轮")).toBeInTheDocument();
    expect(screen.getByText("Review this plan")).toBeInTheDocument();
    expect(screen.getByText("Plan the rollout")).toBeInTheDocument();
    expect(screen.getByText("Assistant answer")).toBeInTheDocument();
    expect(screen.getByText("Second turn answer")).toBeInTheDocument();
    expect(
      container.querySelectorAll('[data-slot="chat-turn"]')
    ).toHaveLength(2);
  });

  it("renders a floating composer shell, a right-aligned user bubble, and plain assistant output", async () => {
    const user = userEvent.setup();
    const { container } = render(<ChatPage />);

    await user.type(
      screen.getByRole("textbox", { name: /prompt/i }),
      "Review this plan"
    );
    await user.click(screen.getByRole("button", { name: "Send" }));

    await act(async () => {
      onTurnEvent?.({
        turnId: "turn-1",
        type: "llm-text-delta",
        data: { text: "Assistant answer" },
      });
    });

    const composerShell = container.querySelector(
      '[data-slot="chat-composer-shell"]'
    );
    const scrollViewport = container.querySelector(
      '[data-slot="chat-scroll-viewport"]'
    );
    const scrollContent = container.querySelector(
      '[data-slot="chat-scroll-content"]'
    );
    const userBubble = screen
      .getByText("Review this plan")
      .closest('[data-slot="chat-turn-user"]');
    const assistantBody = screen
      .getByText("Assistant answer")
      .closest('[data-slot="chat-turn-assistant"]');

    expect(composerShell).toHaveClass("absolute");
    expect(composerShell).toHaveClass("bottom-4");
    expect(composerShell).toHaveClass("left-1/2");
    expect(composerShell).toHaveClass("w-full");
    expect(composerShell).toHaveClass("max-w-3xl");
    expect(composerShell).toHaveClass("-translate-x-1/2");
    expect(composerShell).not.toHaveClass("inset-x-0");
    expect(scrollViewport).toHaveClass("scrollbar-hide");
    expect(scrollContent).toHaveStyle({ paddingBottom: "220px" });
    expect(userBubble).toHaveClass("ml-auto");
    expect(userBubble).toHaveClass("bg-muted");
    expect(assistantBody).not.toHaveClass("border");
    expect(assistantBody).not.toHaveClass("bg-card");
    expect(container.querySelector(".ai-streamdown")).not.toBeInTheDocument();
  });

  it("marks the pending turn as failed when cancelling the previous running turn rejects", async () => {
    const user = userEvent.setup();

    render(<ChatPage />);

    await user.type(
      screen.getByRole("textbox", { name: /prompt/i }),
      "Review this plan"
    );
    await user.click(screen.getByRole("button", { name: "Send" }));

    await act(async () => {
      onTurnEvent?.({
        turnId: "turn-1",
        type: "llm-text-delta",
        data: { text: "Assistant answer" },
      });
    });

    cancelTurn.mockRejectedValueOnce(new Error("cancel failed"));

    await user.type(
      screen.getByRole("textbox", { name: /prompt/i }),
      "Plan the rollout"
    );
    await user.click(screen.getByRole("button", { name: "Send" }));

    expect(await screen.findByText("cancel failed")).toBeInTheDocument();
    expect(screen.getAllByText("Plan the rollout")).toHaveLength(2);
    expect(startTurn).toHaveBeenCalledTimes(1);
  });

  it("keeps only the latest plan snapshot for a turn", () => {
    const current = createPendingTurn("client-turn-1", "Review this plan");

    const first = reduceTurnEventForTurn(current, {
      data: {
        description: "Starting execution",
        isStreaming: true,
        sourceCallId: "call-1",
        tasks: [
          { id: "task-1", status: "completed", title: "Write failing test" },
        ],
        title: "Execution Plan",
      },
      turnId: "turn-1",
      type: "plan-updated",
    });
    const second = reduceTurnEventForTurn(first, {
      data: {
        isStreaming: true,
        sourceCallId: "call-2",
        tasks: [
          { id: "task-2", status: "in_progress", title: "Implement minimal fix" },
        ],
        title: "Execution Plan",
      },
      turnId: "turn-1",
      type: "plan-updated",
    });

    expect(first.latestPlan?.sourceCallId).toBe("call-1");
    expect(first.latestPlan?.tasks[0].title).toBe("Write failing test");
    expect(second.latestPlan?.sourceCallId).toBe("call-2");
    expect(second.latestPlan?.tasks).toEqual([
      {
        id: "task-2",
        status: "in_progress",
        title: "Implement minimal fix",
      },
    ]);
  });

  it("renders the queue before assistant markdown and hides update_plan tool rows", async () => {
    const user = userEvent.setup();
    render(<ChatPage />);

    await user.type(
      screen.getByRole("textbox", { name: /prompt/i }),
      "Review this plan"
    );
    await user.click(screen.getByRole("button", { name: "Send" }));

    await act(async () => {
      onTurnEvent?.({
        turnId: "turn-1",
        type: "tool-call-prepared",
        data: {
          argumentsJson: '{"plan":[{"step":"Write failing test","status":"completed"}]}',
          callId: "call-update-plan",
          name: "update_plan",
        },
      });
      onTurnEvent?.({
        turnId: "turn-1",
        type: "tool-call-completed",
        data: {
          callId: "call-update-plan",
          result: {
            output: {
              plan: {
                description: "Starting execution",
                isStreaming: false,
                tasks: [
                  { id: "task-1", status: "completed", title: "Write failing test" },
                ],
                title: "Execution Plan",
              },
            },
            status: "success",
          },
        },
      });
      onTurnEvent?.({
        turnId: "turn-1",
        type: "plan-updated",
        data: {
          description: "Starting execution",
          isStreaming: false,
          sourceCallId: "call-update-plan",
          tasks: [
            { id: "task-1", status: "completed", title: "Write failing test" },
          ],
          title: "Execution Plan",
        },
      });
      onTurnEvent?.({
        turnId: "turn-1",
        type: "llm-text-delta",
        data: { text: "Assistant answer" },
      });
    });

    const assistantSection = screen
      .getByText("Assistant answer")
      .closest('[data-slot="chat-turn-assistant"]') as HTMLElement | null;
    const planQueue = within(assistantSection!).getByText("Execution Plan").closest(
      '[data-slot="plan-queue"]'
    ) as HTMLElement | null;
    const assistantText = within(assistantSection!).getByText("Assistant answer");

    expect(planQueue).toBeInTheDocument();
    expect(within(assistantSection!).getByText("Write failing test")).toBeInTheDocument();
    expect(within(assistantSection!).queryByText("update_plan")).not.toBeInTheDocument();
    expect(planQueue?.compareDocumentPosition(assistantText)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING
    );
  });
});
