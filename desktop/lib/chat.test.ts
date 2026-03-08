import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

import {
  cancelConversation,
  continueConversation,
  startConversation,
  subscribeToTurnEvents,
} from "@/lib/chat";

describe("chat ipc helpers", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockReset();
  });

  it("invokes the start conversation command with the serialized payload", async () => {
    invokeMock.mockResolvedValueOnce({
      conversationId: "conversation-1",
      turnId: "turn-1",
    });

    await expect(
      startConversation({
        prompt: "hello",
        targetId: "reviewer",
        targetKind: "agent",
      })
    ).resolves.toEqual({
      conversationId: "conversation-1",
      turnId: "turn-1",
    });

    expect(invokeMock).toHaveBeenCalledWith("start_conversation", {
      input: {
        prompt: "hello",
        targetId: "reviewer",
        targetKind: "agent",
      },
    });
  });

  it("invokes continue and cancel commands with conversation ids", async () => {
    invokeMock.mockResolvedValueOnce({
      conversationId: "conversation-1",
      turnId: "turn-2",
    });
    invokeMock.mockResolvedValueOnce(undefined);

    await continueConversation({
      conversationId: "conversation-1",
      prompt: "continue",
    });
    await cancelConversation({
      conversationId: "conversation-1",
    });

    expect(invokeMock).toHaveBeenNthCalledWith(1, "continue_conversation", {
      input: {
        conversationId: "conversation-1",
        prompt: "continue",
      },
    });
    expect(invokeMock).toHaveBeenNthCalledWith(2, "cancel_conversation", {
      input: {
        conversationId: "conversation-1",
      },
    });
  });

  it("subscribes to desktop turn events and forwards payloads", async () => {
    const unlisten = vi.fn();
    const handler = vi.fn();

    listenMock.mockImplementationOnce(async (_eventName, listener) => {
      listener({
        payload: {
          conversationId: "conversation-1",
          data: { text: "hello" },
          turnId: "turn-1",
          type: "llm-text-delta",
        },
      });
      return unlisten;
    });

    const receivedUnlisten = await subscribeToTurnEvents(handler);

    expect(listenMock).toHaveBeenCalledWith("turn-event", expect.any(Function));
    expect(handler).toHaveBeenCalledWith({
      conversationId: "conversation-1",
      data: { text: "hello" },
      turnId: "turn-1",
      type: "llm-text-delta",
    });
    expect(receivedUnlisten).toBe(unlisten);
  });
});
