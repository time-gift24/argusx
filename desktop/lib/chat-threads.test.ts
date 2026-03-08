import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import {
  createConversationThread,
  listConversationThreads,
  switchConversationThread,
} from "@/lib/chat";

describe("chat thread ipc helpers", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("invokes create, list, and switch thread commands with serialized payloads", async () => {
    invokeMock
      .mockResolvedValueOnce({
        conversationId: "conversation-2",
        isActive: true,
        status: "idle",
        targetId: "execute",
        targetKind: "workflow",
        title: "Planning",
        updatedAtMs: 10,
      })
      .mockResolvedValueOnce([
        {
          conversationId: "conversation-2",
          isActive: true,
          status: "idle",
          targetId: "execute",
          targetKind: "workflow",
          title: "Planning",
          updatedAtMs: 10,
        },
      ])
      .mockResolvedValueOnce({
        conversationId: "conversation-1",
        isActive: true,
        status: "idle",
        targetId: "reviewer",
        targetKind: "agent",
        title: "hello",
        updatedAtMs: 9,
      });

    await createConversationThread({
      targetId: "execute",
      targetKind: "workflow",
      title: "Planning",
    });
    await listConversationThreads();
    await switchConversationThread({
      conversationId: "conversation-1",
    });

    expect(invokeMock).toHaveBeenNthCalledWith(1, "create_conversation_thread", {
      input: {
        targetId: "execute",
        targetKind: "workflow",
        title: "Planning",
      },
    });
    expect(invokeMock).toHaveBeenNthCalledWith(2, "list_conversation_threads");
    expect(invokeMock).toHaveBeenNthCalledWith(3, "switch_conversation_thread", {
      input: {
        conversationId: "conversation-1",
      },
    });
  });
});
