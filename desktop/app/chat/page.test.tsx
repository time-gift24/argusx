import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

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

  it("starts an agent turn and renders streamed updates", async () => {
    const user = userEvent.setup();

    render(<ChatPage />);

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

    expect(screen.getByText("Assistant answer")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Reasoning/i }));
    expect(screen.getByText("Reasoning trace")).toBeInTheDocument();
    expect(screen.getByText("glob")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Cancel" })
    ).not.toBeInTheDocument();
  });
});
