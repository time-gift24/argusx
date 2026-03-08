import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

const {
  cancelConversationMock,
  continueConversationMock,
  listConversationThreadsMock,
  restartConversationMock,
  startConversationMock,
  subscribeToTurnEventsMock,
  switchConversationThreadMock,
} = vi.hoisted(() => ({
  cancelConversationMock: vi.fn(),
  continueConversationMock: vi.fn(),
  listConversationThreadsMock: vi.fn(),
  restartConversationMock: vi.fn(),
  startConversationMock: vi.fn(),
  subscribeToTurnEventsMock: vi.fn(),
  switchConversationThreadMock: vi.fn(),
}));

vi.mock("@/lib/chat", () => ({
  CHAT_AGENTS: [
    {
      description: "Review a change set with an engineering lens",
      id: "reviewer",
      label: "Code Reviewer",
    },
    {
      description: "Break ambiguous work into concrete steps",
      id: "planner",
      label: "Planner",
    },
  ],
  CHAT_WORKFLOWS: [
    {
      description: "Draft a design-oriented implementation brief",
      id: "design",
      label: "Write Design",
    },
    {
      description: "Prepare the task for a focused implementation pass",
      id: "execute",
      label: "Execute Plan",
    },
  ],
  cancelConversation: cancelConversationMock,
  continueConversation: continueConversationMock,
  listConversationThreads: listConversationThreadsMock,
  restartConversation: restartConversationMock,
  startConversation: startConversationMock,
  subscribeToTurnEvents: subscribeToTurnEventsMock,
  switchConversationThread: switchConversationThreadMock,
}));

import ChatPage from "./page";

describe("ChatPage", () => {
  beforeEach(() => {
    startConversationMock.mockReset();
    continueConversationMock.mockReset();
    cancelConversationMock.mockReset();
    listConversationThreadsMock.mockReset();
    restartConversationMock.mockReset();
    subscribeToTurnEventsMock.mockReset();
    switchConversationThreadMock.mockReset();

    startConversationMock.mockResolvedValue({
      conversationId: "conversation-1",
      turnId: "turn-1",
    });
    continueConversationMock.mockResolvedValue({
      conversationId: "conversation-1",
      turnId: "turn-2",
    });
    cancelConversationMock.mockResolvedValue(undefined);
    listConversationThreadsMock.mockResolvedValue([]);
    restartConversationMock.mockResolvedValue({
      conversationId: "conversation-1",
      turnId: "turn-2",
    });
    switchConversationThreadMock.mockResolvedValue({
      conversationId: "conversation-1",
      isActive: true,
      status: "idle",
      targetId: "reviewer",
      targetKind: "agent",
      title: "Review thread",
      updatedAtMs: 1,
    });
  });

  it("starts a conversation and renders streamed text, reasoning, and tool output", async () => {
    const user = userEvent.setup();
    let eventHandler:
      | ((event: {
          conversationId: string;
          data: Record<string, unknown>;
          turnId: string;
          type: string;
        }) => void)
      | undefined;

    subscribeToTurnEventsMock.mockImplementation(async (handler) => {
      eventHandler = handler;
      return vi.fn();
    });

    render(<ChatPage />);

    await waitFor(() => expect(subscribeToTurnEventsMock).toHaveBeenCalledOnce());

    await user.type(
      screen.getByRole("textbox", { name: /prompt/i }),
      "Review this change"
    );
    await user.keyboard("{Enter}");

    expect(startConversationMock).toHaveBeenCalledWith({
      prompt: "Review this change",
      targetId: "reviewer",
      targetKind: "agent",
    });

    act(() => {
      eventHandler?.({
        conversationId: "conversation-1",
        data: { text: "Thinking..." },
        turnId: "turn-1",
        type: "llm-reasoning-delta",
      });
      eventHandler?.({
        conversationId: "conversation-1",
        data: {
          arguments: "{\"path\":\"README.md\"}",
          callId: "call-1",
          name: "read",
        },
        turnId: "turn-1",
        type: "tool-call-prepared",
      });
      eventHandler?.({
        conversationId: "conversation-1",
        data: {
          callId: "call-1",
          output: { bytes: 42, path: "README.md" },
          status: "success",
        },
        turnId: "turn-1",
        type: "tool-call-completed",
      });
      eventHandler?.({
        conversationId: "conversation-1",
        data: { text: "Looks good." },
        turnId: "turn-1",
        type: "llm-text-delta",
      });
    });

    expect(await screen.findByText("Review this change")).toBeInTheDocument();
    expect(screen.getByText("Thinking...")).toBeInTheDocument();
    expect(screen.getByText("Looks good.")).toBeInTheDocument();
    expect(screen.getAllByText(/README\.md/)).toHaveLength(2);
    expect(screen.getByText(/"bytes": 42/)).toBeInTheDocument();
  });

  it("continues an existing conversation and cancels the active turn", async () => {
    const user = userEvent.setup();
    let eventHandler:
      | ((event: {
          conversationId: string;
          data: Record<string, unknown>;
          turnId: string;
          type: string;
        }) => void)
      | undefined;

    subscribeToTurnEventsMock.mockImplementation(async (handler) => {
      eventHandler = handler;
      return vi.fn();
    });

    render(<ChatPage />);

    await waitFor(() => expect(subscribeToTurnEventsMock).toHaveBeenCalledOnce());
    await user.type(screen.getByRole("textbox", { name: /prompt/i }), "hello");
    await user.keyboard("{Enter}");

    act(() => {
      eventHandler?.({
        conversationId: "conversation-1",
        data: { reason: "completed" },
        turnId: "turn-1",
        type: "turn-finished",
      });
    });

    await user.type(screen.getByRole("textbox", { name: /prompt/i }), "continue");
    await user.keyboard("{Enter}");

    expect(continueConversationMock).toHaveBeenCalledWith({
      conversationId: "conversation-1",
      prompt: "continue",
    });

    expect(await screen.findByRole("button", { name: "Cancel Turn" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Cancel Turn" }));

    expect(cancelConversationMock).toHaveBeenCalledWith({
      conversationId: "conversation-1",
    });
  });

  it("renders the prompt composer instead of the retired placeholder", async () => {
    subscribeToTurnEventsMock.mockResolvedValue(vi.fn());

    render(<ChatPage />);
    await waitFor(() =>
      expect(listConversationThreadsMock).toHaveBeenCalledOnce()
    );

    expect(screen.getByRole("textbox", { name: /prompt/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Agents" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Workflows" })).toBeInTheDocument();
    expect(screen.queryByText("对话模块已移除")).not.toBeInTheDocument();
  });

  it("offers a restart action for restartable threads", async () => {
    const user = userEvent.setup();
    subscribeToTurnEventsMock.mockResolvedValue(vi.fn());
    listConversationThreadsMock.mockResolvedValue([
      {
        conversationId: "conversation-1",
        isActive: true,
        status: "restartable",
        targetId: "reviewer",
        targetKind: "agent",
        title: "Interrupted review",
        updatedAtMs: 10,
      },
    ]);

    render(<ChatPage />);

    await waitFor(() =>
      expect(listConversationThreadsMock).toHaveBeenCalledOnce()
    );
    expect(
      await screen.findByRole("button", { name: "Restart Turn" })
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Restart Turn" }));

    expect(restartConversationMock).toHaveBeenCalledWith({
      conversationId: "conversation-1",
    });
  });
});
