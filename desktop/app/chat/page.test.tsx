import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import ChatPage from "./page";

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
    expect(composerShell).toHaveClass("backdrop-blur-sm");
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
});
